//! Audio-event interaction, preview and painting. Edits use one domain mutation path.
use super::*;
use opentrack_core::project::AudioClip;
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tool {
    Move,
    Left,
    Right,
    FadeIn,
    FadeOut,
    Gain,
}
#[derive(Clone, Debug)]
struct Gesture {
    peers: Vec<(u64, AudioClip)>,
    track: u64,
    original: AudioClip,
    press: Pos2,
    grab_beats: f64,
    tool: Tool,
    source_duration: f64,
    candidate: AudioClip,
    target: Option<u64>,
}
#[derive(Clone, Debug)]
struct GroupMember {
    track: u64,
    row: usize,
    original: AudioClip,
    candidate: AudioClip,
    target: u64,
}
#[derive(Clone, Debug)]
struct GroupGesture {
    members: Vec<GroupMember>,
    anchor_id: u64,
    anchor_row: usize,
    grab_beats: f64,
    press: Pos2,
    target_valid: bool,
    group_master: bool,
    tool: Tool,
    group_segment: Option<(u64, Option<f64>, Option<f64>)>,
    segment_bounds: Option<(f64, f64)>,
}

pub(super) struct GroupSegmentPreview {
    pub group_id: u64,
    pub original_bounds: (f64, f64),
    pub candidate_bounds: (f64, f64),
    pub original_clips: Vec<(u64, AudioClip)>,
    pub candidate_clips: Vec<(u64, AudioClip)>,
}
#[derive(Clone, Debug)]
struct MarqueeGesture {
    press: Pos2,
    anchor_beat: f64,
    anchor_content_y: f32,
    initial_primary: Option<u64>,
    initial_extra: HashSet<u64>,
    modifiers: egui::Modifiers,
    active: bool,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    rename: Option<(u64, u64, String, bool)>,
    pub rename_request: Option<(u64, u64, String)>,
    last_name_click: Option<(u64, f64, Pos2)>,
    selected: Option<u64>,
    pub row_focus: bool,
    extra_selected: HashSet<u64>,
    selected_group_segment: Option<(u64, usize)>,
    toolbox: Option<Pos2>,
    pub open_editor: Option<(u64, u64)>,
    pub message: Option<String>,
    gesture: Option<Gesture>,
    group_gesture: Option<GroupGesture>,
    marquee: Option<MarqueeGesture>,
}
/// Selects one event, expanding the selection to its group.
///
/// A grouped event behaves as one object, so clicking any member selects the
/// whole group and every gesture that follows has the same target.
/// Where a real copy of `source` should live, or `None` when it is already the
/// project's own media.
fn audio_copy_destination(source: &Path, project_root: &Path) -> Result<Option<PathBuf>, String> {
    let (destination, copied) = super::audio_import_destination(source, project_root)?;
    Ok(copied.then_some(destination))
}

fn select_event_including_group(project: &Project, state: &mut State, clip_id: u64) {
    let members = project.expand_selection_to_groups(&[clip_id]);
    if members.len() > 1 {
        state.select_many(members);
    } else {
        state.select_one(clip_id);
    }
}

impl State {
    pub fn edit_drag_active(&self) -> bool {
        self.gesture.is_some() || self.group_gesture.is_some()
    }

    pub fn selected_ids(&self) -> Vec<u64> {
        self.selected
            .into_iter()
            .chain(self.extra_selected.iter().copied())
            .collect()
    }
    pub fn focus_row(&mut self) {
        self.clear_selection();
        self.row_focus = true;
    }
    pub fn clear_selection(&mut self) {
        self.row_focus = false;
        self.selected = None;
        self.extra_selected.clear();
        self.selected_group_segment = None;
        self.gesture = None;
        self.group_gesture = None;
        self.marquee = None;
    }
    pub fn select_one(&mut self, id: u64) {
        self.row_focus = false;
        self.selected = Some(id);
        self.extra_selected.clear();
        self.selected_group_segment = None;
    }
    pub fn select_many(&mut self, ids: impl IntoIterator<Item = u64>) {
        self.clear_selection();
        let mut ids = ids.into_iter();
        self.selected = ids.next();
        self.extra_selected.extend(ids);
    }
    fn select_group_segment(
        &mut self,
        group_id: u64,
        segment_index: usize,
        ids: impl IntoIterator<Item = u64>,
    ) {
        self.select_many(ids);
        self.selected_group_segment = Some((group_id, segment_index));
    }
    pub fn selected_group_segment(&self) -> Option<(u64, usize)> {
        self.selected_group_segment
    }
    pub(super) fn group_segment_preview(&self, tempo: f64) -> Option<GroupSegmentPreview> {
        let gesture = self.group_gesture.as_ref()?;
        let group_id = gesture.group_segment?.0;
        let original_bounds = gesture.segment_bounds?;
        let min = gesture
            .members
            .iter()
            .map(|member| member.candidate.start_beats)
            .fold(f64::INFINITY, f64::min);
        let candidate_end = gesture
            .members
            .iter()
            .map(|member| {
                member.candidate.start_beats + member.candidate.duration_seconds * tempo / 60.0
            })
            .fold(f64::NEG_INFINITY, f64::max);
        Some(GroupSegmentPreview {
            group_id,
            original_bounds,
            candidate_bounds: (min, candidate_end),
            original_clips: gesture
                .members
                .iter()
                .map(|member| (member.track, member.original.clone()))
                .collect(),
            candidate_clips: gesture
                .members
                .iter()
                .map(|member| (member.track, member.candidate.clone()))
                .collect(),
        })
    }
}

#[derive(Clone, Debug)]
struct ClipboardItem {
    track_offset: usize,
    channels: u8,
    clip: AudioClip,
}

#[derive(Clone, Debug)]
pub(super) struct Clipboard {
    anchor_track_id: u64,
    anchor_beat: f64,
    anchor_group_id: Option<u64>,
    items: Vec<ClipboardItem>,
}

fn split_selected_clips(project: &mut Project, selected: &HashSet<u64>, beat: f64) -> usize {
    let mut candidate = project.clone();
    let targets = candidate
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .flat_map(|track| {
            track
                .clips
                .iter()
                .filter(|clip| selected.contains(&clip.id))
                .map(move |clip| (track.id, clip.id))
        })
        .collect::<Vec<_>>();
    let split = targets
        .into_iter()
        .filter(|(track_id, clip_id)| {
            candidate
                .split_audio_clip(*track_id, *clip_id, beat)
                .is_some()
        })
        .count();
    let selected_count = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .flat_map(|track| &track.clips)
        .filter(|clip| selected.contains(&clip.id))
        .count();
    if selected_count == selected.len() && split == selected_count && split > 0 {
        *project = candidate;
        split
    } else {
        0
    }
}

fn crossfade_selection(project: &mut Project, ids: &[u64]) -> bool {
    ids.len() == 2 && project.crossfade_audio_clips(ids[0], ids[1])
}

fn move_selected_clips_to_cursor(
    project: &mut Project,
    selected: &HashSet<u64>,
    cursor_beat: f64,
    follow_automation: bool,
) -> usize {
    if !cursor_beat.is_finite() || cursor_beat < 0.0 || selected.is_empty() {
        return 0;
    }
    let clips = project
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .flat_map(|track| {
            track
                .clips
                .iter()
                .filter(|clip| selected.contains(&clip.id))
                .map(move |clip| (track.id, clip.clone()))
        })
        .collect::<Vec<_>>();
    if clips.len() != selected.len() {
        return 0;
    }
    let anchor = clips
        .iter()
        .map(|(_, clip)| clip.start_beats)
        .fold(f64::INFINITY, f64::min);
    let edits = clips
        .iter()
        .map(|(track_id, clip)| {
            let mut moved = clip.clone();
            moved.start_beats = cursor_beat + clip.start_beats - anchor;
            (*track_id, *track_id, moved)
        })
        .collect::<Vec<_>>();
    if project.move_audio_clips_with_automation(&edits, follow_automation) {
        clips.len()
    } else {
        0
    }
}

/// A muted event, or a take that is not the active one, never reaches playback.
pub(super) fn clip_is_audible(clip: &AudioClip) -> bool {
    !clip.muted && clip.take_active
}

impl OpenTrackApp {
    pub(super) fn move_selected_audio_events_to_cursor(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before moving audio events".into();
            return;
        }
        let selected = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .into_iter()
            .collect::<HashSet<_>>();
        let beat = self.data.transport.position_beats.max(0.0);
        let beat = self
            .arrange_view_state
            .snap_grid_beats(self.data.project.beats_per_bar)
            .map_or(beat, |grid| crate::timeline::snap_beats_to_grid(beat, grid));
        let mut candidate = self.data.project.clone();
        let moved = move_selected_clips_to_cursor(
            &mut candidate,
            &selected,
            beat,
            self.automation_follows_events,
        );
        if moved == 0 {
            self.status_message = "Select audio events that can move to the cursor".into();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot move: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Move to Cursor");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.arrange_view_state
            .audio_events
            .select_many(selected.into_iter().collect::<Vec<_>>());
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message =
            format!("Moved {moved} audio event(s) to the cursor; Undo restores them");
    }

