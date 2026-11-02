// SPDX-License-Identifier: Apache-2.0

//! Per-module enumeration isolation.
//!
//! A VST3 module is arbitrary native code. Loading one in the scanner process
//! means a hanging module stalls the whole scan and a crashing module takes the
//! scanner with it, with no way to say which module was at fault. This module
//! runs each module's factory enumeration in a child process of the scanner and
//! attributes a timeout or a crash to that module, so one broken plug-in becomes
//! one rejected module in the report instead of a dead scan.
//!
//! The child is the scanner binary itself in worker mode
//! (`--enumerate-module PATH --output FILE`), so both sides share one
//! implementation of the VST3 loading. The parent never reads the module in
//! process unless isolation is unavailable, and it says so in a diagnostic when
//! it falls back.

use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

/// Upper bound accepted for one module's enumeration.
pub const MAX_MODULE_TIMEOUT: Duration = Duration::from_secs(600);
/// Smallest timeout worth trying; below this a healthy module may be killed.
pub const MIN_MODULE_TIMEOUT: Duration = Duration::from_secs(1);
/// Default per-module budget. Enumeration only reads factory metadata, so a
/// module that needs longer than this is already misbehaving.
pub const DEFAULT_MODULE_TIMEOUT: Duration = Duration::from_secs(30);

/// One module's enumeration timeout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IsolationLimits {
    pub timeout: Duration,
}

impl Default for IsolationLimits {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_MODULE_TIMEOUT,
        }
    }
}

impl IsolationLimits {
    /// Clamps the timeout into a range where isolation means something.
    pub fn sanitized(self) -> Self {
        Self {
            timeout: self.timeout.clamp(MIN_MODULE_TIMEOUT, MAX_MODULE_TIMEOUT),
        }
    }
}

/// Serializable copy of `opentrack_vst3::Vst3ClassInfo`.
///
/// The worker writes this to a file and the parent reads it back, so the data
/// shape is an explicit protocol rather than a `repr` bet.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ClassPayload {
    pub class_id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub category: String,
    pub subcategories: String,
    pub cardinality: i32,
    pub class_flags: u32,
}

/// Serializable copy of `opentrack_vst3::ModuleEnumeration`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct EnumerationPayload {
    pub factory_vendor: String,
    pub factory_url: String,
    pub factory_email: String,
    pub classes: Vec<ClassPayload>,
    pub diagnostics: Vec<String>,
}

impl From<&opentrack_vst3::ModuleEnumeration> for EnumerationPayload {
    fn from(enumeration: &opentrack_vst3::ModuleEnumeration) -> Self {
        Self {
            factory_vendor: enumeration.factory_vendor.clone(),
            factory_url: enumeration.factory_url.clone(),
            factory_email: enumeration.factory_email.clone(),
            classes: enumeration
                .classes
                .iter()
                .map(|class| ClassPayload {
                    class_id: class.class_id.to_hex(),
                    name: class.name.clone(),
                    vendor: class.vendor.clone(),
                    version: class.version.clone(),
                    category: class.category.clone(),
                    subcategories: class.subcategories.clone(),
                    cardinality: class.cardinality,
                    class_flags: class.class_flags,
                })
                .collect(),
            diagnostics: enumeration.diagnostics.clone(),
        }
    }
}

