// SPDX-License-Identifier: Apache-2.0

//! Bounded scan report and JSON/text rendering.
//!
//! The report is the scanner's application-facing output. It records discovery
//! and PE precheck results only. Class enumeration is deliberately absent in
//! this foundation; every x64 module is marked `ready_for_enumeration` and the
//! report states that no module was loaded.
//!
//! Counts, warnings, and diagnostic text are bounded. Paths remain wide
//! `PathBuf` values during discovery; report strings are display copies, and a
//! non-UTF-8 path is explicitly flagged.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use opentrack_plugins::PluginArchitecture;
use opentrack_plugins::PluginClassId;
use opentrack_plugins::PluginScanStatus;
use serde::Deserialize;
use serde::Serialize;

use crate::SCAN_PROTOCOL_VERSION;
use crate::SCANNER_VERSION;
use crate::bounded_diagnostic;
use crate::discovery::DiscoveredModule;
use crate::discovery::DiscoveryLimits;
use crate::discovery::FolderOutcome;
use crate::discovery::ModuleKind;
use crate::discovery::discover_all;
use crate::pe::PeFileError;
use crate::pe::read_pe_machine_from_file;

/// Maximum configured folders represented in one report.
pub const MAX_REPORT_FOLDERS: usize = 64;

/// Overall result of reading one configured folder.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderStatus {
    /// Folder was read.
    Scanned,
    /// Folder does not exist.
    Missing,
    /// Configured path is not a directory.
    NotDirectory,
    /// Folder could not be read.
    Error,
}

/// Report entry for one configured folder.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FolderReport {
    /// Display copy of the folder path.
    pub path: String,
    /// False when `path` is a lossy display copy.
    pub path_utf8: bool,
    /// Folder outcome.
    pub status: FolderStatus,
    /// Unique modules found in this folder before global deduplication.
    pub modules_found: usize,
    /// Duplicate module paths skipped in this folder.
    pub duplicates_skipped: usize,
    /// Bounded warnings.
    pub warnings: Vec<String>,
    /// True when this folder reached a discovery limit.
    pub truncated: bool,
    /// Diagnostic for `status: error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<String>,
}

/// Report entry for one discovered module image.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModuleReport {
    /// Display copy of the module path.
    pub path: String,
    /// False when `path` is a lossy display copy.
    pub path_utf8: bool,
    /// Single-file or bundle discovery shape.
    pub kind: ModuleKind,
    /// Display copy of the bundle path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_path: Option<String>,
    /// Selected `Contents\<platform>` directory for a bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_directory: Option<String>,
    /// Architecture derived from the PE header.
    pub architecture: PluginArchitecture,
    /// Raw COFF machine value when the header was readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pe_machine: Option<u16>,
    /// Discovery/precheck status. Class enumeration is not implemented.
    pub status: PluginScanStatus,
    /// File size in bytes when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Class IDs are empty until `opentrack-vst3` enumeration exists.
    pub class_ids: Vec<PluginClassId>,
    /// Bounded diagnostic for rejection or path conversion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<String>,
}

/// Aggregate counts for a scan report.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScanTotals {
    /// Number of folders supplied by the caller.
    pub configured_folders: usize,
    /// Number of folder reports included.
    pub folders_reported: usize,
    /// Unique modules reported.
    pub modules: usize,
    /// Modules that passed the x64 PE precheck.
    pub ready_for_enumeration: usize,
    /// Modules rejected because their machine type is not x64.
    pub unsupported_architecture: usize,
    /// Modules rejected because they are not valid PE images.
    pub invalid_module: usize,
    /// Modules that could not be read.
    pub unreadable: usize,
    /// Modules with any rejected status.
    pub rejected: usize,
    /// Duplicate paths skipped across all folders.
    pub duplicates_skipped: usize,
    /// True when a configured folder or module limit was reached.
    pub truncated: bool,
}