    /// Copies the selected events after the selection, as one undo step.
    pub(super) fn duplicate_selected_audio_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before duplicating audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to duplicate".into();
            return;
        }
        let mut candidate = self.data.project.clone();
        let created = candidate.duplicate_audio_clips(&ids);
        if created.is_empty() {
            self.status_message = "The selected audio events could not be duplicated".into();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot duplicate: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Duplicate");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.arrange_view_state
            .audio_events
            .select_many(created.iter().copied());
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Duplicated {} audio event(s) after the selection; Undo removes them",
            created.len()
        );
    }

    /// Mutes or unmutes the selected events as one undo step.
    pub(super) fn set_selected_audio_events_muted(&mut self, muted: bool) {
        if self.is_recording() {
            self.status_message = "Stop recording before muting audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to mute or unmute".into();
            return;
        }
        let mut candidate = self.data.project.clone();
        let changed = candidate.set_audio_clips_muted(&ids, muted);
        if changed == 0 {
            self.status_message = if muted {
                "The selected audio events are already muted".into()
            } else {
                "The selected audio events are already unmuted".into()
            };
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot change mute: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Mute Events");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "{} {changed} audio event(s); Undo restores them",
            if muted { "Muted" } else { "Unmuted" }
        );
    }

    /// Repeats the selected events `count` more times after themselves.
    pub(super) fn repeat_selected_audio_events(&mut self, count: u32) {
        if self.is_recording() {
            self.status_message = "Stop recording before repeating audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to repeat".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        let created = candidate.repeat_audio_clips(&ids, count);
        if created.is_empty() {
            self.status_message = format!(
                "Cannot repeat {} time(s); check the selection and the timeline end",
                count
            );
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot repeat: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Repeat");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.arrange_view_state
            .audio_events
            .select_many(ids.into_iter().chain(created.iter().copied()));
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Repeated {} audio event(s) {} time(s); Undo removes them",
            created.len(),
            count
        );
    }

    /// Repeats the selected events until the loop end.
    pub(super) fn fill_loop_with_selected_audio_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before filling the loop".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to fill the loop".to_owned();
            return;
        }
        let (start, end) = (
            self.data.project.loop_start_beats,
            self.data.project.loop_end_beats,
        );
        let mut candidate = self.data.project.clone();
        let created = candidate.fill_loop_with_audio_clips(&ids, start, end);
        if created.is_empty() {
            self.status_message =
                "Fill Loop needs a loop range after the selected events".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot fill the loop: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Fill Loop");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.arrange_view_state
            .audio_events
            .select_many(ids.into_iter().chain(created.iter().copied()));
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Filled the loop with {} repeated audio event(s); Undo removes them",
            created.len()
        );
    }

    /// Trims the selected events to the loop range.
    pub(super) fn crop_selected_audio_events_to_loop(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before cropping audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to crop".to_owned();
            return;
        }
        let (start, end) = (
            self.data.project.loop_start_beats,
            self.data.project.loop_end_beats,
        );
        let mut candidate = self.data.project.clone();
        let trimmed = candidate.crop_audio_clips_to_range(&ids, start, end);
        if trimmed == 0 {
            self.status_message = "No selected audio event crosses the loop range".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot crop: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Crop to Loop");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message =
            format!("Cropped {trimmed} audio event(s) to the loop range; Undo restores them");
    }

    /// Makes the selected event the active take of its loop-recording group.
    pub(super) fn make_selected_event_active_take(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before choosing a take".to_owned();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        let Some(clip_id) = ids.first().copied() else {
            self.status_message = "Select one recorded take to make active".to_owned();
            return;
        };
        if ids.len() > 1 {
            self.status_message = "Select one take at a time to make it active".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        if !candidate.set_active_take(clip_id) {
            self.status_message =
                "The selected event is not one of several takes in a loop recording".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot switch takes: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Select Take");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = "That take now plays; the others are kept but silent".to_owned();
    }

    /// Joins contiguous pieces of one source into a single event.
    pub(super) fn glue_selected_audio_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before gluing audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.len() < 2 {
            self.status_message = "Select at least two audio events to glue".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        let Some(merged) = candidate.glue_audio_clips(&ids) else {
            self.status_message =
                "Glue needs contiguous pieces of one un-faded source on the same track".to_owned();
            return;
        };
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot glue: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Glue");
        self.data.project = candidate;
        self.arrange_view_state.audio_events.select_many([merged]);
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = "Glued the selected audio events; Undo splits them again".to_owned();
    }

    /// One grid step for nudging, or a quarter bar when snap is off.
    fn arrange_nudge_step(&self) -> f64 {
        self.arrange_view_state
            .snap_grid_beats(self.data.project.beats_per_bar)
            .unwrap_or_else(|| f64::from(self.data.project.beats_per_bar.max(1)) / 4.0)
    }

    /// Moves the selected events' start or end by one grid step.
    ///
    /// The start carries the source offset with it; the end only changes the
    /// duration. Both stop at the media and at the minimum length, and a locked
    /// event is skipped rather than half-edited.
    pub(super) fn nudge_selected_event_edges(
        &mut self,
        edge: opentrack_core::project::ClipEdge,
        right: bool,
    ) {
        if self.is_recording() {
            self.status_message = "Stop recording before nudging event edges".to_owned();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to nudge their edges".to_owned();
            return;
        }
        let step = self.arrange_nudge_step();
        let delta = if right { step } else { -step };
        let min_seconds = 1.0 / self.data.project.sample_rate_hz.max(1) as f64;
        let mut candidate = self.data.project.clone();
        let mut changed = 0;
        for id in ids {
            let Some((path, offset, duration)) = candidate
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter())
                .find(|clip| clip.id == id)
                .map(|clip| {
                    (
                        clip.source_path.clone(),
                        clip.source_offset_seconds,
                        clip.duration_seconds,
                    )
                })
            else {
                continue;
            };
            // A source that is not loaded can only be trimmed inwards: extending
            // past the known extent would read audio that may not exist.
            let source_duration = self
                .data
                .audio_sources
                .get(&path)
                .map(|source| source.duration_seconds())
                .unwrap_or(offset + duration);
            if candidate.nudge_audio_clip_edge(id, edge, delta, source_duration, min_seconds) {
                changed += 1;
            }
        }
        if changed == 0 {
            self.status_message =
                "No event edge could move: locked events and the media limits are respected"
                    .to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot nudge event edges: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next(match edge {
            opentrack_core::project::ClipEdge::Start => "Nudge Event Start",
            opentrack_core::project::ClipEdge::End => "Nudge Event End",
        });
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Nudged {} event {} {} by {step:.3} beat(s); Undo restores them",
            changed,
            if edge == opentrack_core::project::ClipEdge::Start {
                "start"
            } else {
                "end"
            },
            if right { "right" } else { "left" }
        );
    }

    /// Applies the default fade length to the selection's leading or trailing edge.
    pub(super) fn fade_selected_audio_events(&mut self, fade_in: bool) {
        const DEFAULT_FADE_SECONDS: f64 = 0.25;
        if self.is_recording() {
            self.status_message = "Stop recording before fading audio events".to_owned();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to fade".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        let changed = candidate.fade_audio_clips(&ids, fade_in, DEFAULT_FADE_SECONDS);
        if changed == 0 {
            self.status_message =
                "No event changed; locked events and the current fade are left alone".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot fade: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history
            .label_next(if fade_in { "Fade In" } else { "Fade Out" });
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Applied a {DEFAULT_FADE_SECONDS:.2} s {} to {changed} event(s); Undo restores them",
            if fade_in { "fade in" } else { "fade out" }
        );
    }

    /// Silences the selected events by zeroing their gain, keeping the media.
    pub(super) fn silence_selected_audio_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before silencing audio events".to_owned();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to silence".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        let changed = candidate.silence_audio_clips(&ids);
        if changed == 0 {
            self.status_message =
                "No event changed; locked and already silent events are left alone".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot silence: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Silence");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!("Silenced {changed} event(s); the media stays in the pool");
    }

    /// Removes the loop range from the selected events, as one undo step.
    pub(super) fn delete_loop_range_from_selected_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before deleting a range".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to delete a range from".to_owned();
            return;
        }
        let (start, end) = (
            self.data.project.loop_start_beats,
            self.data.project.loop_end_beats,
        );
        let mut candidate = self.data.project.clone();
        let removed = candidate.delete_range_from_audio_clips(&ids, start, end);
        if removed == 0 {
            self.status_message = "No selected audio event overlaps the loop range".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot delete the range: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Delete Range");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message =
            format!("Deleted the loop range from {removed} audio event(s); Undo restores them");
    }

    /// Moves the selected events by one grid step, left or right.
    ///
    /// Locked events and locked group members are refused by the core move, so a
    /// nudge says why instead of writing a partial edit.
    pub(super) fn nudge_selected_audio_events(&mut self, right: bool) {
        if self.is_recording() {
            self.status_message = "Stop recording before nudging audio events".into();
            return;
        }
        let selected = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .into_iter()
            .collect::<HashSet<_>>();
        if selected.is_empty() {
            self.status_message = "Select audio events to nudge".to_owned();
            return;
        }
        let step = self
            .arrange_view_state
            .snap_grid_beats(self.data.project.beats_per_bar)
            .unwrap_or_else(|| f64::from(self.data.project.beats_per_bar.max(1)) / 4.0);
        let delta = if right { step } else { -step };
        let tempo = self.data.project.tempo_bpm;
        let mut candidate = self.data.project.clone();
        let mut edits = Vec::new();
        for track in candidate
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
        {
            for clip in &track.clips {
                if !selected.contains(&clip.id) {
                    continue;
                }
                let mut moved = clip.clone();
                moved.start_beats += delta;
                if moved.start_beats < 0.0 {
                    self.status_message =
                        "Nudging left would move an audio event before the project start"
                            .to_owned();
                    return;
                }
                edits.push((track.id, track.id, moved));
            }
        }
        if edits.is_empty() {
            self.status_message = "The selection no longer contains audio events".to_owned();
            return;
        }
        let end = edits
            .iter()
            .map(|(_, _, clip)| clip.start_beats + clip.duration_seconds * tempo / 60.0)
            .fold(0.0_f64, f64::max);
        if end > 1_000_000.0 {
            self.status_message = "Nudging right would leave the supported timeline".to_owned();
            return;
        }
        if !candidate.move_audio_clips_with_automation(&edits, self.automation_follows_events) {
            self.status_message =
                "The selected audio events could not be nudged (a locked event blocks it)"
                    .to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot nudge: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Nudge");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Nudged {} event(s) {} by {step:.3} beat(s); Undo restores them",
            edits.len(),
            if right { "right" } else { "left" }
        );
    }

    /// Splits every audio event crossing the locators, as one undo step.
    pub(super) fn split_audio_events_at_loop(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before splitting audio events".into();
            return;
        }
        let mut candidate = self.data.project.clone();
        let (split, _) = candidate.split_audio_clips_at_loop();
        if split == 0 {
            self.status_message = "No audio event crosses a locator".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot split: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Split Loop");
        self.data.project = candidate;
        self.arrange_view_state.audio_events.clear_selection();
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message =
            format!("Split {split} audio event(s) at the locators; Undo restores them");
    }

    /// Copies the selected events' media into the project's own import folder.
    ///
    /// The event keeps every edit but stops depending on a shared source file, so
    /// editing or removing that source elsewhere cannot change this project.
    pub(super) fn convert_selected_events_to_real_copy(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before converting audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to convert".to_owned();
            return;
        }
        if !self.project_folder_managed {
            self.status_message =
                "Choose or create this project's folder before converting audio events".to_owned();
            return;
        }
        let Some(project_root) = self
            .project_path
            .as_deref()
            .and_then(Path::parent)
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
        else {
            self.status_message =
                "Save this project into its folder before converting audio events".to_owned();
            return;
        };

        let mut candidate = self.data.project.clone();
        let mut copied = Vec::new();
        let mut sources = Vec::new();
        let mut already_local = 0;
        let mut skipped_pitch = 0;
        let mut issues = Vec::new();
        for track in candidate
            .tracks
            .iter_mut()
            .filter(|track| track.kind == TrackKind::Audio)
        {
            for clip in track.clips.iter_mut() {
                if !ids.contains(&clip.id) {
                    continue;
                }
                if clip.pitch_time.is_some() {
                    // Pitch & Time analysis is keyed to the source bytes and
                    // modification time, so a copy would invalidate it.
                    skipped_pitch += 1;
                    continue;
                }
                match audio_copy_destination(&clip.source_path, &project_root) {
                    Ok(None) => already_local += 1,
                    Ok(Some(destination)) => match fs::copy(&clip.source_path, &destination) {
                        Ok(_) => {
                            copied.push((clip.source_path.clone(), destination.clone()));
                            clip.source_path = destination.clone();
                            sources.push(destination);
                        }
                        Err(error) => issues.push(format!(
                            "could not copy {}: {error}",
                            clip.source_path.display()
                        )),
                    },
                    Err(error) => issues.push(error),
                }
            }
        }

        if copied.is_empty() {
            let mut status = if already_local > 0 {
                "Those audio events already use this project's own media".to_owned()
            } else {
                "No audio event could be converted".to_owned()
            };
            if skipped_pitch > 0 {
                status.push_str(&format!(
                    "; {skipped_pitch} event(s) with Pitch & Time edits are left unchanged because a copy would invalidate the analysis"
                ));
            }
            if !issues.is_empty() {
                status.push_str("; ");
                status.push_str(&issues.join(" · "));
            }
            self.status_message = status;
            return;
        }

        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            // The copies are on disk already; the event still points at its
            // original source, so the project is unchanged and consistent.
            for (_, destination) in &copied {
                let _ = fs::remove_file(destination);
            }
            self.status_message = "Cannot convert: undo snapshot is unavailable".into();
            return;
        }
        self.data.project = candidate;
        for path in sources {
            if let Ok(source) = AudioSource::open(&path) {
                self.data.audio_sources.insert(path, source);
            } else {
                issues.push(format!("{} could not be opened", path.display()));
            }
        }
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        let mut status = format!(
            "Converted {} audio event(s) to copies in Audio/Imports",
            copied.len()
        );
        if already_local > 0 {
            status.push_str(&format!("; {already_local} already used project media"));
        }
        if skipped_pitch > 0 {
            status.push_str(&format!(
                "; {skipped_pitch} event(s) with Pitch & Time edits were left unchanged"
            ));
        }
        if !issues.is_empty() {
            status.push_str("; ");
            status.push_str(&issues.join(" · "));
        }
        self.status_message = status;
    }

    /// Selects the events under the cursor when the preference asks for it.
    pub(super) fn apply_auto_select_under_cursor(&mut self) {
        if !self.auto_select_events_under_cursor {
            return;
        }
        let beat = self.data.transport.position_beats;
        let tempo = self.data.project.tempo_bpm;
        let ids = self
            .data
            .project
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio && !track.muted)
            .flat_map(|track| track.clips.iter())
            .filter(|clip| {
                let end = clip.start_beats + clip.duration_seconds * tempo / 60.0;
                clip.start_beats <= beat && beat < end
            })
            .map(|clip| clip.id)
            .collect::<Vec<_>>();
        self.arrange_view_state
            .audio_events
            .select_many(self.data.project.expand_selection_to_groups(&ids));
    }

    /// Locks or unlocks the selected events as one undo step.
    pub(super) fn set_selected_audio_events_locked(&mut self, locked: bool) {
        if self.is_recording() {
            self.status_message = "Stop recording before locking audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to lock or unlock".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        let changed = candidate.set_audio_clips_locked(&ids, locked);
        if changed == 0 {
            self.status_message = if locked {
                "The selected audio events are already locked".into()
            } else {
                "The selected audio events are not locked".into()
            };
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot change the lock: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Lock Events");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message = format!(
            "{} {changed} audio event(s); Undo restores them",
            if locked { "Locked" } else { "Unlocked" }
        );
    }

    /// Binds the selected events so they move together.
    pub(super) fn group_selected_audio_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before grouping audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        let mut candidate = self.data.project.clone();
        let Some(group) = candidate.group_audio_clips(&ids) else {
            self.status_message = "Select at least two audio events to group".to_owned();
            return;
        };
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot group: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Group Events");
        let members = candidate.expand_selection_to_groups(&ids);
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.arrange_view_state.audio_events.select_many(members);
        self.mark_modified();
        self.status_message =
            format!("Grouped the selected audio events as {group}; Undo removes the group");
    }

    /// Releases the selected events (and their groups) from grouping.
    pub(super) fn ungroup_selected_audio_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before ungrouping audio events".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        let mut candidate = self.data.project.clone();
        let changed = candidate.ungroup_audio_clips(&ids);
        if changed == 0 {
            self.status_message = "The selected audio events are not grouped".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot ungroup: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Ungroup Events");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message =
            format!("Ungrouped {changed} audio event(s); Undo restores the group");
    }

    /// Selects the audio events that overlap the transport position.
    pub(super) fn select_audio_events_under_cursor(&mut self) {
        let beat = self.data.transport.position_beats;
        let tempo = self.data.project.tempo_bpm;
        let ids = self
            .data
            .project
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .flat_map(|track| track.clips.iter())
            .filter(|clip| {
                let end = clip.start_beats + clip.duration_seconds * tempo / 60.0;
                clip.start_beats <= beat && beat < end
            })
            .map(|clip| clip.id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            self.status_message = format!("No audio event is under beat {beat:.3}");
            return;
        }
        let count = ids.len();
        self.arrange_view_state.audio_events.select_many(ids);
        self.status_message = format!("Selected {count} audio event(s) under the cursor");
    }

    /// Moves the selected events so their first event starts at a locator.
    pub(super) fn move_selected_audio_events_to_locator(&mut self, left: bool) {
        if self.is_recording() {
            self.status_message = "Stop recording before moving audio events".into();
            return;
        }
        let selected = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .into_iter()
            .collect::<HashSet<_>>();
        if selected.is_empty() {
            self.status_message = "Select audio events to move".to_owned();
            return;
        }
        let beat = if left {
            self.data.project.loop_start_beats
        } else {
            self.data.project.loop_end_beats
        };
        let mut candidate = self.data.project.clone();
        let moved = move_selected_clips_to_cursor(
            &mut candidate,
            &selected,
            beat,
            self.automation_follows_events,
        );
        if moved == 0 {
            self.status_message = "The selected audio events could not be moved".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot move: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Move to Locator");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.arrange_view_state
            .audio_events
            .select_many(selected.into_iter().collect::<Vec<_>>());
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Moved {moved} audio event(s) to the {} locator at beat {beat:.3}; Undo restores them",
            if left { "left" } else { "right" }
        );
    }

    /// Selects every audio event that is not selected yet, leaving the rest.
    pub(super) fn invert_audio_event_selection(&mut self) {
        let selected = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .into_iter()
            .collect::<HashSet<_>>();
        let all = self
            .data
            .project
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .flat_map(|track| track.clips.iter().map(|clip| clip.id))
            .collect::<Vec<_>>();
        if all.is_empty() {
            self.status_message = "There are no audio events to select".to_owned();
            return;
        }
        let inverted = all
            .iter()
            .copied()
            .filter(|id| !selected.contains(id))
            .collect::<Vec<_>>();
        if inverted.is_empty() {
            self.arrange_view_state.audio_events.clear_selection();
            self.status_message = "Inverted selection: nothing left to select".to_owned();
            return;
        }
        let count = inverted.len();
        self.arrange_view_state.audio_events.select_many(inverted);
        self.status_message = format!("Inverted selection: {count} audio event(s) selected");
    }

    /// Toggles mute on the selection, matching the reference Mute/Unmute Objects command.
    pub(super) fn toggle_selected_audio_events_muted(&mut self) {
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if ids.is_empty() {
            self.status_message = "Select audio events to mute or unmute".into();
            return;
        }
        let muted = ids.iter().all(|id| {
            self.data
                .project
                .tracks
                .iter()
                .flat_map(|track| &track.clips)
                .any(|clip| clip.id == *id && clip.muted)
        });
        self.set_selected_audio_events_muted(!muted);
    }

    /// Selects every audio event in the project, across all tracks.
    pub(super) fn select_all_audio_events(&mut self) {
        let ids = self
            .data
            .project
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .flat_map(|track| track.clips.iter().map(|clip| clip.id))
            .collect::<Vec<_>>();
        if ids.is_empty() {
            self.status_message = "There are no audio events to select".to_owned();
            return;
        }
        let count = ids.len();
        self.arrange_view_state.audio_events.select_many(ids);
        self.status_message = format!("Selected {count} audio event(s)");
    }

    pub(super) fn select_no_audio_events(&mut self) {
        self.arrange_view_state.audio_events.clear_selection();
        self.status_message = "Cleared the event selection".to_owned();
    }

    pub(super) fn crossfade_selected_audio_events(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before creating a crossfade".into();
            return;
        }
        let ids = self.arrange_view_state.audio_events.selected_ids();
        if !crossfade_selection(&mut self.data.project, &ids) {
            self.status_message =
                "Crossfade needs two overlapping audio events on the same track".into();
            return;
        }
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = "Crossfade created; Undo restores the previous fade".into();
    }

    pub(super) fn split_selected_audio_events_at_cursor(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before splitting audio events".into();
            return;
        }
        if let Some((group_id, _)) = self
            .arrange_view_state
            .audio_events
            .selected_group_segment()
        {
            // A group segment is a virtual event spanning its members, so it is
            // split through the group so every member keeps the shared boundary.
            let beat = self.data.transport.position_beats;
            let mut candidate = self.data.project.clone();
            let (changed, _) = candidate.split_group_audio_clips(group_id, beat);
            if !changed {
                self.status_message = "The group segment does not cross the cursor".into();
                return;
            }
            self.project_history.finish_frame(&self.data.project, true);
            if !self
                .project_history
                .begin(&self.data.project, self.modified)
            {
                self.status_message = "Cannot split: undo snapshot is unavailable".into();
                return;
            }
            self.project_history.label_next("Split at Cursor");
            self.data.project = candidate;
            self.project_history.commit_after_frame = true;
            self.rebuild_audio_plan();
            self.mark_modified();
            self.status_message =
                "Split the group segment at the cursor; Undo restores it".to_owned();
            return;
        }
        let selected: HashSet<_> = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .into_iter()
            .collect();
        if selected.is_empty() {
            self.status_message = "Select audio events to split".into();
            return;
        }
        let beat = self.data.transport.position_beats;
        let mut candidate = self.data.project.clone();
        let split = split_selected_clips(&mut candidate, &selected, beat);
        if split == 0 {
            self.status_message = "No selected audio event crosses the cursor".into();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot split: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Split at Cursor");
        self.data.project = candidate;
        self.arrange_view_state.audio_events.clear_selection();
        self.project_history.finish_frame(&self.data.project, true);
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message =
            format!("Split {split} audio event(s) at the cursor; Undo restores them");
    }

    /// Selects every audio event that overlaps the cycle loop.
    pub(super) fn select_audio_events_in_loop(&mut self) {
        let start = self.data.project.loop_start_beats;
        let end = self.data.project.loop_end_beats;
        if end <= start {
            self.status_message = "Set a loop range before selecting inside it".to_owned();
            return;
        }
        let tempo = self.data.project.tempo_bpm;
        let ids = self
            .data
            .project
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .flat_map(|track| track.clips.iter())
            .filter(|clip| {
                let clip_end = clip.start_beats + clip.duration_seconds * tempo / 60.0;
                clip.start_beats < end && clip_end > start
            })
            .map(|clip| clip.id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            self.status_message = "No audio events overlap the loop".to_owned();
            return;
        }
        let count = ids.len();
        self.arrange_view_state.audio_events.select_many(ids);
        self.status_message = format!("Selected {count} audio event(s) in the loop");
    }

    /// Moves the loop locator to the selection's first or last event edge.
    pub(super) fn set_locator_from_selection(&mut self, left: bool) {
        let tempo = self.data.project.tempo_bpm;
        let selected = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .into_iter()
            .collect::<HashSet<_>>();
        let clips = self
            .data
            .project
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .filter(|clip| selected.contains(&clip.id))
            .collect::<Vec<_>>();
        if clips.is_empty() {
            self.status_message = "Select audio events to place a locator".to_owned();
            return;
        }
        let edge = if left {
            clips
                .iter()
                .map(|clip| clip.start_beats)
                .fold(f64::INFINITY, f64::min)
        } else {
            clips
                .iter()
                .map(|clip| clip.start_beats + clip.duration_seconds * tempo / 60.0)
                .fold(f64::NEG_INFINITY, f64::max)
        };
        if !edge.is_finite() || edge <= 0.0 {
            self.status_message = "The selection does not end after the project start".to_owned();
            return;
        }
        let mut candidate = self.data.project.clone();
        if left {
            if edge >= candidate.loop_end_beats {
                self.status_message =
                    "The left locator must stay before the right locator".to_owned();
                return;
            }
            candidate.loop_start_beats = edge;
        } else {
            if edge <= candidate.loop_start_beats {
                self.status_message =
                    "The right locator must stay after the left locator".to_owned();
                return;
            }
            candidate.loop_end_beats = edge;
        }
        if candidate.validate().is_err() {
            self.status_message = "The locator position is outside the supported range".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot move the locator: undo snapshot is unavailable".into();
            return;
        }
        self.project_history.label_next("Set Locator");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message = format!(
            "Set the {} locator to beat {edge:.3}",
            if left { "left" } else { "right" }
        );
    }

    pub(super) fn cut_selected_audio_events(&mut self) {
        if self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .is_empty()
        {
            self.status_message = "Select audio events to cut".into();
            return;
        }
        self.copy_selected_audio_events();
        self.delete_audio_selection(false);
    }

    pub(super) fn copy_selected_audio_events(&mut self) {
        let selected: HashSet<_> = self
            .arrange_view_state
            .audio_events
            .selected_ids()
            .into_iter()
            .collect();
        if selected.is_empty() {
            self.status_message = "Select audio events to copy".into();
            return;
        }
        let mut found = Vec::new();
        for (row, track) in self.data.project.tracks.iter().enumerate() {
            if track.kind != TrackKind::Audio {
                continue;
            }
            for clip in &track.clips {
                if selected.contains(&clip.id) {
                    found.push((row, track.id, track.audio_channels, clip.clone()));
                }
            }
        }
        if found.is_empty() {
            self.status_message = "The selected audio events are no longer in the project".into();
            return;
        }
        let anchor_row = found.iter().map(|(row, ..)| *row).min().unwrap_or_default();
        let selected_group_id = self
            .data
            .project
            .selected_track
            .and_then(|index| self.data.project.tracks.get(index as usize))
            .filter(|track| track.kind == TrackKind::Group)
            .map(|track| track.id)
            .filter(|group_id| {
                found.iter().all(|(_, track_id, _, _)| {
                    track_belongs_to_group(&self.data.project, *track_id, *group_id)
                })
            });
        let group_track_order = selected_group_id.map(|group_id| {
            self.data
                .project
                .tracks
                .iter()
                .filter(|track| {
                    track.kind == TrackKind::Audio
                        && track_belongs_to_group(&self.data.project, track.id, group_id)
                })
                .map(|track| track.id)
                .collect::<Vec<_>>()
        });
        let copy_track_order = group_track_order.clone().unwrap_or_else(|| {
            self.data
                .project
                .tracks
                .iter()
                .filter(|track| track.kind == TrackKind::Audio)
                .map(|track| track.id)
                .collect()
        });
        let anchor_track_id = group_track_order
            .as_ref()
            .and_then(|tracks| tracks.first().copied())
            .unwrap_or(self.data.project.tracks[anchor_row].id);
        let anchor_track_offset = copy_track_order
            .iter()
            .position(|id| *id == anchor_track_id)
            .unwrap_or_default();
        let anchor_beat = found
            .iter()
            .map(|(_, _, _, clip)| clip.start_beats)
            .fold(f64::INFINITY, f64::min);
        let mut items: Vec<_> = found
            .into_iter()
            .map(|(_row, track_id, channels, clip)| ClipboardItem {
                track_offset: copy_track_order
                    .iter()
                    .position(|id| *id == track_id)
                    .unwrap_or(anchor_track_offset)
                    .saturating_sub(anchor_track_offset),
                channels,
                clip,
            })
            .collect();
        items.sort_by(|a, b| {
            a.track_offset
                .cmp(&b.track_offset)
                .then_with(|| a.clip.start_beats.total_cmp(&b.clip.start_beats))
                .then_with(|| a.clip.id.cmp(&b.clip.id))
        });
        let count = items.len();
        self.audio_event_clipboard = Some(Clipboard {
            anchor_track_id,
            anchor_beat,
            anchor_group_id: selected_group_id,
            items,
        });
        self.status_message = format!("Copied {count} audio event(s)");
    }

    pub(super) fn paste_audio_events(&mut self) {
        let base = self.data.transport.position_beats.max(0.0);
        self.paste_audio_events_at(base);
    }

    /// Pastes the copied events at the beats they were copied from.
    pub(super) fn paste_audio_events_at_origin(&mut self) {
        let Some(clipboard) = self.audio_event_clipboard.as_ref() else {
            self.status_message = "Copy audio events before pasting".into();
            return;
        };
        let base = clipboard.anchor_beat;
        self.paste_audio_events_at(base);
    }

    fn paste_audio_events_at(&mut self, base_beats: f64) {
        if self.is_recording() {
            self.status_message = "Stop recording before pasting audio events".into();
            return;
        }
        let paste = match self.build_paste(base_beats) {
            Ok(paste) => paste,
            Err(message) => {
                self.status_message = message;
                return;
            }
        };
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Undo history is unavailable for this project size".into();
            return;
        }
        let Some(new_ids) = self.data.project.paste_audio_clips(&paste) else {
            self.status_message = "Could not paste the selected audio events".into();
            return;
        };
        self.arrange_view_state.audio_events.select_many(new_ids);
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Pasted {} audio event(s); Undo restores the previous arrangement",
            paste.len()
        );
    }

    /// Pastes the clipboard at the cursor and inserts its length first.
    ///
    /// The ripple and the paste are one undo step: the material after the cursor
    /// moves to make room, and the copied events land at the cursor rather than
    /// over what was already there.
    pub(super) fn paste_time(&mut self) {
        if self.is_recording() {
            self.status_message = "Stop recording before pasting audio events".into();
            return;
        }
        let Some(clipboard) = self.audio_event_clipboard.clone() else {
            self.status_message = "Copy audio events before pasting".into();
            return;
        };
        let tempo = self.data.project.tempo_bpm.max(1.0);
        let span = clipboard
            .items
            .iter()
            .map(|item| item.clip.start_beats + item.clip.duration_seconds * tempo / 60.0)
            .fold(f64::NEG_INFINITY, f64::max)
            - clipboard.anchor_beat;
        if !span.is_finite() || span <= 0.0 {
            self.status_message = "The copied events have no length to paste".into();
            return;
        }
        let base = self.data.transport.position_beats.max(0.0);
        let paste = match self.build_paste(base) {
            Ok(paste) => paste,
            Err(message) => {
                self.status_message = message;
                return;
            }
        };
        let mut candidate = self.data.project.clone();
        if candidate.insert_silence_at(base, span).is_none()
            && has_content_overlapping_or_after(&candidate, base)
        {
            self.status_message = "Cannot paste time: a locked event crosses the cursor".into();
            return;
        }
        let Some(new_ids) = candidate.paste_audio_clips(&paste) else {
            self.status_message = "Could not paste the selected audio events".into();
            return;
        };
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Undo history is unavailable for this project size".into();
            return;
        }
        self.project_history.label_next("Paste Time");
        self.data.project = candidate;
        self.arrange_view_state.audio_events.select_many(new_ids);
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Inserted {span:.3} beat(s) and pasted {} audio event(s); Undo restores both",
            paste.len()
        );
    }

    /// Resolves the clipboard's destinations for a paste based at `base_beats`.
    fn build_paste(&self, base_beats: f64) -> Result<Vec<(usize, AudioClip)>, String> {
        let Some(clipboard) = self.audio_event_clipboard.as_ref() else {
            return Err("Copy audio events before pasting".into());
        };
        let selected_track = self
            .data
            .project
            .selected_track
            .map(|index| index as usize)
            .and_then(|index| self.data.project.tracks.get(index));
        let target_group_id = selected_track
            .filter(|track| track.kind == TrackKind::Group)
            .map(|track| track.id)
            .or_else(|| {
                selected_track
                    .is_none()
                    .then_some(clipboard.anchor_group_id)
                    .flatten()
            })
            .filter(|group_id| {
                self.data
                    .project
                    .tracks
                    .iter()
                    .any(|track| track.id == *group_id && track.kind == TrackKind::Group)
            });
        let group_destinations = target_group_id.map(|group_id| {
            self.data
                .project
                .tracks
                .iter()
                .filter(|track| {
                    track.kind == TrackKind::Audio
                        && track_belongs_to_group(&self.data.project, track.id, group_id)
                })
                .map(|track| track.id)
                .collect::<Vec<_>>()
        });
        if target_group_id.is_some() && group_destinations.as_ref().is_none_or(Vec::is_empty) {
            return Err("The destination group has no audio tracks".into());
        }
        let anchor_index = selected_track
            .filter(|track| track.kind == TrackKind::Audio)
            .and_then(|track| {
                self.data
                    .project
                    .tracks
                    .iter()
                    .position(|item| item.id == track.id)
            })
            .or_else(|| {
                self.data
                    .project
                    .tracks
                    .iter()
                    .position(|track| track.id == clipboard.anchor_track_id)
            });
        let Some(anchor_index) = anchor_index.or_else(|| {
            group_destinations
                .as_ref()
                .and_then(|destinations| destinations.first())
                .and_then(|track_id| {
                    self.data
                        .project
                        .tracks
                        .iter()
                        .position(|t| t.id == *track_id)
                })
        }) else {
            return Err("Choose an audio track to paste onto".into());
        };
        let mut paste = Vec::with_capacity(clipboard.items.len());
        for item in &clipboard.items {
            let destination_track_id = if let Some(destinations) = &group_destinations {
                let Some(track_id) = destinations.get(item.track_offset) else {
                    return Err("The destination group does not have enough audio tracks".into());
                };
                Some(*track_id)
            } else {
                None
            };
            let Some(track_index) = destination_track_id
                .and_then(|track_id| {
                    self.data
                        .project
                        .tracks
                        .iter()
                        .position(|t| t.id == track_id)
                })
                .or_else(|| anchor_index.checked_add(item.track_offset))
            else {
                return Err(
                    "The selected destination tracks do not match the copied events".into(),
                );
            };
            let Some(track) = self.data.project.tracks.get(track_index) else {
                return Err(
                    "The selected destination tracks do not match the copied events".into(),
                );
            };
            if track.kind != TrackKind::Audio || track.audio_channels != item.channels {
                return Err("Paste requires audio tracks with matching channel widths".into());
            }
            let mut clip = item.clip.clone();
            clip.start_beats =
                base_beats.max(0.0) + (item.clip.start_beats - clipboard.anchor_beat);
            paste.push((track_index, clip));
        }
        Ok(paste)
    }
}

