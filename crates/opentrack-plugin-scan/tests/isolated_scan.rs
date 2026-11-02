// SPDX-License-Identifier: Apache-2.0

//! End-to-end checks for per-module enumeration isolation.
//!
//! These drive the real scanner binary, so they cover the parent/worker protocol
//! that unit tests can only model: the worker's exit status, its written result
//! file, and the parent's attribution of a module that cannot load.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use opentrack_plugin_scan::{
    ChildProcessEnumerator, DiscoveryLimits, FolderStatus, IsolationLimits, ModuleEnumerator,
    ScanReport, WorkerResult, build_report_with,
};
use opentrack_plugins::PluginScanStatus;

fn scanner_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_opentrack-plugin-scan"))
}

fn temp_folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "opentrack-isolation-it-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("temp folder");
    folder
}

/// A minimal host-compatible module image.
///
/// The precheck only reads the executable header, so a header-only file reaches
/// enumeration exactly like a real module would and then fails to load — which is
/// the case this test needs.
fn module_image() -> Vec<u8> {
    #[cfg(windows)]
    {
        // MZ, e_lfanew = 0x40, "PE\0\0", machine = AMD64.
        let mut bytes = vec![0_u8; 0x60];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x40_u32.to_le_bytes());
        bytes[0x40..0x44].copy_from_slice(b"PE\0\0");
        bytes[0x44..0x46].copy_from_slice(&0x8664_u16.to_le_bytes());
        bytes
    }
    #[cfg(not(windows))]
    {
        // ELF64, little endian, machine = EM_X86_64 (62).
        let mut bytes = vec![0_u8; 0x60];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2; // 64-bit
        bytes[5] = 1; // little endian
        bytes[6] = 1; // current version
        bytes[16..18].copy_from_slice(&2_u16.to_le_bytes()); // ET_EXEC
        bytes[18..20].copy_from_slice(&62_u16.to_le_bytes()); // EM_X86_64
        bytes
    }
}

/// Runs the real scanner binary as the per-module worker.
///
/// The application resolves the worker from its own executable; a test binary is
/// not the scanner, so the worker program is passed explicitly here.
fn single_module_report(folder: &Path) -> ScanReport {
    let isolation = IsolationLimits {
        timeout: Duration::from_secs(20),
    };
    let worker = ChildProcessEnumerator::with_program(scanner_binary(), &[], isolation);
    assert!(
        !worker.runs_in_process(),
        "this report must come from the child process"
    );
    build_report_with(
        &[folder.to_path_buf()],
        DiscoveryLimits::default(),
        0,
        Some(&worker),
    )
}

#[test]
fn the_worker_mode_reports_a_module_it_cannot_load() {
    let folder = temp_folder("worker");
    let module = folder.join("Bogus.vst3");
    let result = folder.join("result.json");
    std::fs::write(&module, b"not a plugin at all").expect("write module");

    let output = Command::new(scanner_binary())
        .arg("--enumerate-module")
        .arg(&module)
        .arg("--output")
        .arg(&result)
        .output()
        .expect("run the scanner worker");

    // The worker exits successfully when the *module* failed, so the parent can
    // tell a refused module from a worker that died.
    assert!(
        output.status.success(),
        "a refused module is a result, not a worker failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload = std::fs::read_to_string(&result).expect("the worker writes its result");
    match serde_json::from_str::<WorkerResult>(&payload).expect("result is valid JSON") {
        WorkerResult::Failed(reason) => assert!(
            reason.contains("module"),
            "the reason names the failure: {reason}"
        ),
        WorkerResult::Enumerated(_) => panic!("a non-plugin cannot enumerate: {payload}"),
    }
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn the_worker_mode_needs_an_output_path() {
    let folder = temp_folder("worker-output");
    let module = folder.join("Bogus.vst3");
    std::fs::write(&module, b"not a plugin at all").expect("write module");

    let output = Command::new(scanner_binary())
        .arg("--enumerate-module")
        .arg(&module)
        .output()
        .expect("run the scanner worker");
    assert!(
        !output.status.success(),
        "without --output the worker cannot report anything"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--output"),
        "the worker says what is missing"
    );
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn an_isolated_scan_attributes_an_unloadable_module_and_keeps_going() {
    let folder = temp_folder("attribution");
    std::fs::write(folder.join("Broken.vst3"), module_image()).expect("write module");
    std::fs::write(folder.join("Noise.vst3"), b"garbage").expect("write module");

    let report = single_module_report(&folder);
    assert_eq!(report.folders[0].status, FolderStatus::Scanned);
    assert_eq!(
        report.totals.modules, 2,
        "both modules are reported: {report:?}"
    );

    let broken = report
        .modules
        .iter()
        .find(|module| module.path.ends_with("Broken.vst3"))
        .expect("the host-compatible module is reported");
    assert_eq!(
        broken.status,
        PluginScanStatus::LoadFailed,
        "a module that fails inside the worker keeps its own status"
    );
    let diagnostic = broken.diagnostics.as_deref().unwrap_or_default();
    assert!(
        !diagnostic.is_empty(),
        "the failure names a reason: {diagnostic}"
    );

    let noise = report
        .modules
        .iter()
        .find(|module| module.path.ends_with("Noise.vst3"))
        .expect("the unreadable module is reported");
    assert_eq!(noise.status, PluginScanStatus::InvalidModule);

    assert!(
        report.plugins.is_empty(),
        "nothing was enumerated from these files"
    );
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn a_discovery_only_scan_never_loads_a_module() {
    let folder = temp_folder("discovery-only");
    std::fs::write(folder.join("Broken.vst3"), module_image()).expect("write module");

    let report = opentrack_plugin_scan::scan_folders(&[folder.clone()], DiscoveryLimits::default());
    let module = &report.modules[0];
    assert_eq!(module.status, PluginScanStatus::ReadyForEnumeration);
    assert!(module.class_ids.is_empty());
    assert!(report.plugins.is_empty());
    let _ = std::fs::remove_dir_all(folder);
}