/// Bounded scanner output shared by the JSON and text formats.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScanReport {
    /// Scanner report protocol version.
    pub protocol_version: u32,
    /// Scanner build version.
    pub scanner_version: String,
    /// Report generation time in Unix milliseconds.
    pub generated_at_unix_ms: u64,
    /// Folder results.
    pub folders: Vec<FolderReport>,
    /// Module results.
    pub modules: Vec<ModuleReport>,
    /// Aggregate counts.
    pub totals: ScanTotals,
    /// Fixed explanatory notes, including the enumeration TODO.
    pub notes: Vec<String>,
}

impl ScanReport {
    /// Serializes the report as pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Renders a deterministic, bounded plain-text report.
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(output, "OpenTrack VST3 plugin scan");
        let _ = writeln!(output, "protocol: {}", self.protocol_version);
        let _ = writeln!(output, "scanner: {}", self.scanner_version);
        let _ = writeln!(output, "generated_unix_ms: {}", self.generated_at_unix_ms);
        let _ = writeln!(
            output,
            "folders: {} configured, {} reported",
            self.totals.configured_folders, self.totals.folders_reported
        );

        for folder in &self.folders {
            let status = match folder.status {
                FolderStatus::Scanned => "ok",
                FolderStatus::Missing => "missing",
                FolderStatus::NotDirectory => "not-directory",
                FolderStatus::Error => "error",
            };
            let _ = writeln!(
                output,
                "  [{status}] {} ({} modules{})",
                one_line(&folder.path),
                folder.modules_found,
                if folder.truncated { ", truncated" } else { "" }
            );
            for warning in &folder.warnings {
                let _ = writeln!(output, "      warning: {}", one_line(warning));
            }
            if let Some(diagnostics) = &folder.diagnostics {
                let _ = writeln!(output, "      error: {}", one_line(diagnostics));
            }
        }

        let _ = writeln!(
            output,
            "modules: {} reported, {} ready for enumeration, {} rejected, {} duplicate paths skipped",
            self.totals.modules,
            self.totals.ready_for_enumeration,
            self.totals.rejected,
            self.totals.duplicates_skipped
        );
        for module in &self.modules {
            let label = match module.status {
                PluginScanStatus::ReadyForEnumeration => "READY",
                PluginScanStatus::Enumerated => "ENUMERATED",
                _ => "REJECT",
            };
            let machine = module
                .pe_machine
                .map(|machine| format!(" 0x{machine:04x}"))
                .unwrap_or_default();
            let _ = writeln!(
                output,
                "  {label:<10} {:<5} {:<6}{} {}",
                module.architecture.as_str(),
                module.kind.as_str(),
                machine,
                one_line(&module.path)
            );
            if let Some(diagnostics) = &module.diagnostics {
                let _ = writeln!(output, "      reason: {}", one_line(diagnostics));
            }
        }

        if self.totals.truncated {
            let _ = writeln!(output, "truncated: yes");
        }
        for note in &self.notes {
            let _ = writeln!(output, "note: {}", one_line(note));
        }
        output
    }
}