pub(super) struct Lane {
    pub id: u64,
    pub channels: u8,
    pub audio: bool,
    pub rect: Rect,
    pub take_lanes_visible: bool,
    pub take_lane_count: usize,
    pub take_lane_indices: HashMap<u64, usize>,
    pub automation: Option<Rect>,
    pub color: Color32,
}
pub(super) struct Outcome {
    pub changed: bool,
    pub scroll: f32,
    pub crossfade_requested: bool,
}
pub(super) fn take_lane_layout(
    track: &opentrack_core::project::Track,
    tempo_bpm: f64,
) -> (usize, HashMap<u64, usize>) {
    if track.kind != TrackKind::Audio {
        return (1, HashMap::new());
    }
    let tempo = if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
        tempo_bpm
    } else {
        120.0
    };
    let mut lanes = HashMap::with_capacity(track.clips.len());
    let mut highest_lane = 0usize;
    for (index, clip) in track.clips.iter().enumerate() {
        let clip_end = clip.start_beats + clip.duration_seconds * tempo / 60.0;
        let lane = track.clips[..index]
            .iter()
            .filter(|previous| {
                let previous_end = previous.start_beats + previous.duration_seconds * tempo / 60.0;
                previous.start_beats < clip_end && clip.start_beats < previous_end
            })
            .filter_map(|previous| lanes.get(&previous.id).copied())
            .max()
            .map_or(0, |previous_lane| previous_lane + 1);
        highest_lane = highest_lane.max(lane);
        lanes.insert(clip.id, lane);
    }
    (highest_lane + 1, lanes)
}

fn event_tracks_for_lane(project: &Project, lane: &Lane) -> Vec<u64> {
    let Some(track) = project.tracks.iter().find(|track| track.id == lane.id) else {
        return Vec::new();
    };
    if track.kind != TrackKind::Group || !track.group_collapsed {
        return (track.kind == TrackKind::Audio && lane.audio)
            .then_some(track.id)
            .into_iter()
            .collect();
    }
    project
        .tracks
        .iter()
        .filter(|candidate| {
            candidate.kind == TrackKind::Audio
                && track_belongs_to_group(project, candidate.id, track.id)
        })
        .map(|candidate| candidate.id)
        .collect()
}

/// Time ranges represented by the collapsed group's one virtual audio event.
/// A group split marker creates a piece boundary when any child was split at
/// it, and a cut inferred from child clips requires source-contiguous splits on
/// every child track. Independent clip starts/ends stay hidden inside the
/// continuous group event.
pub(super) fn group_audio_segments(project: &Project, group_id: u64) -> Vec<(f64, f64)> {
    let Some(group) = project
        .tracks
        .iter()
        .find(|track| track.id == group_id && track.kind == TrackKind::Group)
    else {
        return Vec::new();
    };
    let child_tracks = project
        .tracks
        .iter()
        .filter(|track| {
            track.kind == TrackKind::Audio
                && !track.clips.is_empty()
                && track_belongs_to_group(project, track.id, group_id)
        })
        .collect::<Vec<_>>();
    let Some((start, end)) = child_tracks
        .iter()
        .flat_map(|track| {
            track.clips.iter().map(|clip| {
                (
                    clip.start_beats,
                    clip.start_beats + clip.duration_seconds * project.tempo_bpm / 60.0,
                )
            })
        })
        .fold(None::<(f64, f64)>, |bounds, (clip_start, clip_end)| {
            Some(match bounds {
                Some((start, end)) => (start.min(clip_start), end.max(clip_end)),
                None => (clip_start, clip_end),
            })
        })
    else {
        return Vec::new();
    };
    if !start.is_finite() || !end.is_finite() || end <= start {
        return Vec::new();
    }

    const CUT_SCALE: f64 = 1_000_000.0;
    let is_split_boundary = |track: &&Track, beat: f64| {
        let mut clips = track.clips.iter().collect::<Vec<_>>();
        clips.sort_by(|a, b| a.start_beats.total_cmp(&b.start_beats));
        clips.windows(2).any(|pair| {
            let left = pair[0];
            let right = pair[1];
            let left_end = left.start_beats + left.duration_seconds * project.tempo_bpm / 60.0;
            let source_end = left.source_offset_seconds + left.duration_seconds;
            (left_end - beat).abs() < 1e-5
                && (right.start_beats - beat).abs() < 1e-5
                && left.source_path == right.source_path
                && (right.source_offset_seconds - source_end).abs()
                    <= 1.0 / project.sample_rate_hz.max(1) as f64
        })
    };
    let mut explicit_cuts = group.group_cut_beats.iter().copied().collect::<Vec<_>>();
    let mut inferred_cuts = Vec::new();
    for track in &child_tracks {
        let mut clips = track.clips.iter().collect::<Vec<_>>();
        clips.sort_by(|a, b| a.start_beats.total_cmp(&b.start_beats));
        for pair in clips.windows(2) {
            let left = pair[0];
            let right = pair[1];
            let beat = left.start_beats + left.duration_seconds * project.tempo_bpm / 60.0;
            let source_end = left.source_offset_seconds + left.duration_seconds;
            if (right.start_beats - beat).abs() < 1e-5
                && left.source_path == right.source_path
                && (right.source_offset_seconds - source_end).abs()
                    <= 1.0 / project.sample_rate_hz.max(1) as f64
            {
                inferred_cuts.push(beat);
            }
        }
    }
    // A group cut is recorded when the group itself is cut, and every clip that
    // crossed it was split. Requiring a boundary on *every* child track dropped
    // the cut as soon as one child had no audio at that beat, which happens
    // routinely once takes are stacked: the group then looked uncut while the
    // children were. Keep the cut when any child was actually split at it.
    explicit_cuts.retain(|beat| {
        beat.is_finite()
            && *beat > start + 1e-6
            && *beat < end - 1e-6
            && child_tracks.iter().any(|track| {
                track.clips.iter().any(|clip| {
                    let clip_end =
                        clip.start_beats + clip.duration_seconds * project.tempo_bpm / 60.0;
                    (clip.start_beats - beat).abs() < 1e-5 || (clip_end - beat).abs() < 1e-5
                })
            })
    });
    inferred_cuts.retain(|beat| {
        beat.is_finite()
            && *beat > start + 1e-6
            && *beat < end - 1e-6
            && child_tracks
                .iter()
                .all(|track| is_split_boundary(track, *beat))
    });
    let mut cuts = explicit_cuts
        .into_iter()
        .chain(inferred_cuts)
        .map(|beat| (beat * CUT_SCALE).round() / CUT_SCALE)
        .collect::<Vec<_>>();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let mut segments = Vec::with_capacity(cuts.len() + 1);
    let mut segment_start = start;
    for cut in cuts {
        segments.push((segment_start, cut));
        segment_start = cut;
    }
    segments.push((segment_start, end));
    segments
}

fn group_clip_fade_ratios(clips: &[(u64, AudioClip)]) -> (f32, f32) {
    let mut fade_in = 0.0f64;
    let mut fade_out = 0.0f64;
    let count = clips.len();
    for (_, clip) in clips {
        fade_in += (clip.fade_in_seconds / clip.duration_seconds.max(1e-9)) as f64;
        fade_out += (clip.fade_out_seconds / clip.duration_seconds.max(1e-9)) as f64;
    }
    if count == 0 {
        (0.0, 0.0)
    } else {
        (
            (fade_in / count as f64).clamp(0.0, 1.0) as f32,
            (fade_out / count as f64).clamp(0.0, 1.0) as f32,
        )
    }
}

pub(super) fn group_segment_proxy(
    project: &Project,
    group_id: u64,
    start: f64,
    end: f64,
) -> Option<AudioClip> {
    let clips = clips_for_group_segment(project, group_id, start, end);
    group_segment_proxy_from_clips(project, group_id, start, end, &clips)
}

pub(super) fn group_segment_proxy_from_clips(
    project: &Project,
    group_id: u64,
    start: f64,
    end: f64,
    clips: &[(u64, AudioClip)],
) -> Option<AudioClip> {
    let (_, mut clip) = clips.first()?.clone();
    let group = project.tracks.iter().find(|track| track.id == group_id)?;
    let duration = (end - start).max(0.0) * 60.0 / project.tempo_bpm.max(1.0);
    let (fade_in, fade_out) = group_clip_fade_ratios(clips);
    clip.start_beats = start;
    clip.duration_seconds = duration;
    clip.fade_in_seconds = duration * fade_in as f64;
    clip.fade_out_seconds = duration * fade_out as f64;
    clip.source_offset_seconds = 0.0;
    clip.source_fades.clear();
    clip.name = group.name.clone();
    Some(clip)
}

fn clips_for_group_segment(
    project: &Project,
    group_id: u64,
    start: f64,
    end: f64,
) -> Vec<(u64, AudioClip)> {
    let seconds_per_beat = 60.0 / project.tempo_bpm.max(1.0);
    project
        .tracks
        .iter()
        .filter(|track| {
            track.kind == TrackKind::Audio && track_belongs_to_group(project, track.id, group_id)
        })
        .flat_map(|track| {
            track.clips.iter().filter_map(move |clip| {
                let clip_end = clip.start_beats + clip.duration_seconds / seconds_per_beat;
                (clip.start_beats >= start - 1e-6 && clip_end <= end + 1e-6)
                    .then(|| (track.id, clip.clone()))
            })
        })
        .collect()
}

fn track_belongs_to_group(project: &Project, track_id: u64, group_id: u64) -> bool {
    let mut parent_id = project
        .tracks
        .iter()
        .find(|track| track.id == track_id)
        .and_then(|track| track.parent_group_id);
    for _ in 0..2 {
        let Some(parent) = parent_id else {
            return false;
        };
        if parent == group_id {
            return true;
        }
        parent_id = project
            .tracks
            .iter()
            .find(|track| track.id == parent)
            .and_then(|track| track.parent_group_id);
    }
    false
}

fn event_rect(c: &AudioClip, lane: &Lane, origin: f32, px: f32, tempo: f64) -> Rect {
    let lane_height = if lane.take_lanes_visible {
        lane.rect.height() / lane.take_lane_count.max(1) as f32
    } else {
        lane.rect.height()
    };
    let lane_index = if lane.take_lanes_visible {
        lane.take_lane_indices.get(&c.id).copied().unwrap_or(0) as f32
    } else {
        0.0
    };
    Rect::from_min_size(
        Pos2::new(
            origin + c.start_beats as f32 * px,
            lane.rect.top() + lane_index * lane_height,
        ),
        Vec2::new((c.duration_seconds * tempo / 60.0) as f32 * px, lane_height),
    )
}
fn wave_rect(r: Rect) -> Rect {
    Rect::from_min_max(
        r.min,
        Pos2::new(r.right(), (r.bottom() - 13.0).max(r.top() + 1.0)),
    )
}
fn gain_y(c: &AudioClip, r: Rect) -> f32 {
    let db = 20.0 * c.gain.max(0.000001).log10();
    r.top() + 3.0 + (-db / 24.0).clamp(0.0, 1.0) * (r.height() - 6.0).max(0.0)
}
fn handles(c: &AudioClip, r: Rect) -> [(Tool, Pos2); 5] {
    let wave = wave_rect(r);
    let width = r.width();
    [
        (Tool::Left, Pos2::new(r.left() + 3.0, r.bottom() - 3.0)),
        (Tool::Right, Pos2::new(r.right() - 3.0, r.bottom() - 3.0)),
        (
            Tool::FadeIn,
            Pos2::new(
                r.left() + width * (c.fade_in_seconds / c.duration_seconds) as f32 + 3.0,
                wave.top() + 3.0,
            ),
        ),
        (
            Tool::FadeOut,
            Pos2::new(
                r.right() - width * (c.fade_out_seconds / c.duration_seconds) as f32 - 3.0,
                wave.top() + 3.0,
            ),
        ),
        (Tool::Gain, Pos2::new(r.center().x, gain_y(c, wave))),
    ]
}
fn hit(c: &AudioClip, r: Rect, pos: Pos2, selected: bool) -> Tool {
    if selected {
        for (tool, p) in handles(c, r) {
            if Rect::from_center_size(p, Vec2::splat(12.0)).contains(pos) {
                return tool;
            }
        }
    }
    Tool::Move
}
fn cursor(tool: Tool) -> egui::CursorIcon {
    match tool {
        Tool::Move => egui::CursorIcon::Move,
        Tool::Gain => egui::CursorIcon::ResizeVertical,
        _ => egui::CursorIcon::ResizeHorizontal,
    }
}
/// True when anything on the timeline crosses or follows `beat`.
///
/// A crossing event matters as much as a later one: inserting time through it
/// splits it, so a locked crossing event must block the ripple.
fn has_content_overlapping_or_after(project: &Project, beat: f64) -> bool {
    let tempo = project.tempo_bpm.max(1.0);
    project.tracks.iter().any(|track| {
        track.clips.iter().any(|clip| {
            clip.start_beats >= beat - 1e-9
                || clip.start_beats + clip.duration_seconds * tempo / 60.0 > beat + 1e-9
        }) || track.midi_parts.iter().any(|part| {
            part.start_beats >= beat - 1e-9 || part.start_beats + part.duration_beats > beat + 1e-9
        }) || track
            .automation_lanes
            .iter()
            .any(|lane| lane.points.iter().any(|point| point.beat >= beat - 1e-9))
    }) || project
        .markers
        .iter()
        .any(|marker| marker.beat >= beat - 1e-9)
}

fn snap(value: f64, grid: Option<f64>) -> f64 {
    grid.map_or(value.max(0.0), |step| {
        crate::timeline::snap_beats_to_grid(value, step)
    })
}

/// Snaps to a nearby event edge or the cursor when one is closer than the grid.
///
/// `tolerance_beats` is the same distance in beats as a few pixels, so the
/// preference does not change with zoom. A target wins only when it is strictly
/// closer than the grid result, so the grid stays predictable.
fn snap_with_targets(value: f64, grid: Option<f64>, targets: &[f64], tolerance_beats: f64) -> f64 {
    let value = value.max(0.0);
    if targets.is_empty() || !tolerance_beats.is_finite() || tolerance_beats <= 0.0 {
        return snap(value, grid);
    }
    let nearest = targets
        .iter()
        .copied()
        .filter(|target| target.is_finite() && *target >= 0.0)
        .min_by(|left, right| (value - left).abs().total_cmp(&(value - right).abs()));
    let Some(nearest) = nearest else {
        return snap(value, grid);
    };
    let distance = (value - nearest).abs();
    if distance > tolerance_beats {
        return snap(value, grid);
    }
    let grid_snapped = snap(value, grid);
    if (value - grid_snapped).abs() <= distance {
        return grid_snapped;
    }
    nearest
}
fn update(
    g: &mut Gesture,
    pos: Pos2,
    origin: f32,
    px: f32,
    tempo: f64,
    grid: Option<f64>,
    lanes: &[Lane],
    min_seconds: f64,
    snap_targets: &[f64],
    snap_tolerance_beats: f64,
) {
    let beats = ((pos.x - origin) / px) as f64;
    let original = &g.original;
    let mut c = original.clone();
    g.target = Some(g.track);
    match g.tool {
        Tool::Move => {
            c.start_beats = snap_with_targets(
                beats - g.grab_beats,
                grid,
                snap_targets,
                snap_tolerance_beats,
            );
            let source = lanes.iter().find(|l| l.id == g.track);
            g.target = lanes
                .iter()
                .find(|l| {
                    l.rect.y_range().contains(pos.y)
                        && l.audio
                        && source.is_some_and(|s| s.channels == l.channels)
                })
                .map(|l| l.id);
        }
        Tool::Left => {
            let end = original.start_beats + original.duration_seconds * tempo / 60.0;
            let earliest =
                (original.start_beats - original.source_offset_seconds * tempo / 60.0).max(0.0);
            c.start_beats = snap_with_targets(beats, grid, snap_targets, snap_tolerance_beats)
                .clamp(earliest, (end - min_seconds * tempo / 60.0).max(earliest));
            let delta = (c.start_beats - original.start_beats) * 60.0 / tempo;
            c.source_offset_seconds = (original.source_offset_seconds + delta).max(0.0);
            c.duration_seconds = (original.duration_seconds - delta).max(min_seconds);
        }
        Tool::Right => {
            let end = snap_with_targets(beats, grid, snap_targets, snap_tolerance_beats);
            c.duration_seconds = ((end - c.start_beats) * 60.0 / tempo).clamp(
                min_seconds,
                (g.source_duration - c.source_offset_seconds).max(min_seconds),
            );
        }
        Tool::FadeIn => {
            c.fade_in_seconds = ((beats - c.start_beats) * 60.0 / tempo)
                .clamp(0.0, (c.duration_seconds - c.fade_out_seconds).max(0.0))
        }
        Tool::FadeOut => {
            c.fade_out_seconds = (c.duration_seconds - (beats - c.start_beats) * 60.0 / tempo)
                .clamp(0.0, (c.duration_seconds - c.fade_in_seconds).max(0.0))
        }
        Tool::Gain => {
            let mut db = (20.0 * original.gain.max(0.000001).log10() - (pos.y - g.press.y) * 0.25)
                .clamp(-60.0, 12.0412);
            if db.abs() < 0.75 {
                db = 0.0;
            }
            c.gain = 10.0_f32.powf(db / 20.0).min(4.0);
        }
    }
    c.fade_in_seconds = c.fade_in_seconds.min(c.duration_seconds);
    c.fade_out_seconds = c
        .fade_out_seconds
        .min((c.duration_seconds - c.fade_in_seconds).max(0.0));
    g.candidate = c;
}
/// Project event waveforms use a slightly hotter display scale so typical
/// recorded peaks fill more of the lane. It never changes gain, envelopes,
/// analysis, or the source media.
const TRACK_WAVEFORM_DISPLAY_GAIN: f32 = 1.75;

