// SPDX-License-Identifier: Apache-2.0

//! A bounded event query, used by the Project Logical Editor.
//!
//! The real product names a scripting surface; this is deliberately smaller: a
//! filter over audio events that either *selects* what it matches or applies one
//! of four safe state changes. It is a query, not a scripting language — adding
//! arbitrary expressions would be a different feature with its own review.
//!
//! Matching happens on the project model, so the same query can drive selection,
//! a state change and tests without any UI.

use serde::{Deserialize, Serialize};

use crate::project::{Project, TrackKind};

/// Maximum characters kept in a text filter.
pub const MAX_QUERY_TEXT_CHARS: usize = 128;

/// Which events a query keeps, by state.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStateFilter {
    #[default]
    Any,
    Muted,
    Unmuted,
    Locked,
    Unlocked,
}

impl EventStateFilter {
    pub const ALL: [Self; 5] = [
        Self::Any,
        Self::Muted,
        Self::Unmuted,
        Self::Locked,
        Self::Unlocked,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Any => "Any state",
            Self::Muted => "Muted",
            Self::Unmuted => "Not muted",
            Self::Locked => "Locked",
            Self::Unlocked => "Not locked",
        }
    }

    fn accepts(self, muted: bool, locked: bool) -> bool {
        match self {
            Self::Any => true,
            Self::Muted => muted,
            Self::Unmuted => !muted,
            Self::Locked => locked,
            Self::Unlocked => !locked,
        }
    }
}

/// One safe change a matched event can be given.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventTransform {
    #[default]
    Mute,
    Unmute,
    Lock,
    Unlock,
}

impl EventTransform {
    pub const ALL: [Self; 4] = [Self::Mute, Self::Unmute, Self::Lock, Self::Unlock];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Mute => "Mute",
            Self::Unmute => "Unmute",
            Self::Lock => "Lock",
            Self::Unlock => "Unlock",
        }
    }
}

/// A bounded filter over audio events.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct EventQuery {
    /// Case-insensitive substring of the event name. Empty matches everything.
    pub name_contains: String,
    /// Case-insensitive substring of the owning track name. Empty matches all.
    pub track_contains: String,
    pub state: EventStateFilter,
    /// Inclusive lower bound on the event start, in beats.
    pub from_beats: Option<f64>,
    /// Exclusive upper bound on the event start, in beats.
    pub to_beats: Option<f64>,
}

impl EventQuery {
    /// Clamps the query into the range matching can honor.
    pub fn sanitize(&mut self) {
        self.name_contains = bounded_text(&self.name_contains);
        self.track_contains = bounded_text(&self.track_contains);
        for value in [&mut self.from_beats, &mut self.to_beats] {
            if value.is_some_and(|beat| !beat.is_finite() || beat < 0.0) {
                *value = None;
            }
        }
        if let (Some(from), Some(to)) = (self.from_beats, self.to_beats)
            && to < from
        {
            // An inverted range matches nothing by accident; drop the upper
            // bound instead of silently keeping a filter the user cannot see.
            self.to_beats = None;
        }
    }

    /// True when the query would match every event, so the UI can say so.
    pub fn matches_everything(&self) -> bool {
        self.name_contains.trim().is_empty()
            && self.track_contains.trim().is_empty()
            && self.state == EventStateFilter::Any
            && self.from_beats.is_none()
            && self.to_beats.is_none()
    }

    fn accepts(&self, clip: &crate::project::AudioClip, track_name: &str) -> bool {
        let name = self.name_contains.trim().to_lowercase();
        if !name.is_empty() && !clip.name.to_lowercase().contains(&name) {
            return false;
        }
        let track = self.track_contains.trim().to_lowercase();
        if !track.is_empty() && !track_name.to_lowercase().contains(&track) {
            return false;
        }
        if let Some(from) = self.from_beats
            && clip.start_beats < from
        {
            return false;
        }
        if let Some(to) = self.to_beats
            && clip.start_beats >= to
        {
            return false;
        }
        self.state.accepts(clip.muted, clip.locked)
    }
}

/// Applies one transform to matched events, in the project's order.
pub fn apply_transform(project: &mut Project, ids: &[u64], transform: EventTransform) -> usize {
    match transform {
        EventTransform::Mute => project.set_audio_clips_muted(ids, true),
        EventTransform::Unmute => project.set_audio_clips_muted(ids, false),
        EventTransform::Lock => project.set_audio_clips_locked(ids, true),
        EventTransform::Unlock => project.set_audio_clips_locked(ids, false),
    }
}

impl Project {
    /// The audio events a query matches, in project order.
    ///
    /// Only audio tracks are searched: MIDI parts are a different object with a
    /// different query contract, and quietly mixing them in would make the
    /// selection count lie.
    pub fn matching_event_ids(&self, query: &EventQuery) -> Vec<u64> {
        let mut ids = Vec::new();
        for track in self
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
        {
            for clip in &track.clips {
                if query.accepts(clip, &track.name) {
                    ids.push(clip.id);
                }
            }
        }
        ids
    }
}

