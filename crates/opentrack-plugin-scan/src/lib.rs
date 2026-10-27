// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Bounded VST3 module discovery, precheck, and factory enumeration helper.
//!
//! This crate implements configured folder discovery, native bundle/single-file
//! module selection, executable architecture prechecks, path duplication
//! control, optional factory class enumeration, and a bounded JSON/text report.
//!
//! Without `--enumerate`, no module is loaded and host-compatible modules remain
//! [`opentrack_plugins::PluginScanStatus::ReadyForEnumeration`]. With it, the
//! scanner process loads each module and records factory class metadata through
//! `opentrack-vst3`. Per-module timeout/crash attribution, quarantine, the job
//! object, instance introspection, and audio hosting remain future work.
//!
//! Discovery keeps [`std::path::PathBuf`] values internally. Report text uses
//! a display conversion; a path that is not valid UTF-8 is marked in the report
//! and produces a diagnostic instead of being silently treated as equivalent.

pub mod discovery;
pub mod image;
pub mod pe;
pub mod report;

#[cfg(test)]
mod test_support;

use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub use discovery::{
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_MODULES, DiscoveredModule, DiscoveryBatch, DiscoveryLimits,
    FolderDiscovery, FolderOutcome, MAX_DEPTH_LIMIT, MAX_MODULES_LIMIT, ModuleKind, discover_all,
    discover_folder, resolve_vst3_bundle,
};
pub use image::{ModuleImageError, ModuleImageFormat, ModuleImageInfo, read_module_image};
pub use pe::{
    PeFileError, PeHeaderError, PeMachine, classify_pe_machine, parse_pe_machine,
    read_pe_machine_from_file,
};
pub use report::{
    FolderReport, FolderStatus, MAX_REPORT_FOLDERS, ModuleReport, ScanReport, ScanTotals,
    build_report, scan_folders, scan_folders_enumerating,
};

/// Version of the scanner report protocol consumed by the application.
///
/// A future app/scanner handshake must reject a mismatch rather than interpreting
/// report fields with different semantics.
pub const SCAN_PROTOCOL_VERSION: u32 = 2;

/// Scanner build version, copied from the crate package version.
pub const SCANNER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns this platform's standard user and system VST3 folders.
///
/// Folders are returned even when they do not exist; discovery reports the
/// missing folder without aborting the scan. Reference-install folders under
/// `G:\dev\Opentrack` are intentionally never included.
pub fn default_vst3_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    #[cfg(windows)]
    {
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            folders.push(
                PathBuf::from(program_files)
                    .join("Common Files")
                    .join("VST3"),
            );
        } else if let Some(program_files) = std::env::var_os("ProgramW6432") {
            folders.push(
                PathBuf::from(program_files)
                    .join("Common Files")
                    .join("VST3"),
            );
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            folders.push(
                PathBuf::from(local_app_data)
                    .join("Programs")
                    .join("Common")
                    .join("VST3"),
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            folders.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Audio")
                    .join("Plug-ins")
                    .join("VST3"),
            );
        }
        folders.push(PathBuf::from("/Library/Audio/Plug-ins/VST3"));
        folders.push(PathBuf::from("/Network/Library/Audio/Plug-ins/VST3"));
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            folders.push(PathBuf::from(home).join(".vst3"));
        }
        folders.extend([
            PathBuf::from("/usr/lib64/vst3"),
            PathBuf::from("/usr/lib/vst3"),
            PathBuf::from("/usr/local/lib64/vst3"),
            PathBuf::from("/usr/local/lib/vst3"),
        ]);
    }
    folders
}

/// Current Unix time in milliseconds, or zero if the system clock is before
/// the Unix epoch.
pub fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Maximum characters retained in a report diagnostic.
pub(crate) const MAX_DIAGNOSTIC_CHARS: usize = 512;

/// Truncates scanner-controlled text to a bounded, character-safe diagnostic.
pub(crate) fn bounded_diagnostic(text: impl Into<String>) -> String {
    let text = text.into();
    if text.chars().count() <= MAX_DIAGNOSTIC_CHARS {
        return text;
    }
    let mut truncated = text
        .chars()
        .take(MAX_DIAGNOSTIC_CHARS.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}