fn paint(
    ui: &egui::Ui,
    c: &AudioClip,
    lane: &Lane,
    origin: f32,
    px: f32,
    tempo: f64,
    clip: Rect,
    sources: &HashMap<PathBuf, AudioSource>,
    selected: bool,
    ghost: bool,
) {
    let r = event_rect(c, lane, origin, px, tempo);
    if !r.intersects(clip) || r.width() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(clip);
    // A muted event stays readable but clearly inactive, matching its silence.
    let alpha = if ghost {
        0.28
    } else if c.muted || !c.take_active {
        0.45
    } else {
        1.0
    };
    painter.rect_filled(r, 0.0, lane.color.gamma_multiply(alpha));
    let wave = wave_rect(r);
    if let Some(source) = sources
        .get(super::sample_editor::playback_path(c))
        .or_else(|| sources.get(&c.source_path))
    {
        super::waveform_view::draw(
            &painter,
            wave,
            clip,
            source.waveform().channels as usize,
            c.duration_seconds,
            |a, b, n, ch| {
                let mut values = source.waveform_columns(
                    a + c.source_offset_seconds,
                    b + c.source_offset_seconds,
                    n,
                    ch,
                );
                for (i, v) in values.iter_mut().enumerate() {
                    let t = a + (b - a) * (i as f64 + 0.5) / n as f64;
                    let gain = c.gain
                        * opentrack_audio::event_envelope(
                            t,
                            c.duration_seconds,
                            c.fade_in_seconds,
                            c.fade_out_seconds,
                        )
                        * c.source_fades
                            .iter()
                            .map(|f| f.gain_at(t + c.source_offset_seconds))
                            .product::<f32>()
                        * alpha;
                    v[0] = (v[0] * gain * TRACK_WAVEFORM_DISPLAY_GAIN).clamp(-1.0, 1.0);
                    v[1] = (v[1] * gain * TRACK_WAVEFORM_DISPLAY_GAIN).clamp(-1.0, 1.0);
                }
                values
            },
        );
    }
    paint_event_chrome(&painter, c, r, clip, lane.color, selected, ghost);
    if !ghost && r.width() > 18.0 {
        // State marks sit in the corner and never move the event itself.
        let mark = if c.locked {
            Some("L")
        } else if c.muted {
            Some("M")
        } else if !c.take_active {
            Some("T")
        } else {
            None
        };
        if let Some(mark) = mark {
            painter.with_clip_rect(clip).text(
                r.right_top() + Vec2::new(-4.0, 0.0),
                Align2::RIGHT_TOP,
                mark,
                FontId::proportional(11.0),
                Color32::WHITE.gamma_multiply(0.8),
            );
        }
        if c.group.is_some() && r.width() > 40.0 {
            painter.with_clip_rect(clip).text(
                r.right_top() + Vec2::new(-16.0, 0.0),
                Align2::RIGHT_TOP,
                "G",
                FontId::proportional(11.0),
                Color32::WHITE.gamma_multiply(0.6),
            );
        }
    }
    if c.locked && !ghost {
        // A locked event reads as set aside: a dashed outline over the event.
        painter.with_clip_rect(clip).rect_stroke(
            r.shrink(2.0),
            0.0,
            Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.45)),
            egui::StrokeKind::Inside,
        );
    }
}

pub(super) fn paint_group_segment_controls(
    ui: &egui::Ui,
    clip: &AudioClip,
    rect: Rect,
    visible: Rect,
    color: Color32,
) {
    paint_event_chrome(
        &ui.painter().with_clip_rect(visible),
        clip,
        rect,
        visible,
        color,
        true,
        false,
    );
}

