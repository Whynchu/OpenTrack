//! Bounded retrospective MIDI capture.
//!
//! While the app receives MIDI input, the last [`MAX_SECONDS`] of it (bounded to
//! [`MAX_EVENTS`] events) is kept in memory so a take that was played before
//! Record was pressed can still become a part. Nothing is written to disk and the
//! buffer is dropped when the project closes; the notes are materialised only when
//! the command runs.

use super::*;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Longest captured window, in seconds.
pub(super) const MAX_SECONDS: f64 = 60.0;
/// Hard bound on buffered events, so a fast controller cannot grow memory.
pub(super) const MAX_EVENTS: usize = 8_192;
/// Shortest note created for a key that was still held at the end of the buffer.
pub(super) const UNCLOSED_NOTE_BEATS: f64 = 0.25;

#[derive(Clone, Copy, Debug)]
pub(super) struct Event {
    pub beat: f64,
    pub message: MidiInputMessage,
    pub at: Instant,
}

/// One note reconstructed from the buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CapturedNote {
    pub start_beats: f64,
    pub end_beats: f64,
    pub key: u8,
    pub channel: u8,
    pub velocity: u8,
}

#[derive(Default)]
pub(super) struct Capture {
    events: VecDeque<Event>,
}

impl Capture {
    pub(super) fn push(&mut self, beat: f64, message: MidiInputMessage, at: Instant) {
        if !beat.is_finite() || beat < 0.0 {
            return;
        }
        self.events.push_back(Event { beat, message, at });
        let cutoff = at.checked_sub(Duration::from_secs_f64(MAX_SECONDS));
        while let Some(front) = self.events.front() {
            let too_old = cutoff.is_some_and(|cutoff| front.at < cutoff);
            if too_old || self.events.len() > MAX_EVENTS {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    pub(super) fn clear(&mut self) {
        self.events.clear();
    }

    pub(super) fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.events.len()
    }

    /// Pairs note-on and note-off events per key and channel.
    ///
    /// A key still held when the buffer ends becomes a short note instead of being
    /// dropped, and the pairing is per channel so two parts' input never merges.
    pub(super) fn notes(&self) -> Vec<CapturedNote> {
        let mut open = std::collections::BTreeMap::<(u8, u8), (f64, u8)>::new();
        let mut notes = Vec::new();
        let mut last_beat = 0.0_f64;
        for event in &self.events {
            last_beat = last_beat.max(event.beat);
            match event.message {
                MidiInputMessage::NoteOn {
                    channel,
                    key,
                    velocity,
                } if velocity > 0 => {
                    open.entry((channel, key)).or_insert((event.beat, velocity));
                }
                MidiInputMessage::NoteOn { channel, key, .. }
                | MidiInputMessage::NoteOff { channel, key, .. } => {
                    if let Some((start, velocity)) = open.remove(&(channel, key)) {
                        notes.push(CapturedNote {
                            start_beats: start,
                            end_beats: event.beat.max(start + 1e-6),
                            key,
                            channel,
                            velocity,
                        });
                    }
                }
                MidiInputMessage::ControlChange { .. } => {}
            }
        }
        for ((channel, key), (start, velocity)) in open {
            notes.push(CapturedNote {
                start_beats: start,
                end_beats: start + UNCLOSED_NOTE_BEATS.max(last_beat - start),
                key,
                channel,
                velocity,
            });
        }
        notes.sort_by(|left, right| {
            left.start_beats
                .total_cmp(&right.start_beats)
                .then(left.key.cmp(&right.key))
        });
        notes
    }
}

impl OpenTrackApp {
    /// Records one incoming MIDI event for the retrospective buffer.
    pub(super) fn capture_retrospective_event(
        &mut self,
        beat: f64,
        message: MidiInputMessage,
        at: Instant,
    ) {
        self.retrospective.push(beat, message, at);
    }

    /// Turns the captured buffer into a MIDI part on the armed destination.
    pub(super) fn retrospective_record(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before capturing retrospectively".to_owned();
            return;
        }
        if !self.project_open {
            self.status_message = "Open a project before recording".to_owned();
            return;
        }
        let notes = self.retrospective.notes();
        if notes.is_empty() {
            self.status_message = "No MIDI input was captured in the last 60 seconds".to_owned();
            return;
        }
        let selected = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize));
        let armed = selected
            .filter(|track| {
                matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) && track.armed
            })
            .or_else(|| {
                self.data.project.tracks.iter().find(|track| {
                    matches!(track.kind, TrackKind::Midi | TrackKind::Instrument) && track.armed
                })
            });
        let Some(track) = armed else {
            self.status_message =
                "Arm a MIDI or Instrument track to capture retrospectively".to_owned();
            return;
        };
        let track_index = self
            .data
            .project
            .tracks
            .iter()
            .position(|candidate| candidate.id == track.id)
            .expect("the armed track is in the project");
        let start = notes
            .iter()
            .map(|note| note.start_beats)
            .fold(f64::INFINITY, f64::min);
        let end = notes
            .iter()
            .map(|note| note.end_beats)
            .fold(f64::NEG_INFINITY, f64::max);
        if !start.is_finite() || !end.is_finite() || end <= start {
            self.status_message = "The captured notes have no usable length".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        let Some(part_id) = candidate.add_midi_part(track_index, start, end - start) else {
            self.status_message = "Could not place a part for the captured notes".to_owned();
            return;
        };
        let mut added = 0;
        for note in &notes {
            if candidate
                .add_midi_note_on_channel(
                    track_index,
                    part_id,
                    note.start_beats - start,
                    (note.end_beats - note.start_beats).max(1e-3),
                    note.key,
                    note.channel,
                    note.velocity,
                )
                .is_some()
            {
                added += 1;
            }
        }
        if added == 0 || candidate.validate().is_err() {
            self.status_message = "The captured notes could not form a valid part".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot capture retrospectively: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Retrospective Record");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.retrospective.clear();
        self.status_message = format!(
            "Captured {added} note(s) across {:.3} beat(s); Undo removes them",
            end - start
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(key: u8) -> MidiInputMessage {
        MidiInputMessage::NoteOn {
            channel: 0,
            key,
            velocity: 100,
        }
    }

    fn off(key: u8) -> MidiInputMessage {
        MidiInputMessage::NoteOff {
            channel: 0,
            key,
            velocity: 0,
        }
    }

    #[test]
    fn the_buffer_is_bounded_by_count_and_age() {
        let mut capture = Capture::default();
        let now = Instant::now();
        for index in 0..MAX_EVENTS + 10 {
            capture.push(index as f64 * 0.01, on(60), now);
        }
        assert_eq!(capture.len(), MAX_EVENTS, "the count bound holds");

        // Events older than the window are dropped as new ones arrive.
        let later = now + Duration::from_secs_f64(MAX_SECONDS + 1.0);
        capture.push(1_000.0, off(60), later);
        assert_eq!(capture.len(), 1, "only the fresh event survives");
        capture.clear();
        assert!(capture.is_empty());
    }

    #[test]
    fn notes_pair_per_key_and_channel_and_close_held_keys() {
        let mut capture = Capture::default();
        let now = Instant::now();
        capture.push(1.0, on(60), now);
        capture.push(1.5, off(60), now);
        capture.push(2.0, on(64), now);
        // The 64 key is never released; it must still become a note.
        let notes = capture.notes();
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!(notes[0].key, 60);
        assert_eq!((notes[0].start_beats, notes[0].end_beats), (1.0, 1.5));
        assert_eq!(notes[1].key, 64);
        assert!(notes[1].end_beats > notes[1].start_beats);
        assert!(notes[1].end_beats >= notes[1].start_beats + UNCLOSED_NOTE_BEATS);

        // Two channels playing the same key do not merge.
        let mut capture = Capture::default();
        capture.push(1.0, on(60), now);
        capture.push(
            1.2,
            MidiInputMessage::NoteOn {
                channel: 1,
                key: 60,
                velocity: 90,
            },
            now,
        );
        capture.push(2.0, off(60), now);
        capture.push(
            2.2,
            MidiInputMessage::NoteOff {
                channel: 1,
                key: 60,
                velocity: 0,
            },
            now,
        );
        let notes = capture.notes();
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!(notes[0].end_beats, 2.0);
        assert_eq!(notes[1].end_beats, 2.2);
    }

    fn app_with_armed_track() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Midi);
        app.data.project.tracks[0].armed = true;
        app.project_history.clear();
        app
    }

    #[test]
    fn the_command_materialises_the_buffer_as_one_undo_step() {
        let mut app = app_with_armed_track();
        // Nothing captured yet: the command refuses with a reason.
        app.retrospective_record();
        assert!(app.status_message.contains("No MIDI input was captured"));

        let now = Instant::now();
        app.capture_retrospective_event(2.0, on(60), now);
        app.capture_retrospective_event(2.5, off(60), now);
        app.capture_retrospective_event(3.0, on(67), now);
        app.capture_retrospective_event(3.5, off(67), now);
        app.retrospective_record();

        let part = &app.data.project.tracks[0].midi_parts[0];
        assert_eq!(part.notes.len(), 2);
        assert_eq!(part.start_beats, 2.0);
        assert!((part.duration_beats - 1.5).abs() < 1e-9);
        assert_eq!(part.notes[0].start_beats, 0.0);
        assert_eq!(part.notes[0].pitch, 60);
        assert_eq!(part.notes[1].pitch, 67);
        assert_eq!(part.notes[1].start_beats, 1.0);
        assert!(app.status_message.contains("Captured 2 note(s)"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Retrospective Record");
        assert!(app.retrospective.is_empty(), "the buffer is consumed");

        // Without an armed track the command refuses instead of guessing.
        app.data.project.tracks[0].armed = false;
        app.capture_retrospective_event(1.0, on(60), now);
        app.retrospective_record();
        assert!(
            app.status_message
                .contains("Arm a MIDI or Instrument track")
        );
    }
}
