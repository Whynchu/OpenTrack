// SPDX-License-Identifier: Apache-2.0

//! Configured VST3 folder discovery.
//!
//! Discovery follows `docs/plugin-hosting-plan.md` section 4.12:
//!
//! - recursively find `.vst3` bundles and single-file modules;
//! - inside a bundle, select the native VST3 binary directory for this target;
//! - never recurse into `Contents\Resources` or documentation;
//! - deduplicate canonical module paths, including overlapping configured
//!   folders;
//! - bound traversal depth, module count, warnings, and diagnostic text.
//!
//! No module is loaded here. The executable-header precheck happens after
//! discovery in [`crate::report`]. Symbolic links are skipped to avoid cycles;
//! Windows junctions are reported by the file system as reparse points and
//! are skipped by the same rule.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::bounded_diagnostic;

/// Default maximum directory recursion depth.
pub const DEFAULT_MAX_DEPTH: usize = 32;

/// Default maximum modules retained by one discovery run.
pub const DEFAULT_MAX_MODULES: usize = 4_096;

/// Hard cap for caller-supplied depth.
pub const MAX_DEPTH_LIMIT: usize = 64;

/// Hard cap for caller-supplied module count.
pub const MAX_MODULES_LIMIT: usize = 16_384;

const MAX_WARNINGS: usize = 64;

const WINDOWS_PLATFORM_DIRECTORIES: [&str; 3] = ["x86_64-win", "x86-win", "arm64-win"];

/// Traversal bounds for one scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscoveryLimits {
    /// Maximum directory depth below each configured folder.
    pub max_depth: usize,
    /// Maximum modules retained by the whole scan.
    pub max_modules: usize,
}

impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            max_modules: DEFAULT_MAX_MODULES,
        }
    }
}

impl DiscoveryLimits {
    /// Returns limits clamped to the supported hard caps.
    pub fn normalized(self) -> Self {
        Self {
            max_depth: self.max_depth.min(MAX_DEPTH_LIMIT),
            max_modules: self.max_modules.clamp(1, MAX_MODULES_LIMIT),
        }
    }
}

/// How a module path was found.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    /// A `.vst3` file directly in a scanned folder.
    SingleFile,
    /// A module inside a `.vst3` bundle.
    Bundle,
}

impl ModuleKind {
    /// Short stable label for report text.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleFile => "single",
            Self::Bundle => "bundle",
        }
    }
}

impl fmt::Display for ModuleKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One discovered module image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredModule {
    /// Path of the loadable module file.
    pub path: PathBuf,
    /// Discovery shape.
    pub kind: ModuleKind,
    /// Bundle directory when `kind` is [`ModuleKind::Bundle`].
    pub bundle_path: Option<PathBuf>,
    /// `Contents\<platform>` directory name selected for a bundle.
    pub platform_directory: Option<String>,
}

/// Result of reading one configured folder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderOutcome {
    /// The folder was read; individual entry failures appear as warnings.
    Scanned,
    /// The configured folder does not exist.
    Missing,
    /// The configured path exists but is not a directory.
    NotDirectory,
    /// The folder could not be read at all.
    Error(String),
}

/// Bounded discovery result for one configured folder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderDiscovery {
    /// Configured folder path.
    pub folder: PathBuf,
    /// Overall folder outcome.
    pub outcome: FolderOutcome,
    /// Unique modules found below the folder.
    pub modules: Vec<DiscoveredModule>,
    /// Bounded warnings for malformed bundles, skipped links, and unreadable
    /// entries.
    pub warnings: Vec<String>,
    /// Paths skipped because they were already discovered.
    pub duplicates_skipped: usize,
    /// True when [`DiscoveryLimits::max_modules`] was reached.
    pub truncated: bool,
}

/// Combined discovery across all configured folders.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryBatch {
    /// Per-folder results in configured order.
    pub folders: Vec<FolderDiscovery>,
    /// Globally deduplicated modules, sorted by path.
    pub modules: Vec<DiscoveredModule>,
    /// Duplicate paths skipped across all folders.
    pub duplicates_skipped: usize,
    /// True when any folder or the combined limit was reached.
    pub truncated: bool,
}