impl EnumerationPayload {
    /// Rebuilds the host type so report code keeps one code path.
    pub fn into_module_enumeration(self) -> Result<opentrack_vst3::ModuleEnumeration, String> {
        let classes = self
            .classes
            .into_iter()
            .map(|class| {
                opentrack_plugins::PluginClassId::from_hex(&class.class_id)
                    .map(|class_id| opentrack_vst3::Vst3ClassInfo {
                        class_id,
                        name: class.name,
                        vendor: class.vendor,
                        version: class.version,
                        category: class.category,
                        subcategories: class.subcategories,
                        cardinality: class.cardinality,
                        class_flags: class.class_flags,
                    })
                    .map_err(|error| format!("worker reported class {}: {error}", class.class_id))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(opentrack_vst3::ModuleEnumeration {
            factory_vendor: self.factory_vendor,
            factory_url: self.factory_url,
            factory_email: self.factory_email,
            classes,
            diagnostics: self.diagnostics,
        })
    }
}

/// What the worker reports back.
///
/// A worker that could load the module writes `Enumerated`; a worker whose module
/// refused to load writes `Failed` and still exits successfully. Anything else —
/// an abnormal exit, a signal, a missing or unreadable file — is attributed to
/// the worker, which means the module crashed or hung the process. Without that
/// distinction a module that simply cannot load would be misreported as a crash.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum WorkerResult {
    Enumerated(Box<EnumerationPayload>),
    Failed(String),
}

/// How one module's enumeration ended.
#[derive(Clone, Debug, PartialEq)]
pub enum IsolationOutcome {
    /// The worker returned factory metadata.
    Enumerated(opentrack_vst3::ModuleEnumeration),
    /// The module reported an error while loading or enumerating.
    Failed(String),
    /// The worker exceeded its budget and was killed.
    TimedOut { limit: Duration },
    /// The worker died without a successful result.
    Crashed(String),
}

impl IsolationOutcome {
    /// Short text recorded as the module's diagnostic.
    pub fn diagnostic(&self) -> Option<String> {
        match self {
            Self::Enumerated(_) => None,
            Self::Failed(error) => Some(error.clone()),
            Self::TimedOut { limit } => Some(format!(
                "module did not finish enumerating within {} s and was terminated",
                limit.as_secs()
            )),
            Self::Crashed(reason) => Some(format!("module process {reason} while enumerating")),
        }
    }
}

/// Enumerates one module. Implementations differ in where the module runs.
pub trait ModuleEnumerator {
    fn enumerate(&self, path: &Path) -> IsolationOutcome;
    /// True when the module is loaded in this process.
    fn runs_in_process(&self) -> bool {
        true
    }
}

/// The historical path: load the module in the scanner process.
pub struct InProcessEnumerator;

impl ModuleEnumerator for InProcessEnumerator {
    fn enumerate(&self, path: &Path) -> IsolationOutcome {
        let loaded = match opentrack_vst3::Vst3Module::open(path) {
            Ok(loaded) => loaded,
            Err(error) => return IsolationOutcome::Failed(error.to_string()),
        };
        match loaded.enumerate() {
            Ok(enumeration) => IsolationOutcome::Enumerated(enumeration),
            Err(error) => IsolationOutcome::Failed(error.to_string()),
        }
    }
}

/// Runs each module's enumeration in a child process with a per-module timeout.
pub struct ChildProcessEnumerator {
    program: PathBuf,
    extra_args: Vec<OsString>,
    limits: IsolationLimits,
}

impl ChildProcessEnumerator {
    /// The scanner binary itself, when this process is that binary.
    ///
    /// Test binaries are deliberately excluded: spawning one with the worker
    /// flag would run a test suite instead of a module.
    pub fn current_scanner(limits: IsolationLimits) -> Option<Self> {
        let exe = std::env::current_exe().ok()?;
        if exe.file_stem().and_then(|stem| stem.to_str()) != Some("opentrack-plugin-scan") {
            return None;
        }
        Some(Self {
            program: exe,
            extra_args: Vec::new(),
            limits: limits.sanitized(),
        })
    }

    /// The scanner binary shipped next to an application executable.
    ///
    /// The application is not the scanner, so it spawns the copy beside it. The
    /// build scripts stage `opentrack-plugin-scan` next to `OpenTrack` for this.
    pub fn sibling_of(directory: &Path, limits: IsolationLimits) -> Option<Self> {
        let name = if cfg!(windows) {
            "opentrack-plugin-scan.exe"
        } else {
            "opentrack-plugin-scan"
        };
        let candidate = directory.join(name);
        candidate.is_file().then(|| Self {
            program: candidate,
            extra_args: Vec::new(),
            limits: limits.sanitized(),
        })
    }