fn paint_event_chrome(
    painter: &egui::Painter,
    c: &AudioClip,
    r: Rect,
    clip: Rect,
    color: Color32,
    selected: bool,
    ghost: bool,
) {
    let alpha = if ghost { 0.28 } else { 1.0 };
    let band = Rect::from_min_max(Pos2::new(r.left(), (r.bottom() - 13.0).max(r.top())), r.max);
    painter.rect_filled(
        band,
        0.0,
        if selected {
            Color32::BLACK.gamma_multiply(alpha)
        } else {
            color.gamma_multiply(0.8 * alpha)
        },
    );
    let wave = wave_rect(r);
    let ink = if selected {
        Color32::WHITE
    } else {
        project_controls::label_ink(color)
    };
    painter.with_clip_rect(band.intersect(clip)).text(
        band.left_top() + Vec2::new(4.0, 0.0),
        Align2::LEFT_TOP,
        &c.name,
        FontId::proportional(11.0),
        ink.gamma_multiply(alpha),
    );
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(
            1.0,
            if selected {
                Color32::WHITE
            } else {
                Color32::BLACK
            }
            .gamma_multiply(alpha),
        ),
        egui::StrokeKind::Inside,
    );
    if selected && !ghost {
        let blue = Color32::WHITE;
        let gain = gain_y(c, wave);
        painter.line_segment(
            [Pos2::new(r.left(), gain), Pos2::new(r.right(), gain)],
            Stroke::new(1.0, blue),
        );
        painter.line_segment(
            [
                Pos2::new(r.left(), wave.bottom()),
                Pos2::new(
                    r.left() + r.width() * (c.fade_in_seconds / c.duration_seconds) as f32,
                    wave.top(),
                ),
            ],
            Stroke::new(1.0, blue),
        );
        painter.line_segment(
            [
                Pos2::new(
                    r.right() - r.width() * (c.fade_out_seconds / c.duration_seconds) as f32,
                    wave.top(),
                ),
                Pos2::new(r.right(), wave.bottom()),
            ],
            Stroke::new(1.0, blue),
        );
        for (_, pos) in handles(c, r) {
            painter.rect_filled(
                Rect::from_center_size(pos, Vec2::splat(6.0)),
                0.0,
                Color32::WHITE,
            );
        }
    }
}
pub(super) fn show_with_tool(
    ui: &mut egui::Ui,
    project: &mut Project,
    sources: &HashMap<PathBuf, AudioSource>,
    lanes: &[Lane],
    visible: Rect,
    origin: f32,
    px: f32,
    grid: Option<f64>,
    track_scroll_y: f32,
    state: &mut State,
    enabled: bool,
    edit_tool: &mut EditTool,
    automation_follows_events: bool,
    snap_targets: &[f64],
) -> Outcome {
    toolbox(ui, visible, &mut state.toolbox, edit_tool);
    let mut split = None;
    let mut group_split = None;
    let mut cut_preview = None;
    let mut out = Outcome {
        changed: false,
        scroll: 0.0,
        crossfade_requested: false,
    };
    let tempo = project.tempo_bpm;
    let mut reset_gain = None;
    let mut rename_done = None;
    let input = ui.input(|i| {
        (
            i.pointer.hover_pos(),
            i.pointer.primary_down(),
            i.pointer.button_released(egui::PointerButton::Primary),
            i.key_pressed(egui::Key::Escape),
            i.pointer.button_pressed(egui::PointerButton::Primary),
            i.modifiers,
        )
    });
    if enabled
        && *edit_tool == EditTool::Select
        && state.gesture.is_none()
        && state.group_gesture.is_none()
        && state.marquee.is_none()
        && input.4
        && let Some(pos) = input.0.filter(|p| visible.contains(*p))
    {
        let over_content = lanes.iter().any(|lane| {
            lane.automation.is_some_and(|r| r.contains(pos))
                || project
                    .tracks
                    .iter()
                    .find(|track| track.id == lane.id)
                    .is_some_and(|track| {
                        matches!(track.kind, TrackKind::Midi | TrackKind::Instrument)
                            && lane.rect.contains(pos)
                            && track.midi_parts.iter().any(|part| {
                                let left = origin + part.start_beats as f32 * px;
                                let right =
                                    origin + (part.start_beats + part.duration_beats) as f32 * px;
                                Rect::from_min_max(
                                    Pos2::new(left, lane.rect.top() + 6.0),
                                    Pos2::new(right, lane.rect.bottom() - 6.0),
                                )
                                .contains(pos)
                            })
                    })
                || event_tracks_for_lane(project, lane).iter().any(|track_id| {
                    project
                        .tracks
                        .iter()
                        .find(|track| track.id == *track_id)
                        .is_some_and(|track| {
                            track
                                .clips
                                .iter()
                                .any(|clip| event_rect(clip, lane, origin, px, tempo).contains(pos))
                        })
                })
        });
        if !over_content {
            state.marquee = Some(MarqueeGesture {
                press: pos,
                anchor_beat: ((pos.x - origin) / px) as f64,
                anchor_content_y: pos.y + track_scroll_y,
                initial_primary: state.selected,
                initial_extra: state.extra_selected.clone(),
                modifiers: input.5,
                active: false,
            });
        }
    }
    if !enabled {
        state.gesture = None;
        state.group_gesture = None;
        state.marquee = None;
    }
    if let Some(mut marquee) = state.marquee.take() {
        if input.3 {
            state.selected = marquee.initial_primary;
            state.extra_selected = marquee.initial_extra;
        } else if let Some(pos) = input.0 {
            marquee.active |= pos.distance(marquee.press) >= 3.0;
            if marquee.active {
                let anchor = Pos2::new(
                    origin + marquee.anchor_beat as f32 * px,
                    marquee.anchor_content_y - track_scroll_y,
                );
                let rect = Rect::from_two_pos(anchor, pos).intersect(visible);
                let mut hits = HashSet::new();
                for lane in lanes.iter().filter(|lane| lane.audio) {
                    for track_id in event_tracks_for_lane(project, lane) {
                        if let Some(track) =
                            project.tracks.iter().find(|track| track.id == track_id)
                        {
                            for clip in &track.clips {
                                if event_rect(clip, lane, origin, px, tempo).intersects(rect) {
                                    hits.insert(clip.id);
                                }
                            }
                        }
                    }
                }
                let mut selected = HashSet::new();
                if marquee.modifiers.shift || marquee.modifiers.ctrl {
                    selected.extend(marquee.initial_primary);
                    selected.extend(marquee.initial_extra.iter().copied());
                }
                if marquee.modifiers.ctrl {
                    for id in hits {
                        if !selected.insert(id) {
                            selected.remove(&id);
                        }
                    }
                } else {
                    selected.extend(hits);
                }
                state.selected = selected.iter().next().copied();
                state.extra_selected = selected;
                if let Some(primary) = state.selected {
                    state.extra_selected.remove(&primary);
                }
                let painter = ui.painter().with_clip_rect(visible);
                painter.rect_filled(rect, 0.0, Color32::WHITE.gamma_multiply(0.10));
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
                if input.1 {
                    let edge = 32.0;
                    let strength = if pos.x < visible.left() + edge {
                        -((visible.left() + edge - pos.x) / edge).clamp(0.0, 1.0)
                    } else if pos.x > visible.right() - edge {
                        ((pos.x - visible.right() + edge) / edge).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    out.scroll = strength * 600.0 * ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
                }
            } else if input.2 {
                state.selected = None;
                state.extra_selected.clear();
                state.selected_group_segment = None;
                state.row_focus = false;
            }
            if input.1 {
                state.marquee = Some(marquee);
                ui.ctx().request_repaint();
            }
        }
    }
    if let Some(mut g) = state.gesture.take() {
        if input.3 {
            if g.tool != Tool::Move {
                out.changed |= project.edit_audio_clip(g.track, g.track, g.original.clone());
                for (track, clip) in &g.peers {
                    out.changed |= project.edit_audio_clip(*track, *track, clip.clone());
                }
            }
        } else if let Some(pos) = input.0 {
            update(
                &mut g,
                pos,
                origin,
                px,
                tempo,
                grid,
                lanes,
                1.0 / project.sample_rate_hz.max(1) as f64,
                snap_targets,
                8.0 / px.max(1.0) as f64,
            );
            ui.ctx().set_cursor_icon(if g.target.is_none() {
                egui::CursorIcon::NotAllowed
            } else {
                cursor(g.tool)
            });
            if g.tool != Tool::Move || input.2 {
                if let Some(target) = g.target {
                    let changed = project.edit_audio_clip(g.track, target, g.candidate.clone());
                    if changed && g.tool == Tool::Move {
                        project.selected_track = project
                            .tracks
                            .iter()
                            .position(|t| t.id == target)
                            .map(|i| i as u32);
                    }
                    out.changed |= changed;
                    for (track, original) in &g.peers {
                        let mut clip = original.clone();
                        match g.tool {
                            Tool::Gain => {
                                clip.gain = if g.candidate.gain == 0.0 {
                                    0.0
                                } else {
                                    (clip.gain * (g.candidate.gain / g.original.gain.max(0.000001)))
                                        .clamp(0.0, 4.0)
                                }
                            }
                            Tool::FadeIn => {
                                clip.fade_in_seconds = (clip.fade_in_seconds
                                    + g.candidate.fade_in_seconds
                                    - g.original.fade_in_seconds)
                                    .clamp(
                                        0.0,
                                        (clip.duration_seconds - clip.fade_out_seconds).max(0.0),
                                    )
                            }
                            Tool::FadeOut => {
                                clip.fade_out_seconds = (clip.fade_out_seconds
                                    + g.candidate.fade_out_seconds
                                    - g.original.fade_out_seconds)
                                    .clamp(
                                        0.0,
                                        (clip.duration_seconds - clip.fade_in_seconds).max(0.0),
                                    )
                            }
                            Tool::Left => {
                                let requested = g.candidate.source_offset_seconds
                                    - g.original.source_offset_seconds;
                                let earliest = (clip.start_beats
                                    - clip.source_offset_seconds * tempo / 60.0)
                                    .max(0.0);
                                let start =
                                    (clip.start_beats + requested * tempo / 60.0).max(earliest);
                                let applied = (start - clip.start_beats) * 60.0 / tempo;
                                clip.start_beats = start;
                                clip.source_offset_seconds =
                                    (clip.source_offset_seconds + applied).max(0.0);
                                clip.duration_seconds = (clip.duration_seconds - applied)
                                    .max(1.0 / project.sample_rate_hz.max(1) as f64);
                            }
                            Tool::Right => {
                                let requested =
                                    g.candidate.duration_seconds - g.original.duration_seconds;
                                let source_duration = sources
                                    .get(&clip.source_path)
                                    .map(|source| source.duration_seconds())
                                    .unwrap_or(clip.duration_seconds + clip.source_offset_seconds);
                                let minimum = 1.0 / project.sample_rate_hz.max(1) as f64;
                                clip.duration_seconds = (clip.duration_seconds + requested).clamp(
                                    minimum,
                                    (source_duration - clip.source_offset_seconds).max(minimum),
                                );
                            }
                            _ => {}
                        }
                        clip.fade_in_seconds = clip.fade_in_seconds.min(clip.duration_seconds);
                        clip.fade_out_seconds = clip
                            .fade_out_seconds
                            .min((clip.duration_seconds - clip.fade_in_seconds).max(0.0));
                        out.changed |= project.edit_audio_clip(*track, *track, clip);
                    }
                }
            }
            if input.1 {
                if matches!(g.tool, Tool::Move | Tool::Left | Tool::Right) {
                    let edge = 32.0;
                    let strength = if pos.x < visible.left() + edge {
                        -((visible.left() + edge - pos.x) / edge).clamp(0.0, 1.0)
                    } else if pos.x > visible.right() - edge {
                        ((pos.x - visible.right() + edge) / edge).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    out.scroll = strength * 600.0 * ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
                }
                if g.tool == Tool::Gain {
                    egui::Tooltip::always_open(
                        ui.ctx().clone(),
                        ui.layer_id(),
                        Id::new("event-gain-tooltip"),
                        egui::PopupAnchor::Position(pos + Vec2::new(14.0, 14.0)),
                    )
                    .show(|ui| {
                        ui.label(format!(
                            "{:.2} dB",
                            20.0 * g.candidate.gain.max(0.000001).log10()
                        ));
                    });
                }
                state.gesture = Some(g);
                ui.ctx().request_repaint();
            }
        }
    }
    if let Some(mut group) = state.group_gesture.take() {
        if input.3 {
            // Group previews do not mutate the Project before release.
        } else if let Some(pos) = input.0 {
            let anchor = group
                .members
                .iter()
                .find(|member| member.original.id == group.anchor_id)
                .cloned();
            if let Some(anchor) = anchor {
                let pointer_beat = ((pos.x - origin) / px) as f64;
                let snapped_start = snap(pointer_beat - group.grab_beats, grid);
                let minimum_start = group
                    .members
                    .iter()
                    .map(|member| member.original.start_beats)
                    .fold(f64::INFINITY, f64::min);
                let delta = (snapped_start - anchor.original.start_beats)
                    .max(-minimum_start)
                    .max(-anchor.original.start_beats);
                if group.group_master {
                    group.target_valid = true;
                    let fade_delta = (pointer_beat - group.grab_beats) * 60.0 / tempo;
                    let gain_db = (-(pos.y - group.press.y) * 0.25).clamp(-72.0, 12.0412);
                    let gain_scale = 10.0_f32.powf(gain_db / 20.0);
                    let trim_delta =
                        if let Some((segment_start, segment_end)) = group.segment_bounds {
                            match group.tool {
                                Tool::Left => snap(pointer_beat, grid) - segment_start,
                                Tool::Right => snap(pointer_beat, grid) - segment_end,
                                _ => 0.0,
                            }
                        } else {
                            0.0
                        };
                    for member in &mut group.members {
                        member.candidate = member.original.clone();
                        match group.tool {
                            Tool::Move => {
                                member.candidate.start_beats = member.original.start_beats + delta;
                            }
                            Tool::FadeIn => {
                                let fade_delta = (pointer_beat - group.grab_beats) * 60.0 / tempo;
                                member.candidate.fade_in_seconds =
                                    (member.original.fade_in_seconds + fade_delta).clamp(
                                        0.0,
                                        (member.original.duration_seconds
                                            - member.original.fade_out_seconds)
                                            .max(0.0),
                                    );
                            }
                            Tool::FadeOut => {
                                member.candidate.fade_out_seconds =
                                    (member.original.fade_out_seconds - fade_delta).clamp(
                                        0.0,
                                        (member.original.duration_seconds
                                            - member.original.fade_in_seconds)
                                            .max(0.0),
                                    );
                            }
                            Tool::Gain => {
                                member.candidate.gain =
                                    (member.original.gain * gain_scale).clamp(0.0, 4.0);
                            }
                            Tool::Left => {
                                if group.segment_bounds.is_some_and(|(start, _)| {
                                    (member.original.start_beats - start).abs() > 1e-5
                                }) {
                                    member.target = member.track;
                                    continue;
                                }
                                let earliest = (member.original.start_beats
                                    - member.original.source_offset_seconds * tempo / 60.0)
                                    .max(0.0);
                                let start =
                                    (member.original.start_beats + trim_delta).max(earliest);
                                let applied = (start - member.original.start_beats) * 60.0 / tempo;
                                member.candidate.start_beats = start;
                                member.candidate.source_offset_seconds =
                                    (member.original.source_offset_seconds + applied).max(0.0);
                                member.candidate.duration_seconds =
                                    (member.original.duration_seconds - applied)
                                        .max(1.0 / project.sample_rate_hz.max(1) as f64);
                            }
                            Tool::Right => {
                                if group.segment_bounds.is_some_and(|(_, end)| {
                                    (member.original.start_beats
                                        + member.original.duration_seconds * tempo / 60.0
                                        - end)
                                        .abs()
                                        > 1e-5
                                }) {
                                    member.target = member.track;
                                    continue;
                                }
                                let source_duration = sources
                                    .get(sample_editor::playback_path(&member.original))
                                    .or_else(|| sources.get(&member.original.source_path))
                                    .map(|source| source.duration_seconds())
                                    .unwrap_or(
                                        member.original.duration_seconds
                                            + member.original.source_offset_seconds,
                                    );
                                let minimum = 1.0 / project.sample_rate_hz.max(1) as f64;
                                member.candidate.duration_seconds =
                                    (member.original.duration_seconds + trim_delta * 60.0 / tempo)
                                        .clamp(
                                            minimum,
                                            (source_duration
                                                - member.original.source_offset_seconds)
                                                .max(minimum),
                                        );
                            }
                        }
                        member.candidate.fade_in_seconds = member
                            .candidate
                            .fade_in_seconds
                            .min(member.candidate.duration_seconds);
                        member.candidate.fade_out_seconds = member.candidate.fade_out_seconds.min(
                            (member.candidate.duration_seconds - member.candidate.fade_in_seconds)
                                .max(0.0),
                        );
                        member.target = member.track;
                    }
                } else {
                    let target_row = lanes
                        .iter()
                        .position(|lane| lane.rect.y_range().contains(pos.y));
                    let row_delta = target_row.map(|row| row as isize - group.anchor_row as isize);
                    group.target_valid = row_delta.is_some();
                    if let Some(row_delta) = row_delta {
                        for member in &mut group.members {
                            member.candidate = member.original.clone();
                            member.candidate.start_beats = member.original.start_beats + delta;
                            member.target = lanes
                                .get(
                                    member
                                        .row
                                        .checked_add_signed(row_delta)
                                        .unwrap_or(usize::MAX),
                                )
                                .filter(|lane| {
                                    lane.audio
                                        && lane.channels
                                            == project
                                                .tracks
                                                .iter()
                                                .find(|track| track.id == member.track)
                                                .map_or(0, |track| track.audio_channels)
                                })
                                .map(|lane| lane.id)
                                .unwrap_or(0);
                            group.target_valid &= member.target != 0;
                        }
                    }
                }
            }
            if input.2 && group.target_valid {
                let delta = group
                    .members
                    .iter()
                    .find(|member| member.original.id == group.anchor_id)
                    .map_or(0.0, |member| {
                        member.candidate.start_beats - member.original.start_beats
                    });
                let (start_delta, end_delta) = if group.tool == Tool::Move {
                    (delta, delta)
                } else if let (Some((segment_start, segment_end)), Some(pos)) =
                    (group.segment_bounds, input.0)
                {
                    let pointer_beat = snap(((pos.x - origin) / px) as f64, grid);
                    match group.tool {
                        Tool::Left => (pointer_beat - segment_start, 0.0),
                        Tool::Right => (0.0, pointer_beat - segment_end),
                        _ => (0.0, 0.0),
                    }
                } else {
                    (0.0, 0.0)
                };
                let edits = group
                    .members
                    .iter()
                    .map(|member| (member.track, member.target, member.candidate.clone()))
                    .collect::<Vec<_>>();
                let moved =
                    project.move_audio_clips_with_automation(&edits, automation_follows_events);
                out.changed |= moved;
                if moved
                    && (start_delta.abs() > f64::EPSILON || end_delta.abs() > f64::EPSILON)
                    && let Some((group_id, start_cut, end_cut)) = group.group_segment
                    && let Some(group_track) =
                        project.tracks.iter_mut().find(|track| track.id == group_id)
                {
                    for (boundary, shift) in [(start_cut, start_delta), (end_cut, end_delta)] {
                        let Some(boundary) = boundary else {
                            continue;
                        };
                        if shift.abs() <= f64::EPSILON {
                            continue;
                        }
                        if let Some(cut) = group_track
                            .group_cut_beats
                            .iter_mut()
                            .find(|cut| (**cut - boundary).abs() < 1e-6)
                        {
                            *cut = (*cut + shift).max(0.0);
                        } else {
                            group_track
                                .group_cut_beats
                                .push((boundary + shift).max(0.0));
                        }
                    }
                    group_track.group_cut_beats.sort_by(f64::total_cmp);
                    group_track
                        .group_cut_beats
                        .dedup_by(|a, b| (*a - *b).abs() < 1e-6);
                    out.changed = true;
                }
                if moved
                    && group.group_master
                    && let Some((group_id, _, _)) = group.group_segment
                    && let Some(anchor_member) = group
                        .members
                        .iter()
                        .find(|member| member.original.id == group.anchor_id)
                    && let Some((segment_index, (start, end))) =
                        group_audio_segments(project, group_id)
                            .into_iter()
                            .enumerate()
                            .find(|(_, (start, end))| {
                                anchor_member.candidate.start_beats >= *start - 1e-6
                                    && anchor_member.candidate.start_beats < *end - 1e-6
                            })
                {
                    let ids = clips_for_group_segment(project, group_id, start, end)
                        .into_iter()
                        .map(|(_, clip)| clip.id)
                        .collect::<Vec<_>>();
                    state.select_group_segment(group_id, segment_index, ids);
                }
                if out.changed && !group.group_master {
                    if let Some(anchor_member) = group
                        .members
                        .iter()
                        .find(|member| member.original.id == group.anchor_id)
                    {
                        project.selected_track = project
                            .tracks
                            .iter()
                            .position(|track| track.id == anchor_member.target)
                            .and_then(|index| u32::try_from(index).ok());
                    }
                }
            }
            if input.1 {
                let edge = 32.0;
                let strength = if pos.x < visible.left() + edge {
                    -((visible.left() + edge - pos.x) / edge).clamp(0.0, 1.0)
                } else if pos.x > visible.right() - edge {
                    ((pos.x - visible.right() + edge) / edge).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                out.scroll = strength * 600.0 * ui.input(|i| i.stable_dt).clamp(0.001, 0.05);
                group.members.iter_mut().for_each(|member| {
                    member.candidate.start_beats = member.original.start_beats
                        + (member.candidate.start_beats - member.original.start_beats);
                });
                state.group_gesture = Some(group);
                ui.ctx().request_repaint();
            }
        }
    }
    for lane in lanes.iter().filter(|lane| lane.audio) {
        let group_master = project
            .tracks
            .iter()
            .find(|track| track.id == lane.id)
            .is_some_and(|track| track.kind == TrackKind::Group && track.group_collapsed);
        for owner_track_id in event_tracks_for_lane(project, lane) {
            let Some(track) = project
                .tracks
                .iter()
                .find(|track| track.id == owner_track_id)
            else {
                continue;
            };
            for c in &track.clips {
                // A collapsed group renders one composite event in the
                // project view. Painting descendant clips here leaks their
                // independent starts/ends and cuts into that overview.
                if group_master {
                    continue;
                }
                let selected = !group_master
                    && (state.selected == Some(c.id) || state.extra_selected.contains(&c.id));
                let ghost = state.gesture.as_ref().is_some_and(|g| {
                    g.tool == Tool::Move && g.original.id == c.id && g.target.is_some()
                }) || state
                    .group_gesture
                    .as_ref()
                    .is_some_and(|g| g.members.iter().any(|member| member.original.id == c.id));
                paint(
                    ui, c, lane, origin, px, tempo, visible, sources, selected, ghost,
                );
                // A collapsed group is edited through one group-level hit
                // surface below. Child rectangles are paint-only there; they
                // must not steal a drag and turn it into an ordinary lane edit.
                let r = event_rect(c, lane, origin, px, tempo).intersect(visible);
                if !r.is_positive() || !enabled {
                    continue;
                }
                let response =
                    ui.interact(r, Id::new(("audio-event", c.id)), Sense::click_and_drag());
                if let Some((track, id, text, focus)) = &mut state.rename
                    && *id == c.id
                {
                    let edit_rect = Rect::from_min_max(
                        Pos2::new(r.left() + 2.0, r.bottom() - 16.0),
                        r.right_bottom(),
                    );
                    let edit = ui.put(
                        edit_rect,
                        egui::TextEdit::singleline(&mut *text)
                            .font(FontId::proportional(10.0))
                            .margin(Vec2::ZERO),
                    );
                    let first = *focus;
                    if first {
                        edit.request_focus();
                        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit.id) {
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(text.chars().count()),
                                )));
                            state.store(ui.ctx(), edit.id);
                        }
                        *focus = false;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        rename_done = Some(None);
                    } else if ui.input(|i| i.key_pressed(egui::Key::Enter))
                        || (!first && edit.lost_focus())
                    {
                        rename_done = Some(Some((*track, *id, text.trim().to_owned())));
                    }
                }
                if response.clicked_by(egui::PointerButton::Primary) {
                    if *edit_tool == EditTool::Select
                        && let Some(pos) = input.0.filter(|p| p.y >= r.bottom() - 13.0)
                    {
                        let time = ui.input(|i| i.time);
                        let delay = ui.ctx().options(|o| o.input_options.max_double_click_delay);
                        if !group_master
                            && state.last_name_click.is_some_and(|(id, before, point)| {
                                id == c.id && time - before < delay && point.distance(pos) < 6.0
                            })
                        {
                            state.rename = Some((owner_track_id, c.id, c.name.clone(), true));
                            state.last_name_click = None;
                        } else {
                            state.last_name_click = Some((c.id, time, pos));
                        }
                    }
                    state.row_focus = false;
                    project.selected_midi_part_id = None;
                    if group_master {
                        let ids = event_tracks_for_lane(project, lane)
                            .into_iter()
                            .flat_map(|track_id| {
                                project
                                    .tracks
                                    .iter()
                                    .find(|track| track.id == track_id)
                                    .into_iter()
                                    .flat_map(|track| track.clips.iter().map(|clip| clip.id))
                            })
                            .collect::<Vec<_>>();
                        state.select_many(ids);
                        project.selected_track = project
                            .tracks
                            .iter()
                            .position(|track| track.id == lane.id)
                            .and_then(|index| u32::try_from(index).ok());
                    } else {
                        project.selected_track = project
                            .tracks
                            .iter()
                            .position(|track| track.id == owner_track_id)
                            .and_then(|index| u32::try_from(index).ok());
                    }
                    if *edit_tool == EditTool::Split {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let beat = snap(((pos.x - origin) / px) as f64, grid);
                            if group_master {
                                group_split = Some((lane.id, beat));
                            } else {
                                split = Some((owner_track_id, c.id, beat));
                            }
                        }
                    } else if !group_master && ui.input(|i| i.modifiers.shift || i.modifiers.ctrl) {
                        state.selected_group_segment = None;
                        if state.selected != Some(c.id) && !state.extra_selected.insert(c.id) {
                            state.extra_selected.remove(&c.id);
                        }
                    } else if !selected && !group_master {
                        select_event_including_group(project, state, c.id);
                    }
                    ui.ctx().request_repaint();
                }
                if response.hovered() && *edit_tool == EditTool::Split {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    if let Some(pos) = response.hover_pos() {
                        let beat = snap(((pos.x - origin) / px) as f64, grid);
                        if let Some(seconds) =
                            c.cut_offset_seconds(beat, tempo, project.sample_rate_hz)
                        {
                            cut_preview = Some(c.start_beats + seconds * tempo / 60.0);
                        }
                    }
                }
                if response.hovered() && *edit_tool != EditTool::Split {
                    if let Some(pos) = response.hover_pos() {
                        let hovered = hit(c, event_rect(c, lane, origin, px, tempo), pos, selected);
                        ui.ctx().set_cursor_icon(if hovered == Tool::Move {
                            egui::CursorIcon::Default
                        } else {
                            cursor(hovered)
                        });
                        let text = match hovered {
                            Tool::FadeIn => "Fade in - drag to adjust selected events",
                            Tool::FadeOut => "Fade out - drag to adjust selected events",
                            Tool::Gain => "Event volume - drag; double-click for 0 dB",
                            Tool::Left => "Trim event start",
                            Tool::Right => "Trim event end",
                            Tool::Move => "",
                        };
                        if !text.is_empty() {
                            response.clone().on_hover_text(text);
                        }
                    }
                }

                if response.double_clicked()
                    && *edit_tool == EditTool::Select
                    && input.0.is_some_and(|pos| {
                        hit(c, event_rect(c, lane, origin, px, tempo), pos, selected) == Tool::Gain
                    })
                {
                    let mut edited = c.clone();
                    edited.gain = 1.0;
                    reset_gain = Some((owner_track_id, edited));
                }
                if response.double_clicked()
                    && *edit_tool == EditTool::Select
                    && !group_master
                    && input.0.is_some_and(|pos| {
                        hit(c, event_rect(c, lane, origin, px, tempo), pos, selected) != Tool::Gain
                    })
                {
                    if input.0.is_some_and(|p| {
                        p.y < event_rect(c, lane, origin, px, tempo).bottom() - 13.0
                    }) {
                        state.open_editor = Some((lane.id, c.id));
                    }
                }
                if *edit_tool == EditTool::Select
                    && response.drag_started_by(egui::PointerButton::Primary)
                    && state.gesture.is_none()
                    && state.group_gesture.is_none()
                {
                    let press = ui.input(|i| i.pointer.press_origin()).unwrap_or(r.center());
                    let was_selected =
                        state.selected == Some(c.id) || state.extra_selected.contains(&c.id);
                    let modifiers = ui.input(|i| i.modifiers);
                    if !was_selected && !modifiers.shift && !modifiers.ctrl {
                        select_event_including_group(project, state, c.id);
                    }
                    let tool = hit(c, event_rect(c, lane, origin, px, tempo), press, selected);
                    let selected_ids = state.selected_ids();
                    let selected_members = project
                        .tracks
                        .iter()
                        .enumerate()
                        .flat_map(|(row, track)| {
                            let selected_ids = &selected_ids;
                            let track_id = track.id;
                            track.clips.iter().filter_map(move |clip| {
                                selected_ids.contains(&clip.id).then(|| GroupMember {
                                    track: track_id,
                                    row,
                                    original: clip.clone(),
                                    candidate: clip.clone(),
                                    target: track_id,
                                })
                            })
                        })
                        .collect::<Vec<_>>();
                    if tool == Tool::Move
                        && was_selected
                        && (group_master || selected_members.len() > 1)
                        && let Some(anchor_row) =
                            lanes.iter().position(|candidate| candidate.id == lane.id)
                    {
                        state.group_gesture = Some(GroupGesture {
                            members: selected_members,
                            anchor_id: c.id,
                            anchor_row,
                            grab_beats: ((press.x - origin) / px) as f64 - c.start_beats,
                            press,
                            target_valid: true,
                            group_master,
                            tool: Tool::Move,
                            group_segment: None,
                            segment_bounds: None,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    if c.locked {
                        // Locked events refuse move, trim, fade and split, so the
                        // press only selects instead of starting a gesture.
                        select_event_including_group(project, state, c.id);
                        state.message =
                            Some("This audio event is locked; unlock it to edit".to_owned());
                        continue;
                    }
                    if !was_selected {
                        select_event_including_group(project, state, c.id);
                    }
                    let mut g = Gesture {
                        peers: if was_selected
                            && matches!(
                                tool,
                                Tool::Gain
                                    | Tool::FadeIn
                                    | Tool::FadeOut
                                    | Tool::Left
                                    | Tool::Right
                            ) {
                            selected_members
                                .into_iter()
                                .filter(|m| m.original.id != c.id)
                                .map(|m| (m.track, m.original))
                                .collect()
                        } else {
                            Vec::new()
                        },
                        track: owner_track_id,
                        original: c.clone(),
                        candidate: c.clone(),
                        press,
                        grab_beats: ((press.x - origin) / px) as f64 - c.start_beats,
                        tool,
                        source_duration: sources
                            .get(&c.source_path)
                            .map(|s| s.duration_seconds())
                            .unwrap_or(c.duration_seconds + c.source_offset_seconds),
                        target: Some(owner_track_id),
                    };
                    if let Some(pos) = input.0 {
                        update(
                            &mut g,
                            pos,
                            origin,
                            px,
                            tempo,
                            grid,
                            lanes,
                            1.0 / project.sample_rate_hz.max(1) as f64,
                            snap_targets,
                            8.0 / px.max(1.0) as f64,
                        );
                    }
                    state.gesture = Some(g);
                    ui.ctx().request_repaint();
                }
            }
        }
        if group_master {
            let segments = group_audio_segments(project, lane.id);
            let segment_count = segments.len();
            for (segment_index, (segment_start, segment_end)) in segments.into_iter().enumerate() {
                let segment_rect = Rect::from_min_max(
                    Pos2::new(origin + segment_start as f32 * px, lane.rect.top()),
                    Pos2::new(origin + segment_end as f32 * px, lane.rect.bottom()),
                )
                .intersect(visible);
                if !segment_rect.is_positive() {
                    continue;
                }
                let members = clips_for_group_segment(project, lane.id, segment_start, segment_end);
                if members.is_empty() {
                    continue;
                }
                let response = ui.interact(
                    segment_rect,
                    Id::new(("group-audio-piece", lane.id, segment_index)),
                    Sense::click_and_drag(),
                );
                if response.hovered() {
                    if *edit_tool == EditTool::Split {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        if let Some(pos) = response.hover_pos() {
                            let beat = snap(((pos.x - origin) / px) as f64, grid);
                            cut_preview = Some(beat);
                        }
                    } else {
                        // Keep the standard arrow while moving/editing a group
                        // segment; egui's drag response would otherwise use a
                        // grab-style cursor that differs from ordinary clips.
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
                    }
                }
                if response.clicked() {
                    state.select_group_segment(
                        lane.id,
                        segment_index,
                        members.iter().map(|(_, clip)| clip.id),
                    );
                    state.row_focus = false;
                    project.selected_track = project
                        .tracks
                        .iter()
                        .position(|track| track.id == lane.id)
                        .and_then(|index| u32::try_from(index).ok());
                    project.selected_midi_part_id = None;
                    if let Some(track) = project.tracks.iter().find(|track| track.id == lane.id) {
                        project.selected_track = Some(
                            project
                                .tracks
                                .iter()
                                .position(|candidate| candidate.id == track.id)
                                .unwrap_or_default() as u32,
                        );
                    }
                }
                if response.clicked_by(egui::PointerButton::Primary)
                    && *edit_tool == EditTool::Split
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    group_split = Some((lane.id, snap(((pos.x - origin) / px) as f64, grid)));
                }
                if *edit_tool == EditTool::Select
                    && response.drag_started_by(egui::PointerButton::Primary)
                    && state.group_gesture.is_none()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    let anchor = members
                        .iter()
                        .min_by(|(_, a), (_, b)| a.start_beats.total_cmp(&b.start_beats))
                        .cloned();
                    if let Some((_anchor_track, anchor_clip)) = anchor {
                        let anchor_row = lanes
                            .iter()
                            .position(|candidate| candidate.id == lane.id)
                            .unwrap_or_default();
                        state.select_group_segment(
                            lane.id,
                            segment_index,
                            members.iter().map(|(_, clip)| clip.id),
                        );
                        let segment_rect = Rect::from_min_max(
                            Pos2::new(origin + segment_start as f32 * px, lane.rect.top()),
                            Pos2::new(origin + segment_end as f32 * px, lane.rect.bottom()),
                        );
                        let proxy =
                            group_segment_proxy(project, lane.id, segment_start, segment_end);
                        let tool = proxy
                            .as_ref()
                            .map_or(Tool::Move, |clip| hit(clip, segment_rect, pos, true));
                        state.group_gesture = Some(GroupGesture {
                            members: members
                                .into_iter()
                                .map(|(track, original)| GroupMember {
                                    track,
                                    row: 0,
                                    candidate: original.clone(),
                                    original,
                                    target: track,
                                })
                                .collect(),
                            anchor_id: anchor_clip.id,
                            anchor_row,
                            grab_beats: if tool == Tool::Move {
                                ((pos.x - origin) / px) as f64 - anchor_clip.start_beats
                            } else if tool == Tool::Left {
                                segment_start
                            } else if tool == Tool::Right {
                                segment_end
                            } else {
                                ((pos.x - origin) / px) as f64
                            },
                            press: pos,
                            target_valid: true,
                            group_master: true,
                            tool,
                            group_segment: Some((
                                lane.id,
                                (segment_index > 0).then_some(segment_start),
                                (segment_index + 1 < segment_count).then_some(segment_end),
                            )),
                            segment_bounds: Some((segment_start, segment_end)),
                        });
                        project.selected_track = project
                            .tracks
                            .iter()
                            .position(|track| track.id == lane.id)
                            .and_then(|index| u32::try_from(index).ok());
                        state.row_focus = false;
                    }
                }
            }
        }
    }
    if let Some(beat) = cut_preview {
        let x = origin + beat as f32 * px;
        let pointer = ui.input(|i| i.pointer.hover_pos());
        if grid.is_some() {
            crate::system_tool_icons::tool_cursor_at(ui, ToolbarIcon::Split, pointer);
            ui.painter().with_clip_rect(visible).line_segment(
                [Pos2::new(x, visible.top()), Pos2::new(x, visible.bottom())],
                Stroke::new(1.0, Color32::WHITE),
            );
        } else {
            crate::system_tool_icons::tool_cursor_only_at(
                ui,
                ToolbarIcon::Split,
                ui.input(|i| i.pointer.hover_pos()),
            );
        }
    }
    if let Some(request) = rename_done {
        state.rename = None;
        state.rename_request = request;
    }
    if let Some((track, clip, at)) = split {
        if let Some(id) = project.split_audio_clip(track, clip, at) {
            state.selected = Some(id);
            state.extra_selected.clear();
            out.changed = true;
        }
    }
    if let Some((group_id, at)) = group_split {
        let (changed, _) = project.split_group_audio_clips(group_id, at);
        if changed {
            out.changed = true;
            if let Some((segment_index, (start, end))) = group_audio_segments(project, group_id)
                .into_iter()
                .enumerate()
                .find(|(_, (start, _))| (*start - at).abs() < 1e-6)
            {
                let ids = clips_for_group_segment(project, group_id, start, end)
                    .into_iter()
                    .map(|(_, clip)| clip.id)
                    .collect::<Vec<_>>();
                state.select_group_segment(group_id, segment_index, ids);
                project.selected_track = project
                    .tracks
                    .iter()
                    .position(|track| track.id == group_id)
                    .and_then(|index| u32::try_from(index).ok());
            }
        }
    }
    if enabled
        && state.selected.is_some()
        && !ui.ctx().egui_wants_keyboard_input()
        && ui.input(|i| i.key_pressed(egui::Key::X) && !i.modifiers.ctrl && !i.modifiers.alt)
    {
        let ids = state
            .selected
            .into_iter()
            .chain(state.extra_selected.iter().copied())
            .collect::<Vec<_>>();
        if ids.len() == 2 {
            out.crossfade_requested = true;
        } else {
            state.message=Some("Crossfade needs two overlapping audio events on the same track (Shift-click to select).".into());
        }
    }
    if let Some((track, edited)) = reset_gain {
        for t in &mut project.tracks {
            for c in &mut t.clips {
                if state.selected_ids().contains(&c.id) && c.id != edited.id && c.gain != 1.0 {
                    c.gain = 1.0;
                    out.changed = true;
                }
            }
        }
        out.changed |= project.edit_audio_clip(track, track, edited);
    }
    if let Some(g) = state.gesture.as_ref().filter(|g| g.tool == Tool::Move) {
        if let Some(lane) = lanes.iter().find(|l| Some(l.id) == g.target) {
            paint(
                ui,
                &g.candidate,
                lane,
                origin,
                px,
                tempo,
                visible,
                sources,
                true,
                false,
            );
        }
    }
    if let Some(group) = state
        .group_gesture
        .as_ref()
        .filter(|g| g.target_valid && !g.group_master)
    {
        for member in &group.members {
            if let Some(lane) = lanes.iter().find(|lane| lane.id == member.target) {
                paint(
                    ui,
                    &member.candidate,
                    lane,
                    origin,
                    px,
                    tempo,
                    visible,
                    sources,
                    true,
                    false,
                );
            }
        }
    }
    out
}

