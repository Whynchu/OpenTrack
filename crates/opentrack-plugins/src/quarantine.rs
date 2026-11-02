// SPDX-License-Identifier: Apache-2.0

//! Modules the scanner refuses to load again until the user retries.
//!
//! A module that timed out or killed its worker has demonstrated that loading it
//! can stall or crash the process. Re-trying it on every scan costs the user that
//! failure again for no gain, so the record is kept with the plug-in catalog and
//! consulted before the next scan. The record is keyed by module path: a module
//! that was never enumerated has no class IDs to key on.
//!
//! Quarantine is advisory state, not a security boundary. A user who updates or
//! reinstalls a plug-in can retry explicitly, and a successful enumeration drops
//! the record.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::PluginScanStatus;

/// Maximum characters retained in a quarantine reason.
pub const MAX_QUARANTINE_REASON_CHARS: usize = 512;

/// Why a module was quarantined.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineReason {
    /// The module exceeded its enumeration budget.
    TimedOut,
    /// The module's worker process died.
    Crashed,
}

impl QuarantineReason {
    /// Maps a rejected scan status to a quarantine reason, if it is one.
    pub const fn from_status(status: PluginScanStatus) -> Option<Self> {
        match status {
            PluginScanStatus::TimedOut => Some(Self::TimedOut),
            PluginScanStatus::Crashed => Some(Self::Crashed),
            _ => None,
        }
    }

    /// Status the scanner reports for a module skipped because of this record.
    pub const fn status(self) -> PluginScanStatus {
        match self {
            Self::TimedOut => PluginScanStatus::TimedOut,
            Self::Crashed => PluginScanStatus::Crashed,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::TimedOut => "timed out",
            Self::Crashed => "crashed",
        }
    }
}

/// One module that will not be loaded until the user retries it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QuarantineEntry {
    /// Module file that failed.
    pub module_path: PathBuf,
    pub reason: QuarantineReason,
    /// The scanner's diagnostic at the time of failure.
    pub detail: String,
    /// When the record was written, in Unix milliseconds.
    #[serde(default)]
    pub quarantined_at_unix_ms: u64,
}

impl QuarantineEntry {
    pub fn new(
        module_path: PathBuf,
        reason: QuarantineReason,
        detail: impl Into<String>,
        quarantined_at_unix_ms: u64,
    ) -> Self {
        Self {
            module_path,
            reason,
            detail: bounded_reason(detail.into()),
            quarantined_at_unix_ms,
        }
    }

    /// True when this record describes the same module file.
    pub fn matches(&self, path: &Path) -> bool {
        paths_match(&self.module_path, path)
    }

    /// Text shown in the plug-in manager and recorded in the next scan report.
    pub fn skip_reason(&self) -> String {
        format!(
            "skipped: quarantined after it {} on an earlier scan ({})",
            self.reason.label(),
            self.detail
        )
    }
}

/// Keeps at most [`MAX_QUARANTINE_ENTRIES`] records, oldest dropped first.
pub const MAX_QUARANTINE_ENTRIES: usize = 256;

/// Bounds, de-duplicates and sorts quarantined modules for stable persistence.
///
/// The newest record for a module wins, so an updated failure reason replaces the
/// previous one instead of accumulating.
pub fn normalize_quarantine(
    entries: impl IntoIterator<Item = QuarantineEntry>,
) -> Vec<QuarantineEntry> {
    let mut normalized: Vec<QuarantineEntry> = Vec::new();
    for entry in entries {
        let entry = QuarantineEntry {
            detail: bounded_reason(entry.detail),
            ..entry
        };
        match normalized
            .iter_mut()
            .find(|existing| existing.matches(&entry.module_path))
        {
            Some(existing) => {
                if entry.quarantined_at_unix_ms >= existing.quarantined_at_unix_ms {
                    *existing = entry;
                }
            }
            None => normalized.push(entry),
        }
    }
    normalized.sort_by(|left, right| {
        left.module_path
            .to_string_lossy()
            .to_lowercase()
            .cmp(&right.module_path.to_string_lossy().to_lowercase())
    });
    if normalized.len() > MAX_QUARANTINE_ENTRIES {
        normalized.drain(..normalized.len() - MAX_QUARANTINE_ENTRIES);
    }
    normalized
}

/// Removes the record for one module, returning whether it was present.
pub fn release(entries: &mut Vec<QuarantineEntry>, module_path: &Path) -> bool {
    let before = entries.len();
    entries.retain(|entry| !entry.matches(module_path));
    entries.len() != before
}

/// Case-insensitive path comparison, matching how the catalog treats paths.
pub fn paths_match(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(right.to_string_lossy().trim_end_matches(['\\', '/']))
}

