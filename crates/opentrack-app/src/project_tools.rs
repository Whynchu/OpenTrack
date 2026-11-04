//! Project statistics and the beat calculator.
//!
//! Both are read-only views over the open project: nothing here edits, saves or
//! allocates audio. Keeping the arithmetic in small functions makes the dialogs
//! testable without a window.

use opentrack_core::project::{Project, TrackKind};

/// Counts that describe the open project.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ProjectStatistics {
    pub audio_tracks: usize,
    pub midi_tracks: usize,
    pub instrument_tracks: usize,
    pub group_tracks: usize,
    pub fx_tracks: usize,
    pub folder_tracks: usize,
    pub audio_events: usize,
    pub midi_parts: usize,
    pub midi_notes: usize,
    pub midi_controllers: usize,
    pub markers: usize,
    pub media_entries: usize,
    pub missing_media: usize,
    pub automation_points: usize,
    pub inserts: usize,
    pub sends: usize,
}

pub(super) fn project_statistics(project: &Project) -> ProjectStatistics {
    let mut statistics = ProjectStatistics {
        markers: project.markers.len(),
        media_entries: project.media_pool.len(),
        missing_media: project.missing_media().len(),
        ..ProjectStatistics::default()
    };
    for track in &project.tracks {
        match track.kind {
            TrackKind::Audio => statistics.audio_tracks += 1,
            TrackKind::Midi => statistics.midi_tracks += 1,
            TrackKind::Instrument => statistics.instrument_tracks += 1,
            TrackKind::Group => statistics.group_tracks += 1,
            TrackKind::Fx => statistics.fx_tracks += 1,
            TrackKind::InstrumentOutput => statistics.fx_tracks += 1,
            TrackKind::Folder => statistics.folder_tracks += 1,
        }
        statistics.audio_events += track.clips.len();
        statistics.midi_parts += track.midi_parts.len();
        statistics.inserts += track.inserts.len();
        statistics.sends += track.send_parameters.len();
        for part in &track.midi_parts {
            statistics.midi_notes += part.notes.len();
            statistics.midi_controllers += part.controller_events.len();
        }
        for lane in &track.automation_lanes {
            statistics.automation_points += lane.points.len();
        }
    }
    statistics
}

impl ProjectStatistics {
    /// Rows for the dialog, in display order.
    pub(super) fn rows(&self) -> Vec<(&'static str, usize)> {
        vec![
            ("Audio tracks", self.audio_tracks),
            ("MIDI tracks", self.midi_tracks),
            ("Instrument tracks", self.instrument_tracks),
            ("Group tracks", self.group_tracks),
            ("FX channels", self.fx_tracks),
            ("Folders", self.folder_tracks),
            ("Audio events", self.audio_events),
            ("MIDI parts", self.midi_parts),
            ("MIDI notes", self.midi_notes),
            ("MIDI controller events", self.midi_controllers),
            ("Markers", self.markers),
            ("Media pool entries", self.media_entries),
            ("Missing media", self.missing_media),
            ("Automation points", self.automation_points),
            ("Insert effects", self.inserts),
            ("Sends", self.sends),
        ]
    }
}

/// Which field the user edited last, so the other one can follow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BeatCalculatorInput {
    Beats,
    Seconds,
}

/// The beat calculator's dialog state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BeatCalculator {
    pub beats: f64,
    pub seconds: f64,
    pub last_edited: BeatCalculatorInput,
}

impl BeatCalculator {
    pub(super) fn from_locators(project: &Project) -> Self {
        let beats = (project.loop_end_beats - project.loop_start_beats).max(0.0);
        Self {
            beats,
            seconds: beats_to_seconds(beats, project.tempo_bpm),
            last_edited: BeatCalculatorInput::Beats,
        }
    }

    /// Recomputes the field the user did not edit.
    pub(super) fn refresh(&mut self, tempo_bpm: f64) {
        match self.last_edited {
            BeatCalculatorInput::Beats => {
                self.seconds = beats_to_seconds(self.beats, tempo_bpm);
            }
            BeatCalculatorInput::Seconds => {
                self.beats = seconds_to_beats(self.seconds, tempo_bpm);
            }
        }
    }
}