/// Hold secondary button, hover a tool, release to dismiss. Selection updates the shared toolbar.
pub(super) fn toolbox(
    ui: &mut egui::Ui,
    bounds: Rect,
    anchor: &mut Option<Pos2>,
    tool: &mut EditTool,
) {
    let (pos, pressed, down, released) = ui.input(|i| {
        (
            i.pointer.hover_pos(),
            i.pointer.button_pressed(egui::PointerButton::Secondary),
            i.pointer.secondary_down(),
            i.pointer.button_released(egui::PointerButton::Secondary),
        )
    });
    if pressed && pos.is_some_and(|p| bounds.contains(p)) {
        *anchor = pos;
    }
    if let Some(origin) = *anchor {
        let start = Pos2::new(
            origin.x.min(ui.ctx().content_rect().right() - 94.0),
            origin.y.min(ui.ctx().content_rect().bottom() - 38.0),
        );
        egui::Area::new(ui.id().with("held-toolbox"))
            .fixed_pos(start)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (value, icon, label) in [
                            (EditTool::Select, ToolbarIcon::Select, "Object Selection"),
                            (EditTool::Split, ToolbarIcon::Split, "Scissors"),
                        ] {
                            let (r, response) =
                                ui.allocate_exact_size(Vec2::new(36.0, 28.0), Sense::hover());
                            let hover = pos.is_some_and(|p| r.contains(p));
                            ui.painter().rect_filled(
                                r,
                                1.0,
                                if *tool == value || hover {
                                    theme::palette(ui).accent
                                } else {
                                    theme::palette(ui).surface_raised
                                },
                            );
                            crate::icons::paint_toolbar(ui, r.shrink(5.0), icon, Color32::BLACK);
                            response.on_hover_text(label);
                            if hover && (down || released) {
                                *tool = value;
                            }
                        }
                    });
                });
            });
        if released || !down {
            *anchor = None;
        } else {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        ctx: egui::Context,
        project: Project,
        state: State,
        lanes: Vec<Lane>,
        grid: Option<f64>,
        tool: EditTool,
        time: f64,
        guide: Option<f32>,
        crossfade_requested: bool,
    }
    impl Fixture {
        fn new() -> Self {
            let mut project = Project::default();
            project.tempo_bpm = 120.0;
            project.add_track(TrackKind::Audio);
            project.add_track(TrackKind::Audio);
            project.add_track_with_audio_channels(TrackKind::Audio, 1);
            project
                .add_audio_clip(0, PathBuf::from("test.wav"), 1.0, 4.0)
                .unwrap();
            let lanes = project
                .tracks
                .iter()
                .enumerate()
                .map(|(i, t)| Lane {
                    id: t.id,
                    channels: t.audio_channels,
                    audio: true,
                    rect: Rect::from_min_size(
                        Pos2::new(100.0, i as f32 * 70.0),
                        Vec2::new(1100.0, 70.0),
                    ),
                    take_lanes_visible: false,
                    take_lane_count: 1,
                    take_lane_indices: HashMap::new(),
                    automation: None,
                    color: Color32::from_rgb(110, 80, 150),
                })
                .collect();
            let mut f = Self {
                ctx: egui::Context::default(),
                project,
                state: State::default(),
                lanes,
                grid: Some(0.25),
                tool: EditTool::Select,
                time: 0.0,
                guide: None,
                crossfade_requested: false,
            };
            f.frame(vec![]);
            f.frame(vec![]);
            f
        }
        fn frame(&mut self, events: Vec<egui::Event>) -> bool {
            self.time += 1.0 / 60.0;
            let mut changed = false;
            let mut crossfade_requested = false;
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 230.0))),
                    events,
                    time: Some(self.time),
                    ..Default::default()
                },
                |ui| {
                    // Match the project view's full-row hover target on audio lanes.
                    // It must leave pointer ownership to the clip edit handles below.
                    let _ = ui.interact(
                        Rect::from_min_max(Pos2::new(100.0, 0.0), Pos2::new(1200.0, 210.0)),
                        Id::new(("track-arrange-row", self.project.tracks[0].id)),
                        Sense::hover(),
                    );
                    let result = show_with_tool(
                        ui,
                        &mut self.project,
                        &HashMap::new(),
                        &self.lanes,
                        Rect::from_min_max(Pos2::new(100.0, 0.0), Pos2::new(1200.0, 210.0)),
                        100.0,
                        100.0,
                        self.grid,
                        0.0,
                        &mut self.state,
                        true,
                        &mut self.tool,
                        false,
                        &[],
                    );
                    changed = result.changed;
                    crossfade_requested = result.crossfade_requested;
                },
            );
            self.crossfade_requested = crossfade_requested;
            self.guide = out.shapes.iter().find_map(|s| match &s.shape {
                egui::Shape::LineSegment { points, stroke }
                    if stroke.color == Color32::WHITE
                        && points[0].y == 0.0
                        && points[1].y == 210.0 =>
                {
                    Some(points[0].x)
                }
                _ => None,
            });
            let _ = self
                .ctx
                .tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
            out.textures_delta.clear();
            changed
        }
        fn button(&mut self, pos: Pos2, pressed: bool) -> bool {
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ])
        }
        fn select(&mut self) {
            self.button(Pos2::new(450.0, 30.0), true);
            self.button(Pos2::new(450.0, 30.0), false);
            self.frame(vec![]);
        }
    }
    #[test]
    fn scissors_cut_at_shared_grid_and_double_click_requests_editor() {
        let mut f = Fixture::new();
        f.tool = EditTool::Split;
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(445.0, 30.0))]);
        assert_eq!(f.guide, Some(450.0));
        f.button(Pos2::new(445.0, 30.0), true);
        assert!(f.button(Pos2::new(445.0, 30.0), false));
        assert_eq!(f.project.tracks[0].clips.len(), 2);
        assert_eq!(f.project.tracks[0].clips[1].start_beats, 3.5);
        assert_eq!(f.project.tracks[0].clips[1].source_offset_seconds, 1.25);
        f.tool = EditTool::Select;
        f.time += 0.5;
        f.frame(vec![]);
        f.frame(vec![]);
        for _ in 0..2 {
            f.button(Pos2::new(650.0, 30.0), true);
            f.button(Pos2::new(650.0, 30.0), false);
        }
        assert_eq!(
            f.state.open_editor,
            Some((f.project.tracks[0].id, f.project.tracks[0].clips[1].id))
        );
    }
    #[test]
    fn snap_off_cut_preview_has_scissors_without_a_guide() {
        let mut f = Fixture::new();
        f.tool = EditTool::Split;
        f.grid = None;
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(445.0, 30.0))]);
        assert_eq!(f.guide, None, "Snap off must not draw a snapped guide line");
    }

    #[test]
    fn held_secondary_toolbox_changes_shared_tool_on_hover_and_dismisses() {
        let mut f = Fixture::new();
        let button = |pressed, pos| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let anchor = Pos2::new(110.0, 110.0);
        f.frame(vec![
            egui::Event::PointerMoved(anchor),
            button(true, anchor),
        ]);
        f.frame(vec![]);
        let cut = Pos2::new(174.0, 127.0);
        f.frame(vec![egui::Event::PointerMoved(cut)]);
        assert_eq!(f.tool, EditTool::Split);
        f.frame(vec![button(false, cut)]);
        assert!(f.state.toolbox.is_none());
        f.frame(vec![
            egui::Event::PointerMoved(anchor),
            button(true, anchor),
        ]);
        f.frame(vec![]);
        let select = Pos2::new(132.0, 127.0);
        f.frame(vec![egui::Event::PointerMoved(select)]);
        assert_eq!(f.tool, EditTool::Select);
        f.frame(vec![button(false, select)]);
    }
    #[test]
    fn shift_selection_and_x_apply_real_overlap_fades() {
        let mut f = Fixture::new();
        let second = f
            .project
            .add_audio_clip(0, "second.wav".into(), 7.0, 3.0)
            .unwrap();
        f.select();
        let pos = Pos2::new(1100.0, 30.0);
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        for pressed in [true, false] {
            f.frame(vec![
                egui::Event::ModifiersChanged(shift),
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: shift,
                },
            ]);
        }
        assert!(f.state.extra_selected.contains(&second));
        f.frame(vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Key {
                key: egui::Key::X,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        assert!(
            f.crossfade_requested,
            "X must request a crossfade for the two selected events"
        );
        assert!(crossfade_selection(&mut f.project, &f.state.selected_ids()));
        assert_eq!(f.project.tracks[0].clips[0].fade_out_seconds, 1.0);
        assert_eq!(f.project.tracks[0].clips[1].fade_in_seconds, 1.0);
    }
    #[test]
    fn actual_left_drag_previews_without_mutating_then_moves_to_compatible_lane_using_snap() {
        let mut f = Fixture::new();
        f.button(Pos2::new(450.0, 30.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(590.0, 100.0))]);
        assert_eq!(f.project.tracks[0].clips.len(), 1);
        assert!(f.project.tracks[1].clips.is_empty());
        assert_eq!(f.state.gesture.as_ref().unwrap().candidate.start_beats, 2.5);
        assert!(f.button(Pos2::new(590.0, 100.0), false));
        assert!(f.project.tracks[0].clips.is_empty());
        assert_eq!(f.project.tracks[1].clips[0].start_beats, 2.5);
        assert_eq!(f.project.tracks[1].clips[0].duration_seconds, 4.0);
    }
    #[test]
    fn empty_space_drag_selects_intersecting_audio_events_without_dirtying_project() {
        let mut f = Fixture::new();
        let first = f.project.tracks[0].clips[0].id;
        let second = f
            .project
            .add_audio_clip(1, "second.wav".into(), 1.0, 2.0)
            .unwrap();
        let before = f.project.clone();
        f.button(Pos2::new(120.0, 20.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(450.0, 100.0))]);
        f.button(Pos2::new(450.0, 100.0), false);
        let selected = f.state.selected_ids().into_iter().collect::<HashSet<_>>();
        assert_eq!(selected, HashSet::from([first, second]));
        assert_eq!(f.project, before);
    }
    #[test]
    fn selected_events_move_as_one_snapped_group_to_compatible_lane() {
        let mut f = Fixture::new();
        let first = f.project.tracks[0].clips[0].id;
        let second = f
            .project
            .add_audio_clip(0, "second.wav".into(), 7.0, 2.0)
            .unwrap();
        f.select();
        let second_pos = Pos2::new(900.0, 30.0);
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        for pressed in [true, false] {
            f.frame(vec![
                egui::Event::ModifiersChanged(shift),
                egui::Event::PointerMoved(second_pos),
                egui::Event::PointerButton {
                    pos: second_pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: shift,
                },
            ]);
        }
        assert_eq!(f.state.selected_ids().len(), 2);
        f.button(Pos2::new(450.0, 30.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(550.0, 100.0))]);
        assert_eq!(f.project.tracks[0].clips.len(), 2);
        assert!(f.button(Pos2::new(550.0, 100.0), false));
        assert!(f.project.tracks[0].clips.is_empty());
        let moved_first = &f.project.tracks[1].clips[0];
        let moved_second = &f.project.tracks[1].clips[1];
        assert_eq!((moved_first.id, moved_second.id), (first, second));
        assert_eq!(moved_first.start_beats, 2.0);
        assert_eq!(moved_second.start_beats, 8.0);
        assert_eq!(moved_second.start_beats - moved_first.start_beats, 6.0);
    }
    #[test]
    fn incompatible_lane_drop_is_rejected_and_snap_off_preserves_free_position() {
        let mut f = Fixture::new();
        let before = f.project.tracks[0].clips[0].clone();
        f.button(Pos2::new(450.0, 30.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(590.0, 170.0))]);
        f.button(Pos2::new(590.0, 170.0), false);
        assert_eq!(f.project.tracks[0].clips[0], before);
        assert!(f.project.tracks[2].clips.is_empty());
        f.grid = None;
        f.button(Pos2::new(450.0, 30.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(590.0, 100.0))]);
        f.button(Pos2::new(590.0, 100.0), false);
        assert!((f.project.tracks[1].clips[0].start_beats - 2.4).abs() < 0.0001);
    }
    #[test]
    fn selected_white_trim_handle_changes_source_offset_and_escape_restores_original() {
        let mut f = Fixture::new();
        f.select();
        let before = f.project.tracks[0].clips[0].clone();
        f.button(Pos2::new(203.0, 67.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(300.0, 67.0))]);
        f.frame(vec![]);
        let c = &f.project.tracks[0].clips[0];
        assert_eq!(c.start_beats, 2.0);
        assert_eq!(c.source_offset_seconds, 0.5);
        assert_eq!(c.duration_seconds, 3.5);
        f.frame(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(f.project.tracks[0].clips[0], before);
        f.button(Pos2::new(300.0, 67.0), false);
    }
    #[test]
    fn double_clicking_event_name_edits_the_file_name_instead_of_opening_the_sample_editor() {
        let mut f = Fixture::new();
        f.select();
        let name = Pos2::new(450.0, 64.0);
        f.button(name, true);
        f.button(name, false);
        f.button(name, true);
        f.button(name, false);
        f.frame(vec![]);
        assert!(f.state.rename.is_some());
        assert!(f.state.open_editor.is_none());
        f.frame(vec![egui::Event::Text("Lead vocal".into())]);
        f.frame(vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(
            f.state.rename_request.as_ref().map(|r| r.2.as_str()),
            Some("Lead vocal")
        );
        assert_eq!(
            f.project.tracks[0].clips[0].source_path,
            PathBuf::from("test.wav"),
            "the UI only requests a prepared file operation"
        );
    }

    #[test]
    fn selected_event_fades_and_volume_adjust_as_a_group_and_escape_restores_them() {
        let mut f = Fixture::new();
        let first = f.project.tracks[0].clips[0].id;
        let second = f
            .project
            .add_audio_clip(1, PathBuf::from("second.wav"), 1.0, 4.0)
            .unwrap();
        f.state.select_many([first, second]);
        f.frame(vec![]);
        f.button(Pos2::new(203.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(400.0, 3.0))]);
        f.button(Pos2::new(400.0, 3.0), false);
        assert!((f.project.tracks[0].clips[0].fade_in_seconds - 1.0).abs() < 0.001);
        assert!((f.project.tracks[1].clips[0].fade_in_seconds - 1.0).abs() < 0.001);
        f.frame(vec![]);
        f.button(Pos2::new(600.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(600.0, 27.0))]);
        f.frame(vec![]);
        for track in &f.project.tracks[..2] {
            assert!((20.0 * track.clips[0].gain.log10() + 6.0).abs() < 0.01);
        }
        f.frame(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        for track in &f.project.tracks[..2] {
            assert_eq!(track.clips[0].gain, 1.0);
        }
        f.button(Pos2::new(600.0, 27.0), false);
    }

    #[test]
    fn fade_and_gain_handles_change_real_event_parameters_and_gain_snaps_back_to_unity() {
        let mut f = Fixture::new();
        f.select();
        f.button(Pos2::new(203.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(400.0, 3.0))]);
        f.button(Pos2::new(400.0, 3.0), false);
        assert!((f.project.tracks[0].clips[0].fade_in_seconds - 1.0).abs() < 0.001);
        f.button(Pos2::new(600.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(600.0, 27.0))]);
        f.frame(vec![]);
        assert!((20.0 * f.project.tracks[0].clips[0].gain.log10() + 6.0).abs() < 0.01);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(600.0, 4.0))]);
        f.button(Pos2::new(600.0, 4.0), false);
        assert_eq!(f.project.tracks[0].clips[0].gain, 1.0);
    }
    #[test]
    fn bounds_cover_whole_lane_and_all_edit_handles_are_white() {
        let mut f = Fixture::new();
        f.select();
        let c = &f.project.tracks[0].clips[0];
        let r = event_rect(c, &f.lanes[0], 100.0, 100.0, 120.0);
        assert_eq!(r.top(), 0.0);
        assert_eq!(r.bottom(), 70.0);
        let mut out = f.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 230.0))),
                ..Default::default()
            },
            |ui| {
                paint(
                    ui,
                    c,
                    &f.lanes[0],
                    100.0,
                    100.0,
                    120.0,
                    ui.max_rect(),
                    &HashMap::new(),
                    true,
                    false,
                );
            },
        );
        let handles=out.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.fill==Color32::WHITE && r.rect.size()==Vec2::splat(6.0))).count();
        assert_eq!(handles, 5);
        out.textures_delta.clear();
    }
    #[test]
    fn right_trim_fade_out_and_gain_boost_are_undoable_and_preserve_media() {
        let mut f = Fixture::new();
        f.select();
        let before = f.project.clone();
        let mut history = ProjectHistory::default();
        history.begin(&before, false);
        f.button(Pos2::new(997.0, 67.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(850.0, 67.0))]);
        f.button(Pos2::new(850.0, 67.0), false);
        assert!((f.project.tracks[0].clips[0].duration_seconds - 3.25).abs() < 0.0001);
        f.button(Pos2::new(847.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(750.0, 3.0))]);
        f.button(Pos2::new(750.0, 3.0), false);
        assert!((f.project.tracks[0].clips[0].fade_out_seconds - 0.5).abs() < 0.001);
        f.button(Pos2::new(525.0, 3.0), true);
        f.frame(vec![egui::Event::PointerMoved(Pos2::new(525.0, -21.0))]);
        f.button(Pos2::new(525.0, -21.0), false);
        assert!((20.0 * f.project.tracks[0].clips[0].gain.log10() - 6.0).abs() < 0.01);
        assert!(history.finish_frame(&f.project, true));
        let undo = history.undo(&f.project).unwrap();
        assert_eq!(undo, before);
        assert_eq!(history.redo(&undo).unwrap(), f.project);
    }
}