/// Discovers modules in one folder.
pub fn discover_folder(folder: &Path, limits: DiscoveryLimits) -> FolderDiscovery {
    let limits = limits.normalized();
    let mut result = FolderDiscovery {
        folder: folder.to_path_buf(),
        outcome: FolderOutcome::Scanned,
        modules: Vec::new(),
        warnings: Vec::new(),
        duplicates_skipped: 0,
        truncated: false,
    };

    match std::fs::metadata(folder) {
        Ok(metadata) if !metadata.is_dir() => {
            result.outcome = FolderOutcome::NotDirectory;
            return result;
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            result.outcome = FolderOutcome::Missing;
            return result;
        }
        Err(error) => {
            result.outcome = FolderOutcome::Error(bounded_diagnostic(format!(
                "cannot read folder {}: {error}",
                folder.display()
            )));
            return result;
        }
    }

    if let Err(error) = std::fs::read_dir(folder) {
        result.outcome = FolderOutcome::Error(bounded_diagnostic(format!(
            "cannot read folder {}: {error}",
            folder.display()
        )));
        return result;
    }

    let mut discoverer = Discoverer {
        limits,
        modules: Vec::new(),
        warnings: Vec::new(),
        seen_modules: HashSet::new(),
        visited_directories: HashSet::new(),
        duplicates_skipped: 0,
        truncated: false,
    };

    let root_key = path_key(folder);
    discoverer.visited_directories.insert(root_key);
    let _ = discoverer.walk(folder, 0);

    result.modules = discoverer.modules;
    result
        .modules
        .sort_by(|left, right| left.path.cmp(&right.path));
    result.warnings = discoverer.warnings;
    result.duplicates_skipped = discoverer.duplicates_skipped;
    result.truncated = discoverer.truncated;
    result
}

/// Discovers modules across all configured folders and deduplicates paths.
pub fn discover_all(folders: &[PathBuf], limits: DiscoveryLimits) -> DiscoveryBatch {
    let limits = limits.normalized();
    let mut per_folder = Vec::with_capacity(folders.len());
    let mut modules = Vec::new();
    let mut seen = HashSet::new();
    let mut global_duplicates = 0usize;
    let mut truncated = false;

    for folder in folders {
        let discovered = discover_folder(folder, limits);
        truncated |= discovered.truncated;
        for module in &discovered.modules {
            if modules.len() >= limits.max_modules {
                truncated = true;
                continue;
            }
            let key = path_key(&module.path);
            if seen.insert(key) {
                modules.push(module.clone());
            } else {
                global_duplicates += 1;
            }
        }
        per_folder.push(discovered);
    }

    modules.sort_by(|left, right| left.path.cmp(&right.path));
    DiscoveryBatch {
        duplicates_skipped: per_folder
            .iter()
            .map(|folder| folder.duplicates_skipped)
            .sum::<usize>()
            + global_duplicates,
        truncated,
        folders: per_folder,
        modules,
    }
}

struct Discoverer {
    limits: DiscoveryLimits,
    modules: Vec<DiscoveredModule>,
    warnings: Vec<String>,
    seen_modules: HashSet<PathBuf>,
    visited_directories: HashSet<PathBuf>,
    duplicates_skipped: usize,
    truncated: bool,
}