fn bounded_reason(text: String) -> String {
    if text.chars().count() <= MAX_QUARANTINE_REASON_CHARS {
        return text;
    }
    let mut truncated = text
        .chars()
        .take(MAX_QUARANTINE_REASON_CHARS.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, reason: QuarantineReason, at: u64) -> QuarantineEntry {
        QuarantineEntry::new(PathBuf::from(path), reason, "worker died", at)
    }

    #[test]
    fn only_timeouts_and_crashes_are_quarantined() {
        assert_eq!(
            QuarantineReason::from_status(PluginScanStatus::TimedOut),
            Some(QuarantineReason::TimedOut)
        );
        assert_eq!(
            QuarantineReason::from_status(PluginScanStatus::Crashed),
            Some(QuarantineReason::Crashed)
        );
        for status in [
            PluginScanStatus::Enumerated,
            PluginScanStatus::LoadFailed,
            PluginScanStatus::InvalidModule,
            PluginScanStatus::UnsupportedArchitecture,
            PluginScanStatus::Unreadable,
            PluginScanStatus::ReadyForEnumeration,
            PluginScanStatus::NotScanned,
            PluginScanStatus::Quarantined,
            PluginScanStatus::NeedsAuthorization,
        ] {
            assert_eq!(QuarantineReason::from_status(status), None, "{status:?}");
        }
        assert_eq!(
            QuarantineReason::TimedOut.status(),
            PluginScanStatus::TimedOut
        );
        assert_eq!(
            QuarantineReason::Crashed.status(),
            PluginScanStatus::Crashed
        );
    }

    #[test]
    fn records_deduplicate_by_path_keeping_the_newest() {
        let normalized = normalize_quarantine([
            entry(r"C:\VST3\Hang.vst3", QuarantineReason::TimedOut, 10),
            entry(r"C:\VST3\Dead.vst3", QuarantineReason::Crashed, 5),
            QuarantineEntry {
                detail: "worker was terminated by signal 11".to_owned(),
                quarantined_at_unix_ms: 20,
                ..entry(r"C:\VST3\Dead.vst3", QuarantineReason::Crashed, 0)
            },
        ]);
        assert_eq!(normalized.len(), 2, "one record per module");
        let dead = normalized
            .iter()
            .find(|entry| {
                entry
                    .module_path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("dead.vst3")
            })
            .expect("record kept");
        assert_eq!(dead.quarantined_at_unix_ms, 20);
        assert!(dead.detail.contains("signal 11"));
        assert_eq!(
            normalized[0].module_path.to_string_lossy().to_lowercase(),
            r"c:\vst3\dead.vst3",
            "records are sorted for stable persistence"
        );
    }

    #[test]
    fn an_older_record_never_replaces_a_newer_one() {
        let normalized = normalize_quarantine([
            QuarantineEntry {
                detail: "newest".to_owned(),
                quarantined_at_unix_ms: 99,
                ..entry("/vst3/a.vst3", QuarantineReason::Crashed, 0)
            },
            entry("/vst3/a.vst3", QuarantineReason::TimedOut, 1),
        ]);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].detail, "newest");
        assert_eq!(normalized[0].reason, QuarantineReason::Crashed);
    }

    #[test]
    fn paths_match_case_insensitively_and_ignore_trailing_separators() {
        assert!(paths_match(
            Path::new(r"C:\VST3\Thing.vst3"),
            Path::new(r"c:\vst3\thing.VST3")
        ));
        assert!(paths_match(
            Path::new("/vst3/Thing.vst3/"),
            Path::new("/vst3/Thing.vst3")
        ));
        assert!(!paths_match(
            Path::new("/vst3/Thing.vst3"),
            Path::new("/vst3/Other.vst3")
        ));
    }

    #[test]
    fn releasing_one_module_leaves_the_rest_and_bounds_the_reason() {
        let mut entries = normalize_quarantine([
            entry("/vst3/a.vst3", QuarantineReason::Crashed, 1),
            entry("/vst3/b.vst3", QuarantineReason::TimedOut, 2),
        ]);
        assert!(release(&mut entries, Path::new("/VST3/A.vst3")));
        assert_eq!(entries.len(), 1);
        assert!(!release(&mut entries, Path::new("/vst3/a.vst3")));
        assert_eq!(entries[0].module_path, PathBuf::from("/vst3/b.vst3"));

        let long = QuarantineEntry::new(
            PathBuf::from("/vst3/c.vst3"),
            QuarantineReason::Crashed,
            "x".repeat(MAX_QUARANTINE_REASON_CHARS * 2),
            0,
        );
        assert_eq!(long.detail.chars().count(), MAX_QUARANTINE_REASON_CHARS);
        assert!(long.detail.ends_with("..."));
    }

    #[test]
    fn the_skip_reason_names_the_failure_and_its_cause() {
        let entry = QuarantineEntry::new(
            PathBuf::from("/vst3/Hang.vst3"),
            QuarantineReason::TimedOut,
            "module did not finish enumerating within 30 s",
            0,
        );
        let reason = entry.skip_reason();
        assert!(reason.contains("timed out"), "{reason}");
        assert!(
            reason.contains("30 s"),
            "the original detail is kept: {reason}"
        );
    }
}
