// SPDX-License-Identifier: Apache-2.0

//! Bounded scan report and JSON/text rendering.
//!
//! The report is the scanner's application-facing output. Discovery and PE
//! precheck always run. When enumeration is requested, the scanner loads each
//! x64 module in its own process and records factory class metadata; buses,
//! parameters, editors and latency remain instance-introspection follow-ups.
//!
//! Counts, warnings, and diagnostic text are bounded. Paths remain wide
//! `PathBuf` values during discovery; report strings are display copies, and a
//! non-UTF-8 path is explicitly flagged.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use opentrack_plugins::PluginArchitecture;
use opentrack_plugins::PluginClassId;
use opentrack_plugins::PluginDescriptor;
use opentrack_plugins::PluginScanStatus;
use opentrack_vst3::{Vst3Module, class_descriptor};
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
    /// Discovery/precheck/enumeration status.
    pub status: PluginScanStatus,
    /// File size in bytes when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Audio module class IDs retained from factory enumeration.
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
    /// Modules whose factory was loaded and class metadata read.
    #[serde(default)]
    pub enumerated_modules: usize,
    /// Unique plug-in classes retained after class-ID deduplication.
    #[serde(default)]
    pub plugins: usize,
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
    /// Enumerated classes, deduplicated by class ID.
    #[serde(default)]
    pub plugins: Vec<PluginDescriptor>,
    /// Fixed explanatory notes.
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
            "modules: {} reported, {} ready, {} enumerated, {} rejected, {} duplicate paths skipped",
            self.totals.modules,
            self.totals.ready_for_enumeration,
            self.totals.enumerated_modules,
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

        let _ = writeln!(output, "plugins: {} unique classes", self.plugins.len());
        for plugin in &self.plugins {
            let _ = writeln!(
                output,
                "  {:?} {} | {} | {} | {}",
                plugin.category,
                one_line(&plugin.name),
                one_line(&plugin.vendor),
                plugin.class_id,
                one_line(&plugin.module_path.to_string_lossy())
            );
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
    enumerate: bool,
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

    let mut module_reports = batch.modules.iter().map(inspect_module).collect::<Vec<_>>();
    let mut plugins = Vec::new();
    if enumerate {
        let mut retained = std::collections::HashMap::<PluginClassId, PathBuf>::new();
        for module in &mut module_reports {
            if module.status != PluginScanStatus::ReadyForEnumeration
                || module.architecture != PluginArchitecture::X64
            {
                continue;
            }
            if !module.path_utf8 {
                append_diagnostic(
                    module,
                    "class enumeration skipped because the module path is not valid UTF-8"
                        .to_owned(),
                );
                continue;
            }
            let path = PathBuf::from(&module.path);
            let loaded = match Vst3Module::open(&path) {
                Ok(loaded) => loaded,
                Err(error) => {
                    module.status = PluginScanStatus::LoadFailed;
                    append_diagnostic(module, error.to_string());
                    continue;
                }
            };
            let enumeration = match loaded.enumerate() {
                Ok(enumeration) => enumeration,
                Err(error) => {
                    module.status = PluginScanStatus::LoadFailed;
                    append_diagnostic(module, error.to_string());
                    continue;
                }
            };

            for diagnostic in enumeration.diagnostics {
                append_diagnostic(module, diagnostic);
            }
            if enumeration.classes.is_empty() {
                append_diagnostic(
                    module,
                    format!(
                        "factory loaded (vendor '{}') but exposed no audio module classes",
                        enumeration.factory_vendor
                    ),
                );
                continue;
            }
            let mut class_ids = Vec::new();
            for class in enumeration.classes {
                if let Some(existing) = retained.get(&class.class_id) {
                    append_diagnostic(
                        module,
                        format!(
                            "duplicate class {} ('{}') already enumerated from {}",
                            class.class_id,
                            class.name,
                            existing.display()
                        ),
                    );
                    continue;
                }
                retained.insert(class.class_id, path.clone());
                class_ids.push(class.class_id);
                plugins.push(descriptor_with_class_subcategories(
                    &path,
                    module.architecture,
                    &class,
                    generated_at_unix_ms,
                ));
            }
            module.class_ids = class_ids;
            module.status = PluginScanStatus::Enumerated;
        }
    }

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

    let enumerated_modules = module_reports
        .iter()
        .filter(|module| module.status == PluginScanStatus::Enumerated)
        .count();
    let mut notes = if enumerate {
        vec![
            "Factory class metadata was enumerated in this scanner process. Buses, parameters, editors, latency and audio hosting require instance introspection and are not implemented yet.".to_owned(),
        ]
    } else {
        vec![
            "Class enumeration was not requested. No VST3 module was loaded; ready entries are PE precheck results only.".to_owned(),
        ]
    };
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
            enumerated_modules,
            plugins: plugins.len(),
            duplicates_skipped: batch.duplicates_skipped,
            truncated: batch.truncated || configured_folders > reported_folders.len(),
        },
        folders: folder_reports,
        modules: module_reports,
        plugins,
        notes,
    }
}

