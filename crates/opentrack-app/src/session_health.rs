//! One place that answers "is anything wrong with this session?".
//!
//! The checks are the ones OpenTrack already performs elsewhere — save
//! validation, routing validation, missing media, failed plug-in loads, the
//! project's own limits — collected so a user does not have to discover them one
//! dialog at a time. Reporting is separate from acting: each issue says what to do
//! and the existing command does it.

use std::path::Path;

use opentrack_core::project::Project;

/// How much an issue matters right now.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum HealthSeverity {
    /// Worth knowing, but the project is fine.
    Info,
    /// The project will lose or misplace something if left alone.
    Warning,
    /// The project cannot be saved or played correctly.
    Error,
}

impl HealthSeverity {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }
}

/// One finding of the health check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct HealthIssue {
    pub severity: HealthSeverity,
    pub summary: String,
    pub detail: String,
}

impl HealthIssue {
    fn new(
        severity: HealthSeverity,
        summary: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            summary: summary.into(),
            detail: detail.into(),
        }
    }
}

/// Everything the health check looks at, gathered by the caller.
pub(super) struct HealthInput<'a> {
    pub project: &'a Project,
    /// Insert failures reported by the plug-in host, `(channel, slot, reason)`.
    pub insert_failures: &'a [(String, String)],
    /// Size of the snapshot the undo history holds for this project, in bytes.
    pub snapshot_bytes: usize,
    pub snapshot_limit: usize,
    /// Free-form facts the caller wants shown without judging them.
    pub notes: Vec<(String, String)>,
}

/// The findings, most severe first.
pub(super) fn review(input: HealthInput<'_>) -> Vec<HealthIssue> {
    let mut issues = Vec::new();
    let project = input.project;

    if let Err(message) = project.validate() {
        issues.push(HealthIssue::new(
            HealthSeverity::Error,
            "The project cannot be saved or played as it stands",
            message,
        ));
    }
    if let Err(error) = project.validate_routing() {
        issues.push(HealthIssue::new(
            HealthSeverity::Error,
            "Channel routing is not playable",
            format!("{error}. Re-route the channel in Channel Settings."),
        ));
    }
    let missing = project.missing_media();
    if !missing.is_empty() {
        let names = missing
            .iter()
            .take(3)
            .map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string())
            })
            .collect::<Vec<_>>()
            .join(", ");
        issues.push(HealthIssue::new(
            HealthSeverity::Warning,
            format!("{} source file(s) are missing", missing.len()),
            format!("{names}. Project > Relink Missing Media finds them in a folder."),
        ));
    }
    if !input.insert_failures.is_empty() {
        let names = input
            .insert_failures
            .iter()
            .take(3)
            .map(|(channel, slot)| format!("{channel} slot {slot}"))
            .collect::<Vec<_>>()
            .join(", ");
        let first_reason = input
            .insert_failures
            .first()
            .map(|(_, reason)| reason.as_str())
            .unwrap_or_default();
        issues.push(HealthIssue::new(
            HealthSeverity::Warning,
            format!(
                "{} VST3 insert(s) did not load",
                input.insert_failures.len()
            ),
            format!("{names}: {first_reason}. Click the red ! in Channel Settings to retry."),
        ));
    }
    if !insert_failures_belong_to_project(input.insert_failures, project) {
        issues.push(HealthIssue::new(
            HealthSeverity::Info,
            "A failed insert belongs to a channel that is no longer in the project",
            "It can be ignored; the next plan rebuild drops it.".to_owned(),
        ));
    }
    if input.snapshot_limit > 0 && input.snapshot_bytes > input.snapshot_limit / 2 {
        let percent = input.snapshot_bytes * 100 / input.snapshot_limit.max(1);
        issues.push(HealthIssue::new(
            HealthSeverity::Warning,
            format!("The undo snapshot uses {percent}% of its limit"),
            "Very large projects stop recording undo steps. Save, then split the project if this grows.".to_owned(),
        ));
    }

    let clips = project
        .tracks
        .iter()
        .map(|track| track.clips.len())
        .sum::<usize>();
    let parts = project
        .tracks
        .iter()
        .map(|track| track.midi_parts.len())
        .sum::<usize>();
    issues.push(HealthIssue::new(
        HealthSeverity::Info,
        format!(
            "{} track(s), {clips} audio event(s), {parts} MIDI part(s), {} marker(s)",
            project.tracks.len(),
            project.markers.len()
        ),
        format!(
            "{} Hz, {} BPM, {} beats per bar",
            project.sample_rate_hz, project.tempo_bpm, project.beats_per_bar
        ),
    ));
    for (summary, detail) in input.notes {
        issues.push(HealthIssue::new(HealthSeverity::Info, summary, detail));
    }

    issues.sort_by(|left, right| right.severity.cmp(&left.severity));
    issues
}