fn bounded_text(text: &str) -> String {
    if text.chars().count() <= MAX_QUERY_TEXT_CHARS {
        return text.to_owned();
    }
    text.chars().take(MAX_QUERY_TEXT_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;
    use std::path::PathBuf;

    fn project_with_events() -> Project {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Midi);
        project.tracks[0].name = "Drums".to_owned();
        project.tracks[1].name = "Bass DI".to_owned();
        // 2 s at 120 BPM is 4 beats.
        project
            .add_audio_clip(0, PathBuf::from("kick.wav"), 0.0, 2.0)
            .unwrap();
        project
            .add_audio_clip(0, PathBuf::from("snare.wav"), 8.0, 2.0)
            .unwrap();
        project
            .add_audio_clip(1, PathBuf::from("bass.wav"), 4.0, 2.0)
            .unwrap();
        project
    }

    #[test]
    fn an_empty_query_matches_every_audio_event_and_nothing_else() {
        let project = project_with_events();
        let query = EventQuery::default();
        assert!(query.matches_everything());
        let ids = project.matching_event_ids(&query);
        assert_eq!(ids.len(), 3, "three audio events, no MIDI parts");
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn the_query_filters_by_event_name_track_name_and_position() {
        let project = project_with_events();

        let by_name = EventQuery {
            name_contains: "SNARE".to_owned(),
            ..EventQuery::default()
        };
        assert_eq!(project.matching_event_ids(&by_name), vec![2]);

        let by_track = EventQuery {
            track_contains: "bass".to_owned(),
            ..EventQuery::default()
        };
        assert_eq!(project.matching_event_ids(&by_track), vec![3]);

        let by_range = EventQuery {
            from_beats: Some(4.0),
            to_beats: Some(8.0),
            ..EventQuery::default()
        };
        assert_eq!(
            project.matching_event_ids(&by_range),
            vec![3],
            "the range is inclusive below and exclusive above"
        );

        let combined = EventQuery {
            track_contains: "drums".to_owned(),
            from_beats: Some(8.0),
            ..EventQuery::default()
        };
        assert_eq!(project.matching_event_ids(&combined), vec![2]);
    }

    #[test]
    fn the_query_filters_by_state() {
        let mut project = project_with_events();
        project.set_audio_clips_muted(&[1], true);
        project.set_audio_clips_locked(&[2], true);

        let muted = EventQuery {
            state: EventStateFilter::Muted,
            ..EventQuery::default()
        };
        assert_eq!(project.matching_event_ids(&muted), vec![1]);

        let locked = EventQuery {
            state: EventStateFilter::Locked,
            ..EventQuery::default()
        };
        assert_eq!(project.matching_event_ids(&locked), vec![2]);

        let unlocked = EventQuery {
            state: EventStateFilter::Unlocked,
            ..EventQuery::default()
        };
        assert_eq!(project.matching_event_ids(&unlocked), vec![1, 3]);
    }

    #[test]
    fn transforms_change_only_the_matched_events() {
        let mut project = project_with_events();
        let query = EventQuery {
            track_contains: "drums".to_owned(),
            ..EventQuery::default()
        };
        let ids = project.matching_event_ids(&query);
        assert_eq!(apply_transform(&mut project, &ids, EventTransform::Mute), 2);
        assert!(project.tracks[0].clips.iter().all(|clip| clip.muted));
        assert!(!project.tracks[1].clips[0].muted);

        // Repeating the same transform reports no change.
        assert_eq!(apply_transform(&mut project, &ids, EventTransform::Mute), 0);

        // Locking the drums, then querying by state, finds exactly those events.
        let ids = project.matching_event_ids(&query);
        assert_eq!(apply_transform(&mut project, &ids, EventTransform::Lock), 2);
        let locked = EventQuery {
            state: EventStateFilter::Locked,
            ..EventQuery::default()
        };
        assert_eq!(project.matching_event_ids(&locked), ids);
        assert_eq!(
            apply_transform(&mut project, &ids, EventTransform::Unlock),
            2
        );
        assert!(project.matching_event_ids(&locked).is_empty());
    }

    #[test]
    fn clamping_keeps_the_query_usable() {
        let mut query = EventQuery {
            name_contains: "x".repeat(MAX_QUERY_TEXT_CHARS * 2),
            from_beats: Some(8.0),
            to_beats: Some(2.0),
            ..EventQuery::default()
        };
        query.sanitize();
        assert_eq!(query.name_contains.chars().count(), MAX_QUERY_TEXT_CHARS);
        assert_eq!(query.from_beats, Some(8.0));
        assert_eq!(
            query.to_beats, None,
            "an inverted range drops its upper bound"
        );

        let mut negative = EventQuery {
            from_beats: Some(-1.0),
            to_beats: Some(f64::NAN),
            ..EventQuery::default()
        };
        negative.sanitize();
        assert_eq!(negative.from_beats, None);
        assert_eq!(negative.to_beats, None);
    }

    #[test]
    fn the_query_round_trips_and_defaults_to_matching_everything() {
        let query = EventQuery {
            name_contains: "take".to_owned(),
            track_contains: "guitar".to_owned(),
            state: EventStateFilter::Locked,
            from_beats: Some(4.0),
            to_beats: Some(12.0),
        };
        let encoded = serde_json::to_string(&query).unwrap();
        let restored: EventQuery = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, query);
        assert!(!restored.matches_everything());

        // An older or empty payload keeps matching everything.
        let legacy: EventQuery = serde_json::from_str("{}").unwrap();
        assert!(legacy.matches_everything());
    }
}