impl Discoverer {
    /// Walks a directory. Returns true when traversal should stop.
    fn walk(&mut self, directory: &Path, depth: usize) -> bool {
        let read_dir = match std::fs::read_dir(directory) {
            Ok(read_dir) => read_dir,
            Err(error) => {
                self.warn(format!(
                    "cannot read directory {}: {error}",
                    directory.display()
                ));
                return false;
            }
        };

        let mut entries = Vec::new();
        for entry in read_dir {
            match entry {
                Ok(entry) => entries.push(entry),
                Err(error) => self.warn(format!(
                    "cannot read an entry in {}: {error}",
                    directory.display()
                )),
            }
        }
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    self.warn(format!("cannot inspect {}: {error}", path.display()));
                    continue;
                }
            };

            if file_type.is_symlink() {
                self.warn(format!("skipped symbolic link {}", path.display()));
                continue;
            }

            if file_type.is_dir() {
                if is_vst3_name(&path) {
                    match find_bundle_module(&path, true) {
                        Ok(Some(module)) => {
                            if self.add_module(module) {
                                return true;
                            }
                        }
                        Ok(None) => self.warn(format!(
                            "VST3 bundle has no module under Contents: {}",
                            path.display()
                        )),
                        Err(error) => self.warn(format!(
                            "cannot inspect VST3 bundle {}: {error}",
                            path.display()
                        )),
                    }
                    continue;
                }

                if depth >= self.limits.max_depth {
                    continue;
                }
                let key = path_key(&path);
                if self.visited_directories.insert(key) && self.walk(&path, depth + 1) {
                    return true;
                }
            } else if file_type.is_file() && is_vst3_name(&path) {
                let module = DiscoveredModule {
                    path,
                    kind: ModuleKind::SingleFile,
                    bundle_path: None,
                    platform_directory: None,
                };
                if self.add_module(module) {
                    return true;
                }
            }
        }
        false
    }

    /// Adds a module. Returns true when the module limit was reached.
    fn add_module(&mut self, module: DiscoveredModule) -> bool {
        let key = path_key(&module.path);
        if !self.seen_modules.insert(key) {
            self.duplicates_skipped += 1;
            return false;
        }
        self.modules.push(module);
        if self.modules.len() >= self.limits.max_modules {
            self.truncated = true;
            return true;
        }
        false
    }

    fn warn(&mut self, warning: String) {
        if self.warnings.len() < MAX_WARNINGS {
            self.warnings.push(bounded_diagnostic(warning));
        }
    }
}

/// Resolves a `.vst3` bundle to its native executable module, if present.
/// Resources and arbitrary subdirectories are never walked.
pub fn resolve_vst3_bundle(bundle: &Path) -> Result<Option<PathBuf>, String> {
    find_bundle_module(bundle, false).map(|module| module.map(|module| module.path))
}

/// Finds the module selected from a `.vst3` bundle.
fn find_bundle_module(
    bundle: &Path,
    _include_incompatible_architectures: bool,
) -> Result<Option<DiscoveredModule>, String> {
    let contents = bundle.join("Contents");
    let preferred_stem = bundle.file_stem();

    if contents.is_dir() {
        let platform_directories = read_platform_directories(&contents)?;
        for wanted in preferred_platform_directories() {
            let Some((actual_name, directory)) = platform_directories
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
            else {
                continue;
            };
            if let Some(module) = select_module(directory, preferred_stem)? {
                return Ok(Some(DiscoveredModule {
                    path: module,
                    kind: ModuleKind::Bundle,
                    bundle_path: Some(bundle.to_path_buf()),
                    platform_directory: Some(actual_name.clone()),
                }));
            }
        }
        #[cfg(windows)]
        if _include_incompatible_architectures {
            for wanted in WINDOWS_PLATFORM_DIRECTORIES {
                if preferred_platform_directories().contains(&wanted) {
                    continue;
                }
                let Some((actual_name, directory)) = platform_directories
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
                else {
                    continue;
                };
                if let Some(module) = select_module(directory, preferred_stem)? {
                    return Ok(Some(DiscoveredModule {
                        path: module,
                        kind: ModuleKind::Bundle,
                        bundle_path: Some(bundle.to_path_buf()),
                        platform_directory: Some(actual_name.clone()),
                    }));
                }
            }
        }
    }

    // Preserve the older Windows single-file bundle layout. On macOS and
    // Linux, bundle roots contain metadata and resources, so treating an
    // arbitrary root file as executable would select the wrong payload.
    #[cfg(windows)]
    {
        if let Some(module) = select_module(bundle, preferred_stem)? {
            return Ok(Some(DiscoveredModule {
                path: module,
                kind: ModuleKind::Bundle,
                bundle_path: Some(bundle.to_path_buf()),
                platform_directory: None,
            }));
        }
    }

    Ok(None)
}

fn read_platform_directories(contents: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let read_dir =
        std::fs::read_dir(contents).map_err(|error| format!("cannot read Contents: {error}"))?;
    let mut directories = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|error| format!("cannot read Contents entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("cannot inspect Contents entry: {error}"))?;
        if !file_type.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        directories.push((name, entry.path()));
    }
    directories.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(directories)
}