/// Discovers folders and builds a report with the current time.
pub fn scan_folders(folders: &[PathBuf], limits: DiscoveryLimits) -> ScanReport {
    build_report(folders, limits, crate::unix_time_ms(), false)
}

/// Discovers folders and performs factory class enumeration in this process.
pub fn scan_folders_enumerating(folders: &[PathBuf], limits: DiscoveryLimits) -> ScanReport {
    build_report(folders, limits, crate::unix_time_ms(), true)
}

/// Builds a descriptor and copies the VST3 class subcategory text into the
/// format-neutral catalog field. Keeping this in the scanner means
/// `opentrack-plugins` stays independent of the VST3 ABI.
fn descriptor_with_class_subcategories(
    module_path: &Path,
    architecture: PluginArchitecture,
    class: &opentrack_vst3::Vst3ClassInfo,
    scanned_at_unix_ms: u64,
) -> PluginDescriptor {
    let mut descriptor = class_descriptor(module_path, architecture, class, scanned_at_unix_ms);
    descriptor.subcategories = class
        .subcategories
        .split('|')
        .map(str::trim)
        .filter(|subcategory| !subcategory.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    descriptor
}

fn append_diagnostic(module: &mut ModuleReport, diagnostic: String) {
    let mut parts = module.diagnostics.take().into_iter().collect::<Vec<_>>();
    parts.push(diagnostic);
    module.diagnostics = join_diagnostics(parts);
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
            false,
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
        assert_eq!(report.totals.enumerated_modules, 0);
        assert_eq!(report.totals.plugins, 0);
        assert!(report.plugins.is_empty());
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
        assert!(json.contains("\"protocol_version\": 2"));
        assert!(json.contains("\"class_ids\": []"));
        assert!(json.contains("\"ready_for_enumeration\": 1"));
        let parsed: ScanReport = serde_json::from_str(&json).expect("report deserializes");
        assert_eq!(parsed, report);
    }

    #[test]
    fn class_subcategories_are_split_and_trimmed_for_the_catalog() {
        let class = opentrack_vst3::Vst3ClassInfo {
            class_id: PluginClassId::ZERO,
            name: "Example Compressor".to_owned(),
            vendor: "Example Audio".to_owned(),
            version: "1.0".to_owned(),
            category: "Audio Module Class".to_owned(),
            subcategories: "Fx| Dynamics | Compressor|".to_owned(),
            cardinality: 0,
            class_flags: 0,
        };
        let descriptor = descriptor_with_class_subcategories(
            Path::new("Example.vst3"),
            PluginArchitecture::X64,
            &class,
            7,
        );
        assert_eq!(
            descriptor.subcategories,
            vec![
                "Fx".to_owned(),
                "Dynamics".to_owned(),
                "Compressor".to_owned()
            ]
        );
        assert_eq!(descriptor.scanned_at_unix_ms, 7);
    }

    #[test]
    fn text_report_does_not_claim_enumeration_without_the_flag() {
        let text = fixture_report().to_text();
        assert!(text.contains("Class enumeration was not requested"));
        assert!(text.contains("No VST3 module was loaded"));
        assert!(text.contains("READY"));
        assert!(text.contains("REJECT"));
        assert!(!text.contains("ENUMERATED"));
        assert!(!text.contains("audio hosting is implemented"));
    }

    #[test]
    fn failed_enumeration_is_reported_without_losing_the_precheck() {
        let temp = TempDir::new("report-enumeration-failure");
        temp.write(r"Good.vst3", &pe_image(0x8664));
        let report = build_report(
            &[temp.path().to_path_buf()],
            DiscoveryLimits::default(),
            1_700_000_000_000,
            true,
        );
        assert_eq!(report.totals.modules, 1);
        assert_eq!(report.totals.enumerated_modules, 0);
        assert_eq!(report.totals.plugins, 0);
        assert_eq!(report.modules[0].status, PluginScanStatus::LoadFailed);
        assert!(
            report.modules[0]
                .diagnostics
                .as_deref()
                .is_some_and(|text| text.contains("could not open VST3 module"))
        );
    }

    #[test]
    fn missing_folder_is_reported_without_aborting() {
        let temp = TempDir::new("report-missing");
        let report = build_report(
            &[temp.path().join("absent")],
            DiscoveryLimits::default(),
            1,
            false,
        );
        assert_eq!(report.folders.len(), 1);
        assert_eq!(report.folders[0].status, FolderStatus::Missing);
        assert_eq!(report.totals.modules, 0);
    }
}