/// Seconds for a musical length at `tempo_bpm`.
pub(super) fn beats_to_seconds(beats: f64, tempo_bpm: f64) -> f64 {
    if !beats.is_finite() || !tempo_bpm.is_finite() || tempo_bpm <= 0.0 {
        return 0.0;
    }
    beats * 60.0 / tempo_bpm
}

/// Musical length for a duration at `tempo_bpm`.
pub(super) fn seconds_to_beats(seconds: f64, tempo_bpm: f64) -> f64 {
    if !seconds.is_finite() || !tempo_bpm.is_finite() || tempo_bpm <= 0.0 {
        return 0.0;
    }
    seconds * tempo_bpm / 60.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::project::MidiNote;

    fn project_with_content() -> Project {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Midi);
        project.add_track(TrackKind::Instrument);
        project.add_track(TrackKind::Fx);
        project.add_track(TrackKind::Folder);
        project
            .add_audio_clip(0, std::path::PathBuf::from("/nowhere/a.wav"), 0.0, 2.0)
            .expect("clip");
        project
            .add_audio_clip(0, std::path::PathBuf::from("/nowhere/b.wav"), 4.0, 2.0)
            .expect("clip");
        project.add_marker(8.0, "Chorus").expect("marker");
        let part = project.add_midi_part(2, 0.0, 8.0).expect("part");
        project
            .add_midi_note_on_channel(2, part, 1.0, 1.0, 60, 0, 100)
            .expect("note");
        project
    }

    #[test]
    fn statistics_count_what_the_project_holds() {
        let statistics = project_statistics(&project_with_content());
        assert_eq!(statistics.audio_tracks, 1);
        assert_eq!(statistics.group_tracks, 1);
        assert_eq!(statistics.midi_tracks, 1);
        assert_eq!(statistics.instrument_tracks, 1);
        assert_eq!(statistics.fx_tracks, 1);
        assert_eq!(statistics.folder_tracks, 1);
        assert_eq!(statistics.audio_events, 2);
        assert_eq!(statistics.midi_parts, 1);
        assert_eq!(statistics.midi_notes, 1);
        assert_eq!(statistics.markers, 1);
        assert_eq!(
            statistics.missing_media, 2,
            "both sources are absent on this host"
        );
        assert_eq!(statistics.rows().len(), 16);
        assert_eq!(statistics.rows()[6], ("Audio events", 2));
    }

    #[test]
    fn the_beat_calculator_follows_the_edited_field_and_the_tempo() {
        let project = project_with_content();
        let mut calculator = BeatCalculator::from_locators(&project);
        // The default locators span four beats at 120 BPM, which is two seconds.
        assert!(calculator.beats > 0.0);
        assert!((calculator.seconds - beats_to_seconds(calculator.beats, 120.0)).abs() < 1e-9);

        calculator.beats = 8.0;
        calculator.last_edited = BeatCalculatorInput::Beats;
        calculator.refresh(120.0);
        assert!((calculator.seconds - 4.0).abs() < 1e-9);

        calculator.seconds = 6.0;
        calculator.last_edited = BeatCalculatorInput::Seconds;
        calculator.refresh(90.0);
        assert!((calculator.beats - 9.0).abs() < 1e-9);

        // A broken tempo reports zero rather than an infinity.
        calculator.last_edited = BeatCalculatorInput::Beats;
        calculator.refresh(0.0);
        assert_eq!(calculator.seconds, 0.0);
        assert_eq!(beats_to_seconds(f64::NAN, 120.0), 0.0);
        assert_eq!(seconds_to_beats(1.0, f64::INFINITY), 0.0);
    }

    #[test]
    fn a_project_with_no_midi_still_reports_zero_rows() {
        let statistics = project_statistics(&Project::default());
        assert_eq!(statistics.midi_notes, 0);
        assert_eq!(statistics.midi_controllers, 0);
        assert_eq!(statistics.automation_points, 0);
        assert_eq!(statistics.media_entries, 0);
    }
}