/// Selects one `.vst3` file from a platform or bundle directory.
fn select_module(
    directory: &Path,
    preferred_stem: Option<&OsStr>,
) -> Result<Option<PathBuf>, String> {
    let read_dir = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read module directory: {error}"))?;
    let mut candidates = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|error| format!("cannot read module entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("cannot inspect module entry: {error}"))?;
        if file_type.is_file() && is_native_module_file(&entry.path()) {
            candidates.push(entry.path());
        }
    }
    candidates.sort();

    if let Some(preferred_stem) = preferred_stem
        && let Some(preferred) = candidates.iter().find(|candidate| {
            candidate.file_stem().is_some_and(|stem| {
                stem.to_string_lossy()
                    .eq_ignore_ascii_case(&preferred_stem.to_string_lossy())
            })
        })
    {
        return Ok(Some(preferred.clone()));
    }

    Ok(candidates.into_iter().next())
}

fn preferred_platform_directories() -> Vec<&'static str> {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        return vec!["x86_64-win"];
    }
    #[cfg(all(windows, target_arch = "x86"))]
    {
        return vec!["x86-win"];
    }
    #[cfg(all(windows, target_arch = "aarch64"))]
    {
        return vec!["arm64-win"];
    }
    #[cfg(target_os = "macos")]
    {
        return vec!["MacOS"];
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return vec!["x86_64-linux"];
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return vec!["aarch64-linux"];
    }
    #[cfg(all(target_os = "linux", target_arch = "x86"))]
    {
        return vec!["i386-linux", "i686-linux"];
    }
    #[allow(unreachable_code)]
    Vec::new()
}

fn is_native_module_file(path: &Path) -> bool {
    #[cfg(windows)]
    {
        return is_vst3_name(path);
    }
    #[cfg(target_os = "macos")]
    {
        // The VST3 format puts the Mach-O payload in Contents/MacOS without
        // requiring a filename extension.
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        return path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("so") || extension.eq_ignore_ascii_case("vst3")
        });
    }
    #[allow(unreachable_code)]
    is_vst3_name(path)
}

fn is_vst3_name(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
}