/// True when every failed insert still points at a channel in the project.
fn insert_failures_belong_to_project(failures: &[(String, String)], project: &Project) -> bool {
    let names = project
        .channels()
        .map(|channel| channel.name.as_str())
        .collect::<Vec<_>>();
    failures
        .iter()
        .all(|(channel, _)| names.iter().any(|name| *name == channel))
}

/// A short sentence for the status line.
pub(super) fn summary(issues: &[HealthIssue]) -> String {
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == HealthSeverity::Error)
        .count();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == HealthSeverity::Warning)
        .count();
    match (errors, warnings) {
        (0, 0) => "Session health: nothing to fix".to_owned(),
        (0, warnings) => format!("Session health: {warnings} warning(s)"),
        (errors, 0) => format!("Session health: {errors} error(s)"),
        (errors, warnings) => format!("Session health: {errors} error(s), {warnings} warning(s)"),
    }
}

/// True when a path is a managed project folder (used by the caller's notes).
pub(super) fn is_managed(root: &Path) -> bool {
    root.join("Audio").join("Recordings").is_dir() && root.join("Metadata").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::project::TrackKind;

    fn clean_project() -> Project {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project
    }

    fn input<'a>(
        project: &'a Project,
        failures: &'a [(String, String)],
        snapshot_bytes: usize,
    ) -> HealthInput<'a> {
        HealthInput {
            project,
            insert_failures: failures,
            snapshot_bytes,
            snapshot_limit: 64 * 1024 * 1024,
            notes: Vec::new(),
        }
    }

    #[test]
    fn a_clean_session_reports_only_the_summary() {
        let project = clean_project();
        let issues = review(input(&project, &[], 1024));
        assert_eq!(issues.len(), 1, "only the info summary: {issues:?}");
        assert_eq!(issues[0].severity, HealthSeverity::Info);
        assert!(issues[0].summary.contains("1 track(s), 0 audio event(s)"));
        assert_eq!(summary(&issues), "Session health: nothing to fix");
    }

    #[test]
    fn missing_media_and_an_unplayable_route_are_reported_with_what_to_do() {
        let mut project = clean_project();
        project
            .add_audio_clip(0, std::path::PathBuf::from("/nowhere/kick.wav"), 0.0, 2.0)
            .expect("clip");
        project.tracks[0].output_routing = Some("Surround 5.1".to_owned());

        let issues = review(input(&project, &[], 1024));
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == HealthSeverity::Error
                    && issue.summary.contains("routing"))
        );
        let media = issues
            .iter()
            .find(|issue| issue.summary.contains("missing"))
            .expect("missing media is reported");
        assert_eq!(media.severity, HealthSeverity::Warning);
        assert!(media.detail.contains("Relink Missing Media"));
        assert_eq!(summary(&issues), "Session health: 1 error(s), 1 warning(s)");
    }

    #[test]
    fn failed_inserts_and_a_large_snapshot_are_warnings() {
        let project = clean_project();
        let failures = vec![("Audio 01".to_owned(), "module load failed".to_owned())];
        let issues = review(input(&project, &failures, 40 * 1024 * 1024));
        assert!(
            issues
                .iter()
                .any(|issue| issue.summary.contains("1 VST3 insert(s) did not load"))
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.summary.contains("undo snapshot uses"))
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue.summary.contains("no longer in the project"))
        );
    }

    #[test]
    fn a_stale_insert_failure_is_explained_as_harmless() {
        let project = clean_project();
        let failures = vec![("Deleted Channel".to_owned(), "gone".to_owned())];
        let issues = review(input(&project, &failures, 1024));
        assert!(
            issues
                .iter()
                .any(|issue| issue.summary.contains("no longer in the project"))
        );
    }

    #[test]
    fn issues_are_ordered_by_severity() {
        let mut project = clean_project();
        project
            .add_audio_clip(0, std::path::PathBuf::from("/nowhere/a.wav"), 0.0, 1.0)
            .expect("clip");
        project.tracks[0].output_routing = Some("Surround 5.1".to_owned());
        let issues = review(input(&project, &[], 1024));
        assert!(
            issues
                .windows(2)
                .all(|pair| pair[0].severity >= pair[1].severity),
            "errors come first: {issues:?}"
        );
        assert_eq!(issues[0].severity, HealthSeverity::Error);
    }
}