#[cfg(test)]
mod group_lane_tests {
    use super::*;

    fn grouped_project() -> Project {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let group_id = project.tracks[0].id;
        project.tracks[1].parent_group_id = Some(group_id);
        project.tracks[2].parent_group_id = Some(group_id);
        project
    }

    #[test]
    fn a_group_cut_through_take_lanes_stays_visible() {
        let mut project = grouped_project();
        let group_id = project.tracks[0].id;
        // Two takes stacked in one lane plus one plain clip: 4 s at 120 bpm is 8 beats.
        project
            .add_audio_clip(1, PathBuf::from("take-a.wav"), 0.0, 4.0)
            .unwrap();
        project
            .add_audio_clip(1, PathBuf::from("take-b.wav"), 0.0, 4.0)
            .unwrap();
        project
            .add_audio_clip(2, PathBuf::from("plain.wav"), 0.0, 4.0)
            .unwrap();

        let (split, _) = project.split_group_audio_clips(group_id, 4.0);
        assert!(split, "group split should succeed");

        let segments = group_audio_segments(&project, group_id);
        assert_eq!(
            segments.len(),
            2,
            "the cut should be visible on the group, got {segments:?}"
        );
    }

    #[test]
    fn a_group_cut_stays_visible_when_a_child_starts_later() {
        let mut project = grouped_project();
        let group_id = project.tracks[0].id;
        // Lane 1 holds two takes over the whole span; lane 2 only has audio after
        // the cut, which is what a later take in a group looks like.
        project
            .add_audio_clip(1, PathBuf::from("take-a.wav"), 0.0, 4.0)
            .unwrap();
        project
            .add_audio_clip(1, PathBuf::from("take-b.wav"), 0.0, 4.0)
            .unwrap();
        project
            .add_audio_clip(2, PathBuf::from("late.wav"), 5.0, 1.5)
            .unwrap();

        let (split, _) = project.split_group_audio_clips(group_id, 4.0);
        assert!(split, "group split should succeed");

        let segments = group_audio_segments(&project, group_id);
        assert_eq!(
            segments.len(),
            2,
            "the cut should be visible on the group, got {segments:?}"
        );
    }
}

#[cfg(test)]
mod split_selection_tests {
    use super::*;