/// Returns a canonical path for duplicate control, falling back to the path
/// itself when the file system cannot canonicalize it.
fn path_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn names(modules: &[DiscoveredModule]) -> Vec<String> {
        modules
            .iter()
            .map(|module| {
                module
                    .path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    fn native_platform_directory() -> &'static str {
        preferred_platform_directories()
            .first()
            .copied()
            .expect("this target has a VST3 platform directory")
    }

    #[cfg(windows)]
    fn native_module_filename(stem: &str) -> String {
        format!("{stem}.vst3")
    }

    #[cfg(target_os = "macos")]
    fn native_module_filename(stem: &str) -> String {
        stem.to_owned()
    }

    #[cfg(target_os = "linux")]
    fn native_module_filename(stem: &str) -> String {
        format!("{stem}.so")
    }

    #[cfg(windows)]
    fn incompatible_windows_platform_directory() -> &'static str {
        WINDOWS_PLATFORM_DIRECTORIES
            .iter()
            .copied()
            .find(|directory| !preferred_platform_directories().contains(directory))
            .expect("Windows has another known VST3 platform directory")
    }

    #[test]
    fn finds_bundles_singles_and_ignores_resources() {
        let temp = TempDir::new("discovery");
        let native_directory = native_platform_directory();
        let native_filename = native_module_filename("Bundle");
        temp.write(
            &format!("Bundle.vst3/Contents/{native_directory}/{native_filename}"),
            b"native fixture",
        );
        #[cfg(windows)]
        {
            let incompatible_directory = incompatible_windows_platform_directory();
            temp.write(
                &format!("Bundle.vst3/Contents/{incompatible_directory}/Bundle.vst3"),
                b"incompatible fixture",
            );
        }
        temp.write(
            "Bundle.vst3/Contents/Resources/documentation.vst3",
            b"must not be discovered",
        );
        temp.write("Singles/Effect.vst3", b"single fixture");
        temp.write("Singles/Nested/Instrument.vst3", b"nested fixture");

        let result = discover_folder(temp.path(), DiscoveryLimits::default());

        assert_eq!(result.outcome, FolderOutcome::Scanned);
        assert!(!result.truncated);
        assert_eq!(result.modules.len(), 3);
        assert_eq!(
            names(&result.modules),
            vec!["Bundle", "Effect", "Instrument"]
        );

        let bundle = result
            .modules
            .iter()
            .find(|module| module.kind == ModuleKind::Bundle)
            .expect("bundle module");
        assert_eq!(
            bundle
                .path
                .parent()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str()),
            Some(native_directory)
        );
        assert_eq!(bundle.platform_directory.as_deref(), Some(native_directory));
        assert_eq!(bundle.bundle_path, Some(temp.path().join("Bundle.vst3")));
        assert!(
            !result
                .modules
                .iter()
                .any(|module| module.path.to_string_lossy().contains("Resources"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn retains_incompatible_windows_bundle_for_precheck() {
        let temp = TempDir::new("x86-bundle");
        let incompatible_directory = incompatible_windows_platform_directory();
        temp.write(
            &format!("Legacy.vst3/Contents/{incompatible_directory}/Legacy.vst3"),
            b"incompatible fixture",
        );

        let result = discover_folder(temp.path(), DiscoveryLimits::default());

        assert_eq!(result.modules.len(), 1);
        assert_eq!(
            result.modules[0].platform_directory.as_deref(),
            Some(incompatible_directory)
        );
    }

    #[test]
    fn deduplicates_overlapping_folders() {
        let temp = TempDir::new("dedupe");
        temp.write(r"One.vst3", b"one");
        temp.write("Nested/Two.vst3", b"two");

        let folders = vec![
            temp.path().to_path_buf(),
            temp.path().join("Nested"),
            temp.path().to_path_buf(),
        ];
        let batch = discover_all(&folders, DiscoveryLimits::default());

        assert_eq!(batch.folders.len(), 3);
        assert_eq!(batch.modules.len(), 2);
        assert_eq!(batch.duplicates_skipped, 3);
        assert!(!batch.truncated);
    }

    #[test]
    fn respects_module_limit_and_depth() {
        let temp = TempDir::new("limits");
        temp.write(r"One.vst3", b"one");
        temp.write(r"Two.vst3", b"two");
        temp.write(r"Three.vst3", b"three");

        let limited = discover_folder(
            temp.path(),
            DiscoveryLimits {
                max_depth: 0,
                max_modules: 2,
            },
        );
        assert_eq!(limited.modules.len(), 2);
        assert!(limited.truncated);

        let shallow = discover_folder(
            temp.path(),
            DiscoveryLimits {
                max_depth: 0,
                max_modules: 10,
            },
        );
        assert_eq!(shallow.modules.len(), 3);
        assert!(!shallow.truncated);

        let nested = TempDir::new("depth");
        nested.write(r"Top.vst3", b"top");
        nested.write("Deep/Lower/Nested.vst3", b"nested");
        let depth_zero = discover_folder(
            nested.path(),
            DiscoveryLimits {
                max_depth: 0,
                max_modules: 10,
            },
        );
        assert_eq!(names(&depth_zero.modules), vec!["Top"]);

        let depth_two = discover_folder(
            nested.path(),
            DiscoveryLimits {
                max_depth: 2,
                max_modules: 10,
            },
        );
        assert_eq!(names(&depth_two.modules), vec!["Nested", "Top"]);
    }

    #[test]
    fn reports_missing_and_non_directory_folders() {
        let temp = TempDir::new("folder-status");
        let missing = discover_folder(
            &temp.path().join("does-not-exist"),
            DiscoveryLimits::default(),
        );
        assert_eq!(missing.outcome, FolderOutcome::Missing);
        assert!(missing.modules.is_empty());

        let file = temp.write("plain.vst3", b"not a directory");
        let not_directory = discover_folder(&file, DiscoveryLimits::default());
        assert_eq!(not_directory.outcome, FolderOutcome::NotDirectory);
    }

    #[test]
    fn warns_about_bundle_without_module() {
        let temp = TempDir::new("empty-bundle");
        temp.mkdir(&format!(
            "Empty.vst3/Contents/{}",
            native_platform_directory()
        ));

        let result = discover_folder(temp.path(), DiscoveryLimits::default());

        assert!(result.modules.is_empty());
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("has no module"))
        );
    }

    #[test]
    fn limits_are_clamped() {
        let limits = DiscoveryLimits {
            max_depth: usize::MAX,
            max_modules: 0,
        }
        .normalized();
        assert_eq!(limits.max_depth, MAX_DEPTH_LIMIT);
        assert_eq!(limits.max_modules, 1);
    }
}
