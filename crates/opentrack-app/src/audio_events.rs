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
impl State {
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

impl OpenTrackApp {
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
        if self.is_recording() {
            self.status_message = "Stop recording before pasting audio events".into();
            return;
        }
        let Some(clipboard) = self.audio_event_clipboard.clone() else {
            self.status_message = "Copy audio events before pasting".into();
            return;
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
        if target_group_id.is_some() && group_destinations.as_ref().is_none_or(Vec::is_empty) {
            self.status_message = "The destination group has no audio tracks".into();
            return;
        }
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
            self.status_message = "Choose an audio track to paste onto".into();
            return;
        };
        let mut paste = Vec::with_capacity(clipboard.items.len());
        for item in &clipboard.items {
            let destination_track_id = if let Some(destinations) = &group_destinations {
                let Some(track_id) = destinations.get(item.track_offset) else {
                    self.status_message =
                        "The destination group does not have enough audio tracks".into();
                    return;
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
                self.status_message =
                    "The selected destination tracks do not match the copied events".into();
                return;
            };
            let Some(track) = self.data.project.tracks.get(track_index) else {
                self.status_message =
                    "The selected destination tracks do not match the copied events".into();
                return;
            };
            if track.kind != TrackKind::Audio || track.audio_channels != item.channels {
                self.status_message =
                    "Paste requires audio tracks with matching channel widths".into();
                return;
            }
            let mut clip = item.clip.clone();
            clip.start_beats = self.data.transport.position_beats.max(0.0)
                + (item.clip.start_beats - clipboard.anchor_beat);
            paste.push((track_index, clip));
        }
        if !self.project_history.begin(&self.data.project) {
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
/// Only cuts shared by every populated child track create selectable pieces.
/// A shared cut is either a group split marker with matching child boundaries
/// or source-contiguous split clips on every child track. Independent clip
/// starts/ends stay hidden inside the continuous group event.
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
    explicit_cuts.retain(|beat| {
        beat.is_finite()
            && *beat > start + 1e-6
            && *beat < end - 1e-6
            && child_tracks.iter().all(|track| {
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
fn snap(value: f64, grid: Option<f64>) -> f64 {
    grid.map_or(value.max(0.0), |step| {
        crate::timeline::snap_beats_to_grid(value, step)
    })
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
) {
    let beats = ((pos.x - origin) / px) as f64;
    let original = &g.original;
    let mut c = original.clone();
    g.target = Some(g.track);
    match g.tool {
        Tool::Move => {
            c.start_beats = snap(beats - g.grab_beats, grid);
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
            c.start_beats =
                snap(beats, grid).clamp(earliest, (end - min_seconds * tempo / 60.0).max(earliest));
            let delta = (c.start_beats - original.start_beats) * 60.0 / tempo;
            c.source_offset_seconds = (original.source_offset_seconds + delta).max(0.0);
            c.duration_seconds = (original.duration_seconds - delta).max(min_seconds);
        }
        Tool::Right => {
            let end = snap(beats, grid);
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
    let alpha = if ghost { 0.28 } else { 1.0 };
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
) -> Outcome {
    toolbox(ui, visible, &mut state.toolbox, edit_tool);
    let mut split = None;
    let mut group_split = None;
    let mut cut_preview = None;
    let mut out = Outcome {
        changed: false,
        scroll: 0.0,
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
                let moved = project.move_audio_clips(&edits);
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
                        state.select_one(c.id);
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
                        state.select_one(c.id);
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
                    if !was_selected {
                        state.select_one(c.id);
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
        if ids.len() == 2 && project.crossfade_audio_clips(ids[0], ids[1]) {
            out.changed = true;
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
            };
            f.frame(vec![]);
            f.frame(vec![]);
            f
        }
        fn frame(&mut self, events: Vec<egui::Event>) -> bool {
            self.time += 1.0 / 60.0;
            let mut changed = false;
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 230.0))),
                    events,
                    time: Some(self.time),
                    ..Default::default()
                },
                |ui| {
                    changed = show_with_tool(
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
                    )
                    .changed;
                },
            );
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
        assert!(f.frame(vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Key {
                key: egui::Key::X,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE
            }
        ]));
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
        history.begin(&before);
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