    /// Resolves the worker for the running executable: the scanner itself, or
    /// the copy staged beside an application executable.
    pub fn for_current_executable(limits: IsolationLimits) -> Option<Self> {
        if let Some(scanner) = Self::current_scanner(limits) {
            return Some(scanner);
        }
        let exe = std::env::current_exe().ok()?;
        Self::sibling_of(exe.parent()?, limits)
    }

    /// Builds an enumerator around any worker program. Used by the isolation
    /// tests to drive a process that hangs or exits abnormally.
    pub fn with_program(
        program: impl Into<PathBuf>,
        args: &[&str],
        limits: IsolationLimits,
    ) -> Self {
        Self {
            program: program.into(),
            extra_args: args.iter().map(OsString::from).collect(),
            limits: limits.sanitized(),
        }
    }

    fn command(&self, path: &Path, output: &Path) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.extra_args)
            .arg("--enumerate-module")
            .arg(path)
            .arg("--output")
            .arg(output)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        command
    }
}

/// Poll interval while waiting for a worker.
const POLL_INTERVAL: Duration = Duration::from_millis(20);
static NEXT_OUTPUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn worker_output_path() -> PathBuf {
    let serial = NEXT_OUTPUT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "opentrack-enumeration-{}-{serial}.json",
        std::process::id()
    ))
}

/// Waits for the child, killing it once the budget is spent.
fn wait_with_limit(child: &mut Child, limit: Duration) -> Result<std::process::ExitStatus, Output> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(error) => return Err(Output::Crashed(format!("could not be waited on: {error}"))),
        }
        if started.elapsed() >= limit {
            // The module may be wedged inside the worker; killing the worker is
            // the only way to take the scan back.
            let _ = child.kill();
            let _ = child.wait();
            return Err(Output::TimedOut);
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

enum Output {
    TimedOut,
    Crashed(String),
}

impl ModuleEnumerator for ChildProcessEnumerator {
    fn enumerate(&self, path: &Path) -> IsolationOutcome {
        let output_path = worker_output_path();
        let _ = std::fs::remove_file(&output_path);
        let mut command = self.command(path, &output_path);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return IsolationOutcome::Crashed(format!(
                    "could not start the module worker: {error}"
                ));
            }
        };
        let status = match wait_with_limit(&mut child, self.limits.timeout) {
            Ok(status) => status,
            Err(Output::TimedOut) => {
                let _ = std::fs::remove_file(&output_path);
                return IsolationOutcome::TimedOut {
                    limit: self.limits.timeout,
                };
            }
            Err(Output::Crashed(reason)) => {
                let _ = std::fs::remove_file(&output_path);
                return IsolationOutcome::Crashed(reason);
            }
        };
        let bytes = std::fs::read(&output_path);
        let _ = std::fs::remove_file(&output_path);

        if !status.success() {
            // A worker that reported an error writes the reason to its stderr;
            // a worker that died leaves a platform-specific status instead.
            let stderr = child
                .stderr
                .take()
                .map(|mut stderr| {
                    let mut text = String::new();
                    let _ = io::Read::read_to_string(&mut stderr, &mut text);
                    text
                })
                .unwrap_or_default();
            let reason = if stderr.trim().is_empty() {
                describe_status(status)
            } else {
                format!("{}: {}", describe_status(status), stderr.trim())
            };
            return IsolationOutcome::Crashed(reason);
        }
        match bytes {
            Ok(bytes) => match serde_json::from_slice::<WorkerResult>(&bytes) {
                Ok(WorkerResult::Enumerated(payload)) => match payload.into_module_enumeration() {
                    Ok(enumeration) => IsolationOutcome::Enumerated(enumeration),
                    Err(error) => IsolationOutcome::Failed(error),
                },
                Ok(WorkerResult::Failed(reason)) => IsolationOutcome::Failed(reason),
                Err(error) => IsolationOutcome::Crashed(format!(
                    "worker produced an unreadable result: {error}"
                )),
            },
            Err(error) => IsolationOutcome::Crashed(format!(
                "worker finished without writing a result: {error}"
            )),
        }
    }

    fn runs_in_process(&self) -> bool {
        false
    }
}