/// Discovers folders and builds a report with an explicit timestamp.
pub fn build_report(
    folders: &[PathBuf],
    limits: DiscoveryLimits,
    generated_at_unix_ms: u64,
) -> ScanReport {
    let configured_folders = folders.len();
    let reported_folders = &folders[..configured_folders.min(MAX_REPORT_FOLDERS)];
    let batch = discover_all(reported_folders, limits);

    let folder_reports = batch
        .folders
        .iter()
        .map(|folder| {
            let (status, diagnostics) = match &folder.outcome {
                FolderOutcome::Scanned => (FolderStatus::Scanned, None),
                FolderOutcome::Missing => (FolderStatus::Missing, None),
                FolderOutcome::NotDirectory => (FolderStatus::NotDirectory, None),
                FolderOutcome::Error(message) => (FolderStatus::Error, Some(message.clone())),
            };
            let (path, path_utf8) = display_path(&folder.folder);
            FolderReport {
                path,
                path_utf8,
                status,
                modules_found: folder.modules.len(),
                duplicates_skipped: folder.duplicates_skipped,
                warnings: folder.warnings.clone(),
                truncated: folder.truncated,
                diagnostics,
            }
        })
        .collect::<Vec<_>>();

    let module_reports = batch.modules.iter().map(inspect_module).collect::<Vec<_>>();

    let ready_for_enumeration = module_reports
        .iter()
        .filter(|module| {
            matches!(
                module.status,
                PluginScanStatus::ReadyForEnumeration | PluginScanStatus::Enumerated
            )
        })
        .count();
    let unsupported_architecture = module_reports
        .iter()
        .filter(|module| module.status == PluginScanStatus::UnsupportedArchitecture)
        .count();
    let invalid_module = module_reports
        .iter()
        .filter(|module| module.status == PluginScanStatus::InvalidModule)
        .count();
    let unreadable = module_reports
        .iter()
        .filter(|module| module.status == PluginScanStatus::Unreadable)
        .count();
    let rejected = module_reports
        .iter()
        .filter(|module| module.status.is_rejected())
        .count();

    let mut notes = vec![
        "Class enumeration is not implemented in this build. No VST3 module was loaded and no plug-in host is claimed; ready entries are PE precheck results only.".to_owned(),
    ];
    if configured_folders > reported_folders.len() {
        notes.push(format!(
            "{} configured folders were omitted by the {MAX_REPORT_FOLDERS}-folder report limit.",
            configured_folders - reported_folders.len()
        ));
    }
    if batch.truncated {
        notes.push(
            "A discovery limit was reached; additional modules may exist beyond this report."
                .to_owned(),
        );
    }
    if batch.duplicates_skipped > 0 {
        notes.push(format!(
            "{} duplicate module paths were skipped; class-ID deduplication will occur after enumeration.",
            batch.duplicates_skipped
        ));
    }

    ScanReport {
        protocol_version: SCAN_PROTOCOL_VERSION,
        scanner_version: SCANNER_VERSION.to_owned(),
        generated_at_unix_ms,
        totals: ScanTotals {
            configured_folders,
            folders_reported: folder_reports.len(),
            modules: module_reports.len(),
            ready_for_enumeration,
            unsupported_architecture,
            invalid_module,
            unreadable,
            rejected,
            duplicates_skipped: batch.duplicates_skipped,
            truncated: batch.truncated || configured_folders > reported_folders.len(),
        },
        folders: folder_reports,
        modules: module_reports,
        notes,
    }
}

/// Discovers folders and builds a report with the current time.
pub fn scan_folders(folders: &[PathBuf], limits: DiscoveryLimits) -> ScanReport {
    build_report(folders, limits, crate::unix_time_ms())
}

fn inspect_module(module: &DiscoveredModule) -> ModuleReport {
    let (path, path_utf8) = display_path(&module.path);
    let bundle_path = module
        .bundle_path
        .as_deref()
        .map(|bundle| display_path(bundle).0);
    let size_bytes = std::fs::metadata(&module.path)
        .ok()
        .map(|metadata| metadata.len());

    let mut diagnostics = Vec::new();
    if !path_utf8 {
        diagnostics.push(
            "module path is not valid UTF-8; report text uses a lossy display copy".to_owned(),
        );
    }

    // TODO(vst3): after `opentrack-vst3` exists, run the isolated factory
    // enumeration here and populate class IDs/descriptors. This foundation
    // deliberately stops at the PE precheck and never loads the module.
    let (architecture, pe_machine, status) = match read_pe_machine_from_file(&module.path) {
        Ok(machine) => {
            let architecture = machine.architecture();
            if architecture.is_host_compatible() {
                (
                    architecture,
                    Some(machine.raw()),
                    PluginScanStatus::ReadyForEnumeration,
                )
            } else {
                diagnostics.push(format!(
                    "PE machine {machine} is {architecture}; the x64 host accepts only x64 modules"
                ));
                (
                    architecture,
                    Some(machine.raw()),
                    PluginScanStatus::UnsupportedArchitecture,
                )
            }
        }
        Err(PeFileError::Io(error)) => {
            diagnostics.push(format!("cannot read module: {error}"));
            (
                PluginArchitecture::Other,
                None,
                PluginScanStatus::Unreadable,
            )
        }
        Err(PeFileError::Header(error)) => {
            diagnostics.push(format!("invalid module: {error}"));
            (
                PluginArchitecture::Other,
                None,
                PluginScanStatus::InvalidModule,
            )
        }
    };

    ModuleReport {
        path,
        path_utf8,
        kind: module.kind,
        bundle_path,
        platform_directory: module.platform_directory.clone(),
        architecture,
        pe_machine,
        status,
        size_bytes,
        class_ids: Vec::new(),
        diagnostics: join_diagnostics(diagnostics),
    }
}