    #[test]
    fn split_at_cursor_splits_a_selected_group_segment() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Group);
        project.add_track(TrackKind::Audio);
        let group_id = project.tracks[0].id;
        project.tracks[1].parent_group_id = Some(group_id);
        app.data.project = project;
        app.data
            .project
            .add_audio_clip(1, PathBuf::from("take.wav"), 0.0, 4.0)
            .unwrap();
        let ids = clips_for_group_segment(&app.data.project, group_id, 0.0, 8.0)
            .into_iter()
            .map(|(_, clip)| clip.id)
            .collect::<Vec<_>>();
        app.arrange_view_state
            .audio_events
            .select_group_segment(group_id, 0, ids);
        app.data.transport.position_beats = 4.0;
        app.project_history.clear();

        app.split_selected_audio_events_at_cursor();

        assert_eq!(
            group_audio_segments(&app.data.project, group_id).len(),
            2,
            "the group now has two segments"
        );
        assert!(app.modified);
        assert!(app.status_message.contains("group segment"));
    }

    #[test]
    fn repeat_fill_loop_crop_and_invert_are_single_undo_steps() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        // 2 s at 120 BPM is 4 beats.
        let id = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        app.project_history.clear();

        // Repeat twice: the original plus two copies, all selected.
        app.arrange_view_state.audio_events.select_many([id]);
        app.repeat_selected_audio_events(2);
        assert_eq!(app.data.project.tracks[0].clips.len(), 3);
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids().len(),
            3,
            "the whole repeated run stays selected"
        );

        // Fill Loop repeats just that event until the loop end: the loop is
        // 0-16 and the event is 4 beats, so three more copies are added.
        app.data.project.set_loop_range(0.0, 16.0);
        app.arrange_view_state.audio_events.select_many([id]);
        app.fill_loop_with_selected_audio_events();
        assert_eq!(app.data.project.tracks[0].clips.len(), 6);

        // Crop trims the events crossing the loop and leaves the rest alone.
        let before_crop = app.data.project.clone();
        app.data.project.set_loop_range(2.0, 6.0);
        app.arrange_view_state.audio_events.select_many(
            app.data.project.tracks[0]
                .clips
                .iter()
                .map(|clip| clip.id)
                .collect::<Vec<_>>(),
        );
        app.crop_selected_audio_events_to_loop();
        assert_ne!(app.data.project, before_crop);
        let clips_in_range = app.data.project.tracks[0]
            .clips
            .iter()
            .filter(|clip| {
                let end = clip.start_beats + clip.duration_seconds * 120.0 / 60.0;
                clip.start_beats >= 2.0 - 1e-9 && end <= 6.0 + 1e-9
            })
            .count();
        assert_eq!(
            clips_in_range, 3,
            "the three events crossing the range were trimmed into it"
        );
        assert_eq!(
            app.data.project.tracks[0].clips.len(),
            6,
            "events entirely outside the range are left untouched"
        );

        // Invert selects what is not selected.
        app.arrange_view_state
            .audio_events
            .select_many([app.data.project.tracks[0].clips[0].id]);
        app.invert_audio_event_selection();
        assert!(
            !app.arrange_view_state
                .audio_events
                .selected_ids()
                .is_empty()
        );
        assert_ne!(
            app.arrange_view_state.audio_events.selected_ids(),
            vec![app.data.project.tracks[0].clips[0].id]
        );
        assert!(app.modified);
    }

    #[test]
    fn repeat_and_crop_report_what_they_cannot_do_without_editing() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let before = app.data.project.clone();

        // Nothing selected.
        app.repeat_selected_audio_events(2);
        assert!(app.status_message.contains("Select audio events"));
        app.fill_loop_with_selected_audio_events();
        app.crop_selected_audio_events_to_loop();
        assert_eq!(app.data.project.tracks[0].clips, before.tracks[0].clips);

        // Selected, but the loop ends before the event does.
        let id = app.data.project.tracks[0].clips[0].id;
        app.arrange_view_state.audio_events.select_many([id]);
        app.data.project.set_loop_range(0.0, 2.0);
        app.fill_loop_with_selected_audio_events();
        assert!(
            app.status_message.contains("Fill Loop needs a loop range"),
            "status was {}",
            app.status_message
        );

        // Crop reports a loop range the event does not cross.
        app.data.project.set_loop_range(8.0, 12.0);
        app.crop_selected_audio_events_to_loop();
        assert!(
            app.status_message
                .contains("No selected audio event crosses"),
            "status was {}",
            app.status_message
        );
        assert_eq!(
            app.data.project.tracks[0].clips, before.tracks[0].clips,
            "no event was trimmed"
        );
    }

    /// A minimal readable WAV so a copied event can be opened like a real take.
    fn write_wav(path: &Path, frames: usize) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + (frames * 8) as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&3_u16.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&(48_000_u32 * 8).to_le_bytes());
        bytes.extend_from_slice(&8_u16.to_le_bytes());
        bytes.extend_from_slice(&32_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&((frames * 8) as u32).to_le_bytes());
        for index in 0..frames {
            let value = (index as f32) / 48_000.0;
            bytes.extend_from_slice(&value.to_le_bytes());
            bytes.extend_from_slice(&(-value).to_le_bytes());
        }
        std::fs::write(path, bytes).expect("write wav");
    }

    #[test]
    fn gluing_selected_pieces_is_one_undo_step_and_refuses_otherwise() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let original = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 4.0)
            .unwrap();
        let track_id = app.data.project.tracks[0].id;
        let right = app
            .data
            .project
            .split_audio_clip(track_id, original, 4.0)
            .expect("split");
        app.project_history.clear();

        app.arrange_view_state
            .audio_events
            .select_many([original, right]);
        app.glue_selected_audio_events();
        assert_eq!(app.data.project.tracks[0].clips.len(), 1);
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids(),
            vec![original],
            "the joined event is selected"
        );
        assert!(app.status_message.contains("Glued"));

        // A single event is not glue, and two unrelated events are refused.
        app.arrange_view_state.audio_events.select_many([original]);
        app.glue_selected_audio_events();
        assert!(app.status_message.contains("at least two"));

        let apart = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("b.wav"), 12.0, 2.0)
            .unwrap();
        app.arrange_view_state
            .audio_events
            .select_many([original, apart]);
        app.glue_selected_audio_events();
        assert!(app.status_message.contains("contiguous pieces"));
        assert_eq!(app.data.project.tracks[0].clips.len(), 2);
    }

    #[test]
    fn paste_time_makes_room_then_pastes_as_one_undo_step() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        // The source event is two seconds (four beats); a later event marks the
        // material that must move out of the way.
        let source = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("source.wav"), 0.0, 2.0)
            .unwrap();
        let later = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("later.wav"), 8.0, 2.0)
            .unwrap();
        app.arrange_view_state.audio_events.select_many([source]);
        app.copy_selected_audio_events();
        // The cursor sits in the gap between the source and the later event, so
        // only the later event has to move.
        app.data.transport.position_beats = 6.0;
        app.project_history.clear();

        app.paste_time();
        assert!(app.status_message.contains("Inserted 4.000 beat(s)"));
        let mut starts = app.data.project.tracks[0]
            .clips
            .iter()
            .map(|clip| {
                (
                    clip.start_beats,
                    clip.source_path.to_string_lossy().into_owned(),
                )
            })
            .collect::<Vec<_>>();
        starts.sort_by(|left, right| left.0.total_cmp(&right.0));
        // The source stays at 0; the copy lands at the cursor (6..10); the later
        // event moved from 8..12 to 12..16 to make room.
        assert_eq!(starts[0], (0.0, "source.wav".to_owned()));
        assert_eq!(starts[1], (6.0, "source.wav".to_owned()));
        assert_eq!(starts[2], (12.0, "later.wav".to_owned()));
        assert!(
            starts
                .iter()
                .any(|(start, path)| *start == 12.0 && path == "later.wav")
        );
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .any(|clip| clip.id == later && (clip.start_beats - 12.0).abs() < 1e-9)
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Paste Time");

        // Undo restores the single-step arrangement.
        let restored = app
            .project_history
            .undo(&app.data.project)
            .expect("undo paste time");
        let starts = restored.tracks[0]
            .clips
            .iter()
            .map(|clip| clip.start_beats)
            .collect::<Vec<_>>();
        assert_eq!(starts, vec![0.0, 8.0]);
    }

    #[test]
    fn paste_time_refuses_an_empty_clipboard_and_a_locked_crossing() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.add_track(TrackKind::Audio);
        app.paste_time();
        assert!(app.status_message.contains("Copy audio events"));

        let clip = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        app.arrange_view_state.audio_events.select_many([clip]);
        app.copy_selected_audio_events();
        app.data.project.set_audio_clips_locked(&[clip], true);
        app.data.transport.position_beats = 1.0;
        let before = app.data.project.clone();
        app.paste_time();
        assert!(
            app.status_message.contains("locked event crosses"),
            "status was: {}",
            app.status_message
        );
        assert_eq!(app.data.project, before);
    }

    #[test]
    fn fading_and_silencing_the_selection_are_labelled_undo_steps() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let first = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 4.0)
            .unwrap();
        let second = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("b.wav"), 8.0, 4.0)
            .unwrap();
        app.project_history.clear();
        app.arrange_view_state
            .audio_events
            .select_many([first, second]);

        app.fade_selected_audio_events(true);
        assert_eq!(app.data.project.tracks[0].clips[0].fade_in_seconds, 0.25);
        assert!(app.status_message.contains("fade in"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Fade In");

        app.fade_selected_audio_events(false);
        assert_eq!(app.data.project.tracks[0].clips[0].fade_out_seconds, 0.25);
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Fade Out");

        app.silence_selected_audio_events();
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .all(|clip| clip.gain == 0.0)
        );
        assert!(app.status_message.contains("media stays in the pool"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Silence");
        assert!(
            app.data
                .project
                .media_pool
                .contains(&PathBuf::from("a.wav"))
        );

        // Repeating silence changes nothing and does not write another step.
        let steps = app.project_history.counts().0;
        app.silence_selected_audio_events();
        assert!(app.status_message.contains("No event changed"));
        assert_eq!(app.project_history.counts().0, steps);

        // Nothing selected is reported too.
        app.arrange_view_state.audio_events.clear_selection();
        app.fade_selected_audio_events(true);
        assert!(app.status_message.contains("Select audio events to fade"));
    }

    #[test]
    fn deleting_the_loop_range_is_one_undo_step_and_reports_when_nothing_crosses() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let id = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 8.0)
            .unwrap();
        app.data.project.set_loop_range(4.0, 8.0);
        app.arrange_view_state.audio_events.select_many([id]);
        app.project_history.clear();

        app.delete_loop_range_from_selected_events();
        let spans = app.data.project.tracks[0]
            .clips
            .iter()
            .map(|clip| {
                (
                    clip.start_beats,
                    clip.start_beats + clip.duration_seconds * 120.0 / 60.0,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(spans, vec![(0.0, 4.0), (8.0, 16.0)]);
        assert!(app.status_message.contains("Deleted the loop range from 1"));

        // The remaining parts no longer overlap the range.
        app.delete_loop_range_from_selected_events();
        assert!(
            app.status_message
                .contains("No selected audio event overlaps")
        );
        assert!(app.modified);
    }

    #[test]
    fn snapping_prefers_a_nearby_event_edge_over_the_grid() {
        // Grid is a sixteenth (0.25 beat); an event edge sits at 1.10.
        let grid = Some(0.25);
        let targets = [1.10];

        // Far from the edge: the grid wins.
        assert_eq!(snap_with_targets(0.52, grid, &targets, 0.2), 0.5);
        // Within tolerance and closer than the grid point: the edge wins.
        assert_eq!(snap_with_targets(1.12, grid, &targets, 0.2), 1.10);
        // Closer to the grid than to the edge: the grid stays predictable.
        assert_eq!(snap_with_targets(1.05, grid, &targets, 0.2), 1.0);
        // Beyond tolerance: back to the grid.
        assert_eq!(snap_with_targets(1.40, grid, &targets, 0.2), 1.5);
        // With no targets the behaviour is the plain grid snap.
        assert_eq!(snap_with_targets(0.52, grid, &[], 0.2), 0.5);
        // A zero tolerance keeps the grid.
        assert_eq!(snap_with_targets(1.02, grid, &targets, 0.0), 1.0);
        // No grid means snapping is off altogether, so targets do not apply.
        assert_eq!(snap_with_targets(1.02, None, &targets, 0.2), 1.02);
        assert_eq!(snap_with_targets(-3.0, None, &targets, 0.2), 0.0);
    }

    #[test]
    fn nudging_moves_the_selection_by_one_grid_step() {
        use crate::timeline::QuantizeNote;

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let id = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 4.0, 2.0)
            .unwrap();
        app.project_history.clear();
        app.arrange_view_state.snap_enabled = true;
        app.arrange_view_state.grid_type = crate::timeline::GridType::UseQuantize;
        app.arrange_view_state.quantize.note = QuantizeNote::Eighth;
        app.arrange_view_state.audio_events.select_many([id]);

        // An eighth at 4/4 is half a beat.
        app.nudge_selected_audio_events(true);
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 4.5);
        app.nudge_selected_audio_events(true);
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 5.0);
        app.nudge_selected_audio_events(false);
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 4.5);
        assert!(app.status_message.contains("Nudged 1 event(s) left"));
        assert!(app.modified);
    }

    #[test]
    fn nudging_refuses_at_the_project_start_and_on_locked_events() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let id = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        app.arrange_view_state.audio_events.select_many([id]);
        app.nudge_selected_audio_events(false);
        assert!(app.status_message.contains("before the project start"));
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 0.0);

        app.data.project.set_audio_clips_locked(&[id], true);
        app.nudge_selected_audio_events(true);
        assert!(app.status_message.contains("could not be nudged"));
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 0.0);

        // Nothing selected is reported too.
        app.arrange_view_state.audio_events.clear_selection();
        app.nudge_selected_audio_events(true);
        assert!(app.status_message.contains("Select audio events to nudge"));
    }

    #[test]
    fn split_loop_cuts_at_both_locators_as_one_undo_step() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 8.0)
            .unwrap();
        app.data.project.set_loop_range(4.0, 8.0);
        app.project_history.clear();

        app.split_audio_events_at_loop();
        assert_eq!(app.data.project.tracks[0].clips.len(), 3);
        assert!(app.status_message.contains("Split 2 audio event(s)"));

        // Nothing crosses a locator the second time.
        app.split_audio_events_at_loop();
        assert!(app.status_message.contains("No audio event crosses"));
        assert!(app.modified);
    }

    #[test]
    fn converting_to_a_real_copy_moves_the_event_onto_project_media() {
        let root = std::env::temp_dir().join(format!(
            "opentrack-real-copy-{}-{}",
            std::process::id(),
            "app"
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Audio").join("Recordings")).expect("folders");
        std::fs::create_dir_all(root.join("Audio").join("Imports")).expect("imports");
        std::fs::create_dir_all(root.join("Metadata")).expect("metadata");
        // The shared source lives outside the project, which is the case a real
        // copy is for.
        let outside = root.with_file_name(format!("{}-shared", "opentrack-real-copy-src"));
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).expect("shared folder");
        let shared = outside.join("shared-take.wav");
        write_wav(&shared, 480);

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        let id = app
            .data
            .project
            .add_audio_clip(0, shared.clone(), 0.0, 1.0)
            .unwrap();
        app.project_history.clear();

        app.arrange_view_state.audio_events.select_many([id]);
        app.convert_selected_events_to_real_copy();

        let clip = &app.data.project.tracks[0].clips[0];
        assert_ne!(clip.source_path, shared, "the event left the shared source");
        assert_eq!(
            clip.source_path.parent(),
            Some(root.join("Audio").join("Imports").as_path())
        );
        assert!(clip.source_path.exists(), "the copy exists on disk");
        assert!(
            app.data.audio_sources.contains_key(&clip.source_path),
            "the copy is ready to play"
        );
        assert!(app.status_message.contains("Audio/Imports"));

        // Converting again is a no-op because the media already belongs to the
        // project.
        app.convert_selected_events_to_real_copy();
        assert!(
            app.status_message
                .contains("already use this project's own media")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn converting_leaves_pitch_and_time_events_alone() {
        let root =
            std::env::temp_dir().join(format!("opentrack-real-copy-{}-pitch", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Audio").join("Imports")).expect("imports");
        std::fs::create_dir_all(root.join("Metadata")).expect("metadata");
        let outside = root.with_file_name("opentrack-real-copy-src-pitch");
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&outside).expect("shared folder");
        let shared = outside.join("pitched.wav");
        write_wav(&shared, 480);

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.project_folder_managed = true;
        app.project_path = Some(root.join("Song.opr"));
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, shared.clone(), 0.0, 1.0)
            .unwrap();
        app.data.project.tracks[0].clips[0].pitch_time =
            Some(opentrack_core::pitch_time::PitchTimeEdit {
                detector_version: opentrack_core::pitch_time::DETECTOR_VERSION,
                source_bytes: 0,
                source_modified_ns: 0,
                analysis: opentrack_audio::PitchAnalysis {
                    sample_rate_hz: 16_000,
                    source_start_seconds: 0.0,
                    source_duration_seconds: 1.0,
                    points: Vec::new(),
                },
                notes: Vec::new(),
                rendered_source: None,
                enabled: true,
            });
        let id = app.data.project.tracks[0].clips[0].id;

        app.arrange_view_state.audio_events.select_many([id]);
        app.convert_selected_events_to_real_copy();

        assert_eq!(
            app.data.project.tracks[0].clips[0].source_path, shared,
            "a pitch-edited event keeps its analysed source"
        );
        assert!(
            app.status_message.contains("Pitch & Time"),
            "the reason is explained: {}",
            app.status_message
        );
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }

    #[test]
    fn the_project_logical_editor_selects_and_processes_matches_as_one_step() {
        use opentrack_core::event_query::{EventQuery, EventStateFilter, EventTransform};

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.tracks[0].name = "Drums".to_owned();
        app.data.project.tracks[1].name = "Bass".to_owned();
        let kick = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("kick.wav"), 0.0, 2.0)
            .unwrap();
        let snare = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("snare.wav"), 8.0, 2.0)
            .unwrap();
        let bass = app
            .data
            .project
            .add_audio_clip(1, PathBuf::from("bass.wav"), 4.0, 2.0)
            .unwrap();
        app.project_history.clear();

        // Select every event on the Drums track.
        app.project_logical_editor = ProjectLogicalEditor {
            query: EventQuery {
                track_contains: "drums".to_owned(),
                ..EventQuery::default()
            },
            transform: EventTransform::Mute,
        };
        app.select_logical_query_matches();
        let mut selected = app.arrange_view_state.audio_events.selected_ids();
        selected.sort_unstable();
        assert_eq!(selected, vec![kick, snare]);
        assert_eq!(
            app.data.project.tracks[1].clips[0].id, bass,
            "the other track is untouched"
        );

        // Process mutes exactly those events, as one undo step.
        app.process_logical_editor();
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .all(|clip| clip.muted)
        );
        assert!(!app.data.project.tracks[1].clips[0].muted);
        assert!(
            app.status_message.contains("Mute") && app.status_message.contains("2 event(s)"),
            "status was {}",
            app.status_message
        );

        // Running it again reports that nothing changed.
        app.process_logical_editor();
        assert!(app.status_message.contains("already had that state"));

        // A state query finds the events the previous pass muted.
        app.project_logical_editor.query = EventQuery {
            state: EventStateFilter::Muted,
            ..EventQuery::default()
        };
        app.project_logical_editor.transform = EventTransform::Unmute;
        app.process_logical_editor();
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .all(|clip| !clip.muted)
        );

        // A query that matches nothing says so instead of changing anything.
        app.project_logical_editor.query.name_contains = "no such take".to_owned();
        app.process_logical_editor();
        assert!(app.status_message.contains("matched no events"));
    }

    #[test]
    fn the_project_logical_editor_dialog_renders_its_query() {
        use opentrack_core::event_query::EventQuery;

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, PathBuf::from("kick.wav"), 0.0, 2.0)
            .unwrap();
        app.project_logical_editor.query = EventQuery {
            name_contains: "kick".to_owned(),
            ..EventQuery::default()
        };
        app.project_logical_editor_open = true;

        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_project_logical_editor(&ctx),
            );
            output.textures_delta.clear();
            if frame == 0 {
                continue;
            }
            fn collect(shape: &egui::Shape, texts: &mut Vec<String>) {
                match shape {
                    egui::Shape::Text(text) => texts.push(text.galley.text().to_owned()),
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect(shape, texts);
                        }
                    }
                    _ => {}
                }
            }
            for shape in &output.shapes {
                collect(&shape.shape, &mut texts);
            }
        }
        let joined = texts.join("\n");
        assert!(
            joined.contains("Project Logical Editor"),
            "the dialog is titled: {joined}"
        );
        assert!(
            joined.contains("1 event(s) match this query"),
            "the live match count is shown: {joined}"
        );
        assert!(joined.contains("Select Matching"), "{joined}");
        assert!(joined.contains("Process Matching"), "{joined}");
    }

    #[test]
    fn the_project_logical_editor_query_persists() {
        use opentrack_core::event_query::{EventQuery, EventStateFilter, EventTransform};

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_logical_editor = ProjectLogicalEditor {
            query: EventQuery {
                name_contains: "take 2".to_owned(),
                track_contains: "guitar".to_owned(),
                state: EventStateFilter::Locked,
                from_beats: Some(4.0),
                to_beats: Some(24.0),
            },
            transform: EventTransform::Lock,
        };

        #[derive(Default)]
        struct Storage {
            values: std::collections::HashMap<String, String>,
        }
        impl eframe::Storage for Storage {
            fn get_string(&self, key: &str) -> Option<String> {
                self.values.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.values.insert(key.to_owned(), value);
            }
            fn remove_string(&mut self, key: &str) {
                self.values.remove(key);
            }
            fn flush(&mut self) {}
        }
        let mut storage = Storage::default();
        eframe::set_value(
            &mut storage,
            "project-logical-editor-v1",
            &app.project_logical_editor,
        );
        let restored =
            eframe::get_value::<ProjectLogicalEditor>(&storage, "project-logical-editor-v1")
                .expect("the query persists");
        assert_eq!(restored, app.project_logical_editor);
        assert_eq!(restored.transform, EventTransform::Lock);
        assert_eq!(restored.query.from_beats, Some(4.0));
    }

    #[test]
    fn automation_follows_events_decides_whether_the_curve_travels() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let id = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let track_id = app.data.project.tracks[0].id;
        for beat in [1.0, 3.0] {
            opentrack_core::automation::set_volume_point(
                &mut app.data.project,
                track_id,
                opentrack_core::automation::AutomationPoint { beat, value: 0.5 },
            )
            .unwrap();
        }
        let beats = |app: &OpenTrackApp| {
            app.data.project.tracks[0].automation_lanes[0]
                .points
                .iter()
                .map(|point| point.beat)
                .collect::<Vec<_>>()
        };

        // Off by default: moving the event leaves the curve in place. The event
        // spans beats 0-4, so the points at 1 and 3 sit under it.
        app.arrange_view_state.audio_events.select_many([id]);
        app.data.transport.position_beats = 4.0;
        app.move_selected_audio_events_to_cursor();
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 4.0);
        assert_eq!(beats(&app), vec![1.0, 3.0], "the curve stayed put");

        // Back to the start so the next move has the points under the event.
        app.arrange_view_state.audio_events.select_many([id]);
        app.data.transport.position_beats = 0.0;
        app.move_selected_audio_events_to_cursor();
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 0.0);
        assert_eq!(beats(&app), vec![1.0, 3.0]);

        // On: the points under the event travel with it.
        app.automation_follows_events = true;
        app.arrange_view_state.audio_events.select_many([id]);
        app.data.transport.position_beats = 4.0;
        app.move_selected_audio_events_to_cursor();
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 4.0);
        assert_eq!(
            beats(&app),
            vec![5.0, 7.0],
            "the points moved by the same four beats"
        );
    }

    #[test]
    fn auto_select_under_cursor_follows_the_preference() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let first = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let second = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("b.wav"), 8.0, 2.0)
            .unwrap();

        // Off by default: seeking selects nothing.
        app.data.transport.position_beats = 9.0;
        app.apply_auto_select_under_cursor();
        assert!(
            app.arrange_view_state
                .audio_events
                .selected_ids()
                .is_empty()
        );

        // On: the event under the cursor becomes the selection.
        app.auto_select_events_under_cursor = true;
        app.apply_auto_select_under_cursor();
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids(),
            vec![second]
        );

        // Moving the cursor follows; a position in a gap clears the selection.
        app.data.transport.position_beats = 1.0;
        app.apply_auto_select_under_cursor();
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids(),
            vec![first]
        );
        app.data.transport.position_beats = 6.0;
        app.apply_auto_select_under_cursor();
        assert!(
            app.arrange_view_state
                .audio_events
                .selected_ids()
                .is_empty(),
            "nothing is under the cursor between the events"
        );
    }

    #[test]
    fn only_the_active_loop_take_reaches_playback() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.set_loop_range(0.0, 8.0);
        let created = project
            .add_loop_take_clips(0, PathBuf::from("take.wav"), 0.0, 12.0, 0.0, 8.0)
            .expect("three passes");
        assert_eq!(created.len(), 3);

        let audible = project.tracks[0]
            .clips
            .iter()
            .filter(|clip| clip_is_audible(clip))
            .count();
        assert_eq!(audible, 1, "one take plays, the other two are kept");
        // Trying another take moves playback without deleting audio.
        assert!(project.set_active_take(created[0]));
        assert_eq!(
            project.tracks[0]
                .clips
                .iter()
                .filter(|clip| clip_is_audible(clip))
                .count(),
            1
        );
        assert!(project.tracks[0].clips[0].take_active);
        assert_eq!(project.tracks[0].clips.len(), 3);
    }

    #[test]
    fn the_take_command_is_one_labelled_undo_step() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        app.data.project.set_loop_range(0.0, 8.0);
        let created = app
            .data
            .project
            .add_loop_take_clips(0, PathBuf::from("take.wav"), 0.0, 12.0, 0.0, 8.0)
            .expect("passes");
        app.project_history.clear();

        app.arrange_view_state
            .audio_events
            .select_many([created[0]]);
        app.make_selected_event_active_take();
        assert!(app.status_message.contains("now plays"));
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Select Take");
        assert!(app.data.project.tracks[0].clips[0].take_active);

        // A plain event that is not part of a loop recording is refused.
        let plain = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("plain.wav"), 20.0, 1.0)
            .unwrap();
        app.arrange_view_state.audio_events.select_many([plain]);
        app.make_selected_event_active_take();
        assert!(app.status_message.contains("not one of several takes"));

        // Several selected events are refused too.
        app.arrange_view_state
            .audio_events
            .select_many([created[0], created[1]]);
        app.make_selected_event_active_take();
        assert!(app.status_message.contains("one take at a time"));
    }

    #[test]
    fn app_lock_group_and_ungroup_are_single_undo_steps() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let first = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let second = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("b.wav"), 8.0, 2.0)
            .unwrap();
        app.project_history.clear();

        // Lock the first event.
        app.arrange_view_state.audio_events.select_many([first]);
        app.set_selected_audio_events_locked(true);
        assert!(app.data.project.tracks[0].clips[0].locked);
        assert!(!app.data.project.tracks[0].clips[1].locked);
        assert!(app.status_message.contains("Locked 1 audio event"));

        // A second lock reports that nothing changed.
        app.set_selected_audio_events_locked(true);
        assert!(app.status_message.contains("already locked"));

        // Unlock it again.
        app.set_selected_audio_events_locked(false);
        assert!(!app.data.project.tracks[0].clips[0].locked);
        assert!(app.status_message.contains("Unlocked"));

        // Group both, then ungroup one member (which releases both).
        app.arrange_view_state
            .audio_events
            .select_many([first, second]);
        app.group_selected_audio_events();
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .all(|clip| clip.group.is_some())
        );
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids().len(),
            2,
            "grouping keeps both members selected"
        );
        assert!(app.status_message.contains("Grouped"));

        app.arrange_view_state.audio_events.select_many([second]);
        app.ungroup_selected_audio_events();
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .all(|clip| clip.group.is_none())
        );
        assert!(app.status_message.contains("Ungrouped 2 audio event"));

        // One event alone cannot be grouped.
        app.arrange_view_state.audio_events.select_many([first]);
        app.group_selected_audio_events();
        assert!(app.status_message.contains("at least two"));
        assert!(app.modified);
    }

    #[test]
    fn a_locked_event_drag_selects_without_editing() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let id = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        app.data.project.set_audio_clips_locked(&[id], true);

        // The core refuses the move even if a gesture built one.
        let mut moved = app.data.project.tracks[0].clips[0].clone();
        moved.start_beats = 6.0;
        assert!(
            !app.data.project.move_audio_clips(&[(
                app.data.project.tracks[0].id,
                app.data.project.tracks[0].id,
                moved
            )]),
            "a locked event does not move"
        );
        assert_eq!(app.data.project.tracks[0].clips[0].start_beats, 0.0);
        assert_eq!(app.data.project.expand_selection_to_groups(&[id]), vec![id]);
    }

    #[test]
    fn select_under_cursor_and_move_to_locator_work_on_the_selection() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        // 2 s at 120 BPM is 4 beats: the first spans 0-4, the second 8-12.
        let first = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let second = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("b.wav"), 8.0, 2.0)
            .unwrap();
        app.project_history.clear();

        // Beat 10 is inside only the later event.
        app.data.transport.position_beats = 10.0;
        app.select_audio_events_under_cursor();
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids(),
            vec![second]
        );

        // Beat 4 is the exclusive end of the first event.
        app.data.transport.position_beats = 4.0;
        app.select_audio_events_under_cursor();
        assert!(
            app.arrange_view_state
                .audio_events
                .selected_ids()
                .is_empty()
                || app.arrange_view_state.audio_events.selected_ids() == vec![second],
            "the first event does not cover its end: {:?}",
            app.arrange_view_state.audio_events.selected_ids()
        );

        // Move the later event to the left locator at beat 2.
        app.data.project.set_loop_range(2.0, 16.0);
        app.arrange_view_state.audio_events.select_many([second]);
        app.move_selected_audio_events_to_locator(true);
        let moved = app.data.project.tracks[0]
            .clips
            .iter()
            .find(|clip| clip.id == second)
            .expect("event still present");
        assert!((moved.start_beats - 2.0).abs() < 1e-9);
        assert!(app.modified);

        // The unselected event did not move.
        let unmoved = app.data.project.tracks[0]
            .clips
            .iter()
            .find(|clip| clip.id == first)
            .expect("first event still present");
        assert_eq!(unmoved.start_beats, 0.0);
    }

    #[test]
    fn range_selection_and_locators_follow_the_loop_and_the_selection() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.tempo_bpm = 120.0;
        app.data.project.add_track(TrackKind::Audio);
        let early = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let late = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("b.wav"), 20.0, 2.0)
            .unwrap();

        // The loop covers beats 0 through 8, so only the first event overlaps.
        app.data.project.loop_start_beats = 0.0;
        app.data.project.loop_end_beats = 8.0;
        app.select_audio_events_in_loop();
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids(),
            vec![early]
        );

        // The first event is 2 s at 120 BPM, so it spans beats 0 through 4.
        app.set_locator_from_selection(false);
        assert!((app.data.project.loop_end_beats - 4.0).abs() < 1e-9);
        app.set_locator_from_selection(true);
        assert_eq!(app.data.project.loop_start_beats, 0.0);

        // Selecting the later event moves the right locator to beat 24.
        app.arrange_view_state.audio_events.select_many([late]);
        app.set_locator_from_selection(false);
        assert!((app.data.project.loop_end_beats - 24.0).abs() < 1e-9);
        assert!(app.modified);
    }

    #[test]
    fn muted_events_are_excluded_from_playback_but_kept_in_the_project() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        let clip = &project.tracks[0].clips[0];
        assert!(clip_is_audible(clip));
        project.set_audio_clips_muted(&[id], true);
        let clip = &project.tracks[0].clips[0];
        assert!(!clip_is_audible(clip));
        assert_eq!(clip.start_beats, 0.0, "the event keeps its position");
    }

    #[test]
    fn app_actions_duplicate_mute_and_select_events_as_undo_steps() {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.data.project.add_track(TrackKind::Audio);
        let id = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("a.wav"), 0.0, 2.0)
            .unwrap();
        app.project_history.clear();

        // Duplicate: the copy is created and becomes the selection.
        app.arrange_view_state.audio_events.select_many([id]);
        app.duplicate_selected_audio_events();
        assert_eq!(app.data.project.tracks[0].clips.len(), 2);
        let selected = app.arrange_view_state.audio_events.selected_ids();
        assert_eq!(selected.len(), 1);
        assert_ne!(selected[0], id, "the duplicate is selected");
        assert!(app.modified);

        // Mute the duplicate, then toggle it back.
        app.set_selected_audio_events_muted(true);
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .any(|clip| clip.id == selected[0] && clip.muted)
        );
        app.toggle_selected_audio_events_muted();
        assert!(
            app.data.project.tracks[0]
                .clips
                .iter()
                .all(|clip| !clip.muted)
        );

        // Select all / none work across the whole project.
        app.select_all_audio_events();
        assert_eq!(app.arrange_view_state.audio_events.selected_ids().len(), 2);
        app.select_no_audio_events();
        assert!(
            app.arrange_view_state
                .audio_events
                .selected_ids()
                .is_empty()
        );
    }

    #[test]
    fn split_selected_clips_splits_each_crossing_event_atomically() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("first.wav"), 0.0, 4.0)
            .unwrap();
        let second = project
            .add_audio_clip(1, PathBuf::from("second.wav"), 0.0, 4.0)
            .unwrap();
        let selected = HashSet::from([first, second]);
        assert_eq!(split_selected_clips(&mut project, &selected, 4.0), 2);
        assert_eq!(project.tracks[0].clips.len(), 2);
        assert_eq!(project.tracks[1].clips.len(), 2);
        assert!(project.validate().is_ok());
    }

    #[test]
    fn split_selected_clips_leaves_project_unchanged_when_cursor_misses() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let clip = project
            .add_audio_clip(0, PathBuf::from("event.wav"), 0.0, 1.0)
            .unwrap();
        let before = project.clone();
        assert_eq!(
            split_selected_clips(&mut project, &HashSet::from([clip]), 4.0),
            0
        );
        assert_eq!(project, before);
    }

    #[test]
    fn split_selected_clips_rolls_back_when_only_some_cross_the_cursor() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Audio);
        let crossing = project
            .add_audio_clip(0, PathBuf::from("crossing.wav"), 0.0, 4.0)
            .unwrap();
        let elsewhere = project
            .add_audio_clip(0, PathBuf::from("elsewhere.wav"), 8.0, 2.0)
            .unwrap();
        let before = project.clone();
        assert_eq!(
            split_selected_clips(&mut project, &HashSet::from([crossing, elsewhere]), 4.0),
            0
        );
        assert_eq!(project, before);
    }
}

#[cfg(test)]
mod crossfade_selection_tests {
    use super::*;

    #[test]
    fn crossfade_selection_applies_only_to_two_overlapping_same_track_clips() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("first.wav"), 0.0, 2.0)
            .unwrap();
        let second = project
            .add_audio_clip(0, PathBuf::from("second.wav"), 0.5, 2.0)
            .unwrap();
        assert!(crossfade_selection(&mut project, &[first, second]));
        assert!(project.validate().is_ok());
    }

    #[test]
    fn crossfade_selection_rejects_disjoint_and_wrong_count_without_mutation() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("first.wav"), 0.0, 1.0)
            .unwrap();
        let second = project
            .add_audio_clip(0, PathBuf::from("second.wav"), 2.0, 1.0)
            .unwrap();
        let before = project.clone();
        assert!(!crossfade_selection(&mut project, &[first, second]));
        assert_eq!(project, before);
        assert!(!crossfade_selection(&mut project, &[first]));
        assert_eq!(project, before);
    }
}

#[cfg(test)]
mod move_to_cursor_tests {
    use super::*;

    #[test]
    fn moving_selection_aligns_earliest_clip_and_preserves_offsets() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        project.add_track(TrackKind::Audio);
        let first = project
            .add_audio_clip(0, PathBuf::from("first.wav"), 2.0, 1.0)
            .unwrap();
        let second = project
            .add_audio_clip(1, PathBuf::from("second.wav"), 5.0, 1.0)
            .unwrap();
        assert_eq!(
            move_selected_clips_to_cursor(
                &mut project,
                &HashSet::from([first, second]),
                8.0,
                false,
            ),
            2
        );
        assert_eq!(project.tracks[0].clips[0].start_beats, 8.0);
        assert_eq!(project.tracks[1].clips[0].start_beats, 11.0);
    }

    #[test]
    fn moving_selection_rejects_stale_ids_without_partial_changes() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let clip = project
            .add_audio_clip(0, PathBuf::from("event.wav"), 2.0, 1.0)
            .unwrap();
        let before = project.clone();
        assert_eq!(
            move_selected_clips_to_cursor(
                &mut project,
                &HashSet::from([clip, u64::MAX]),
                8.0,
                false,
            ),
            0
        );
        assert_eq!(project, before);
    }
}