fn describe_status(status: std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("exited with code {code}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return format!("was terminated by signal {signal}");
        }
    }
    "ended abnormally".to_owned()
}

/// Enumerates one module for the worker process, writing the payload if asked.
///
/// The parent and the workspace tests both use this, so the worker path has one
/// implementation.
pub fn run_worker(path: &Path, output: Option<&Path>) -> Result<(), String> {
    let result = match InProcessEnumerator.enumerate(path) {
        IsolationOutcome::Enumerated(enumeration) => {
            WorkerResult::Enumerated(Box::new(EnumerationPayload::from(&enumeration)))
        }
        other => WorkerResult::Failed(
            other
                .diagnostic()
                .unwrap_or_else(|| "module enumeration failed".to_owned()),
        ),
    };
    let Some(output) = output else {
        return Err("the worker needs --output PATH to report its result".to_owned());
    };
    let bytes =
        serde_json::to_vec(&result).map_err(|error| format!("cannot encode result: {error}"))?;
    std::fs::write(output, bytes)
        .map_err(|error| format!("cannot write {}: {error}", output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing_module() -> PathBuf {
        std::env::temp_dir().join(format!(
            "opentrack-isolation-missing-{}.vst3",
            std::process::id()
        ))
    }

    #[test]
    fn an_application_finds_the_scanner_staged_beside_it() {
        let name = if cfg!(windows) {
            "opentrack-plugin-scan.exe"
        } else {
            "opentrack-plugin-scan"
        };
        let directory = std::env::temp_dir().join(format!(
            "opentrack-isolation-sibling-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("temp dir");
        assert!(
            ChildProcessEnumerator::sibling_of(&directory, IsolationLimits::default()).is_none(),
            "no worker next to the application yet"
        );

        std::fs::write(directory.join(name), b"worker").expect("write worker");
        let enumerator = ChildProcessEnumerator::sibling_of(&directory, IsolationLimits::default())
            .expect("the staged worker is found");
        assert_eq!(enumerator.program, directory.join(name));
        assert!(!enumerator.runs_in_process());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn timeouts_are_clamped_into_a_usable_range() {
        let tiny = IsolationLimits {
            timeout: Duration::from_millis(1),
        }
        .sanitized();
        assert_eq!(tiny.timeout, MIN_MODULE_TIMEOUT);
        let huge = IsolationLimits {
            timeout: Duration::from_secs(10_000),
        }
        .sanitized();
        assert_eq!(huge.timeout, MAX_MODULE_TIMEOUT);
        assert_eq!(IsolationLimits::default().timeout, DEFAULT_MODULE_TIMEOUT);
    }

    #[test]
    fn the_payload_round_trips_through_json() {
        let class = opentrack_vst3::Vst3ClassInfo {
            class_id: opentrack_plugins::PluginClassId::from_bytes([7; 16]),
            name: "Studio Compressor".to_owned(),
            vendor: "Example Audio".to_owned(),
            version: "1.2.3".to_owned(),
            category: "Audio Module Class".to_owned(),
            subcategories: "Fx|Dynamics".to_owned(),
            cardinality: 1,
            class_flags: 0,
        };
        let enumeration = opentrack_vst3::ModuleEnumeration {
            factory_vendor: "Example Audio".to_owned(),
            factory_url: String::new(),
            factory_email: String::new(),
            classes: vec![class.clone()],
            diagnostics: vec!["note".to_owned()],
        };
        let payload = EnumerationPayload::from(&enumeration);
        let encoded = serde_json::to_string(&payload).expect("encode");
        let decoded: EnumerationPayload = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded, payload);
        let restored = decoded.into_module_enumeration().expect("rebuild");
        assert_eq!(restored.classes.len(), 1);
        assert_eq!(restored.classes[0].name, class.name);
        assert_eq!(restored.classes[0].class_id, class.class_id);
        assert_eq!(restored.diagnostics, vec!["note".to_owned()]);
    }

    #[test]
    fn an_invalid_class_id_in_a_worker_result_is_reported() {
        let payload = EnumerationPayload {
            classes: vec![ClassPayload {
                class_id: "not-a-class-id".to_owned(),
                name: "Broken".to_owned(),
                vendor: String::new(),
                version: String::new(),
                category: String::new(),
                subcategories: String::new(),
                cardinality: 1,
                class_flags: 0,
            }],
            ..EnumerationPayload::default()
        };
        assert!(payload.into_module_enumeration().is_err());
    }

    #[test]
    fn in_process_enumeration_reports_a_missing_module_as_a_failure() {
        let outcome = InProcessEnumerator.enumerate(&missing_module());
        assert!(
            matches!(outcome, IsolationOutcome::Failed(_)),
            "{outcome:?}"
        );
        assert!(outcome.diagnostic().is_some());
    }

    #[test]
    fn a_missing_worker_program_is_attributed_to_the_module() {
        let enumerator = ChildProcessEnumerator::with_program(
            missing_module().with_file_name("no-such-worker"),
            &[],
            IsolationLimits {
                timeout: Duration::from_secs(1),
            },
        );
        let outcome = enumerator.enumerate(&missing_module());
        assert!(
            matches!(outcome, IsolationOutcome::Crashed(_)),
            "{outcome:?}"
        );
        assert!(
            outcome
                .diagnostic()
                .is_some_and(|text| text.contains("could not start the module worker")),
            "{outcome:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_hanging_module_is_killed_and_reported_as_a_timeout() {
        // `sleep` stands in for a module that never returns; the worker
        // arguments land in `$0..` and are ignored by the shell.
        let enumerator = ChildProcessEnumerator::with_program(
            "/bin/sh",
            &["-c", "sleep 60", "sh"],
            IsolationLimits {
                timeout: Duration::from_secs(1),
            },
        );
        let started = Instant::now();
        let outcome = enumerator.enumerate(&missing_module());
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the scan waited for the module instead of enforcing the budget"
        );
        match outcome {
            IsolationOutcome::TimedOut { limit } => assert_eq!(limit, Duration::from_secs(1)),
            other => panic!("expected a timeout, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_crashing_module_is_attributed_by_exit_status() {
        let enumerator = ChildProcessEnumerator::with_program(
            "/bin/sh",
            &["-c", "echo 'boom: segmentation fault' >&2; exit 139"],
            IsolationLimits::default(),
        );
        let outcome = enumerator.enumerate(&missing_module());
        let diagnostic = outcome.diagnostic().expect("a reason is recorded");
        assert!(diagnostic.contains("exited with code 139"), "{diagnostic}");
        assert!(
            diagnostic.contains("boom"),
            "worker stderr is kept: {diagnostic}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_worker_that_exits_cleanly_without_a_result_is_reported() {
        let enumerator =
            ChildProcessEnumerator::with_program("/bin/true", &[], IsolationLimits::default());
        let outcome = enumerator.enumerate(&missing_module());
        assert!(
            matches!(outcome, IsolationOutcome::Crashed(_)),
            "{outcome:?}"
        );
        assert!(
            outcome
                .diagnostic()
                .is_some_and(|text| text.contains("without writing a result")),
            "{outcome:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_worker_result_is_read_back_and_decoded() {
        let output = worker_output_path();
        let payload = WorkerResult::Enumerated(Box::new(EnumerationPayload {
            factory_vendor: "Example Audio".to_owned(),
            ..EnumerationPayload::default()
        }));
        let encoded = serde_json::to_string(&payload).expect("encode");
        // Write the payload to the last argument, which is the output path.
        let script = format!("for last; do :; done; printf '%s' '{encoded}' > \"$last\"");
        let enumerator = ChildProcessEnumerator::with_program(
            "/bin/sh",
            &["-c", &script, "sh"],
            IsolationLimits::default(),
        );
        // The worker is told the output path by the last two arguments, so this
        // exercises the same read-back path the scanner uses.
        match enumerator.enumerate(&missing_module()) {
            IsolationOutcome::Enumerated(enumeration) => {
                assert_eq!(enumeration.factory_vendor, "Example Audio")
            }
            other => panic!("expected an enumeration, got {other:?}"),
        }
        let _ = std::fs::remove_file(output);
    }
}