fn join_diagnostics(diagnostics: Vec<String>) -> Option<String> {
    if diagnostics.is_empty() {
        return None;
    }
    Some(bounded_diagnostic(diagnostics.join("; ")))
}

fn display_path(path: &Path) -> (String, bool) {
    match path.to_str() {
        Some(path) => (path.to_owned(), true),
        None => (path.to_string_lossy().into_owned(), false),
    }
}

/// Keeps a report line from injecting extra lines or control sequences.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn pe_image(machine: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x80];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        bytes[0x40..0x44].copy_from_slice(b"PE\0\0");
        bytes[0x44..0x46].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    fn fixture_report() -> ScanReport {
        let temp = TempDir::new("report");
        temp.write(r"Good.vst3", &pe_image(0x8664));
        temp.write(r"Legacy.vst3", &pe_image(0x014c));
        temp.write(r"Broken.vst3", b"not a PE image");
        build_report(
            &[temp.path().to_path_buf()],
            DiscoveryLimits::default(),
            1_700_000_000_000,
        )
    }

    #[test]
    fn report_counts_and_statuses_are_correct() {
        let report = fixture_report();
        assert_eq!(report.totals.configured_folders, 1);
        assert_eq!(report.totals.modules, 3);
        assert_eq!(report.totals.ready_for_enumeration, 1);
        assert_eq!(report.totals.unsupported_architecture, 1);
        assert_eq!(report.totals.invalid_module, 1);
        assert_eq!(report.totals.rejected, 2);
        assert!(!report.totals.truncated);

        let good = report
            .modules
            .iter()
            .find(|module| module.path.ends_with("Good.vst3"))
            .expect("x64 module report");
        assert_eq!(good.status, PluginScanStatus::ReadyForEnumeration);
        assert_eq!(good.architecture, PluginArchitecture::X64);
        assert_eq!(good.pe_machine, Some(0x8664));
        assert!(good.class_ids.is_empty());

        let legacy = report
            .modules
            .iter()
            .find(|module| module.path.ends_with("Legacy.vst3"))
            .expect("x86 module report");
        assert_eq!(legacy.status, PluginScanStatus::UnsupportedArchitecture);
        assert_eq!(legacy.architecture, PluginArchitecture::X86);
    }

    #[test]
    fn report_round_trips_through_json() {
        let report = fixture_report();
        let json = report.to_json_pretty().expect("report serializes");
        assert!(json.contains("\"protocol_version\": 1"));
        assert!(json.contains("\"class_ids\": []"));
        assert!(json.contains("\"ready_for_enumeration\": 1"));
        let parsed: ScanReport = serde_json::from_str(&json).expect("report deserializes");
        assert_eq!(parsed, report);
    }

    #[test]
    fn text_report_does_not_claim_enumeration() {
        let text = fixture_report().to_text();
        assert!(text.contains("Class enumeration is not implemented"));
        assert!(text.contains("READY"));
        assert!(text.contains("REJECT"));
        assert!(text.contains("no plug-in host is claimed"));
    }

    #[test]
    fn missing_folder_is_reported_without_aborting() {
        let temp = TempDir::new("report-missing");
        let report = build_report(&[temp.path().join("absent")], DiscoveryLimits::default(), 1);
        assert_eq!(report.folders.len(), 1);
        assert_eq!(report.folders[0].status, FolderStatus::Missing);
        assert_eq!(report.totals.modules, 0);
    }
}
