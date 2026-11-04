//! Application preparation of immutable playback automation.
use super::egui::{self, Color32, Id, Pos2, Rect, Sense, Stroke};
use opentrack_core::automation::{move_target_point, remove_target_point, set_target_point};
use opentrack_core::project::{AutomationLane, AutomationTarget, Track};
use std::sync::Arc;

pub(super) fn playback_volume_automation(track: &Track) -> Option<Arc<AutomationLane>> {
    playback_target_automation(track, AutomationTarget::TrackVolume)
}

pub(super) fn playback_target_automation(
    track: &Track,
    target: AutomationTarget,
) -> Option<Arc<AutomationLane>> {
    if !track.automation_read {
        return None;
    }
    track
        .automation_lanes
        .iter()
        .find(|lane| lane.target == target && !lane.points.is_empty() && lane.is_valid())
        .map(|lane| Arc::new(lane.clone()))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum LaneAction {
    Set {
        track_id: u64,
        target: AutomationTarget,
        beat: f64,
        value: f32,
    },
    Move {
        track_id: u64,
        target: AutomationTarget,
        index: usize,
        beat: f64,
        value: f32,
    },
    Remove {
        track_id: u64,
        target: AutomationTarget,
        index: usize,
    },
    /// Escape during a drag: the caller restores the curve captured when the
    /// gesture began and drops the pending undo entry.
    CancelDrag { track_id: u64 },
}

/// One captured write gesture: monotonic intervals of `(beat, gain)` samples.
///
/// The transport can loop while the fader is held, so a gesture is a sequence of
/// intervals rather than one range; writing them separately keeps material
/// between them untouched.
#[derive(Clone, Default)]
pub(super) struct WriteCapture {
    track_id: Option<u64>,
    target: Option<AutomationTarget>,
    intervals: Vec<Vec<(f64, f32)>>,
    samples: usize,
    overflowed: bool,
}

/// Bound on samples per gesture. At the control rate this is minutes of fader
/// movement; exceeding it aborts the gesture instead of writing a partial curve.
pub(super) const MAX_WRITE_SAMPLES: usize = 4096;

impl WriteCapture {
    pub(super) fn start(&mut self, track_id: u64, target: AutomationTarget, beat: f64, value: f32) {
        self.track_id = Some(track_id);
        self.target = Some(target);
        self.intervals.clear();
        self.intervals.push(vec![(beat, value)]);
        self.samples = 1;
        self.overflowed = false;
    }

    pub(super) fn record(&mut self, beat: f64, value: f32) {
        if self.overflowed || !beat.is_finite() || !value.is_finite() {
            return;
        }
        if self.samples >= MAX_WRITE_SAMPLES {
            self.overflowed = true;
            return;
        }
        let Some(current) = self.intervals.last_mut() else {
            return;
        };
        let last_beat = current.last().map(|sample| sample.0).unwrap_or(beat);
        if beat < last_beat - 1e-9 {
            // The transport looped: start a separate interval.
            self.intervals.push(vec![(beat, value)]);
            self.samples += 1;
            return;
        }
        if beat <= last_beat + 1e-9 {
            return;
        }
        current.push((beat, value));
        self.samples += 1;
    }

    /// Intervals of at least two samples, or an error when the gesture cannot be
    /// written. The capture is consumed either way.
    pub(super) fn finish(
        &mut self,
    ) -> Result<(u64, AutomationTarget, Vec<(f64, f64, Vec<(f64, f32)>)>), String> {
        let track_id = self.track_id.take();
        let target = self.target.take().unwrap_or(AutomationTarget::TrackVolume);
        let overflowed = std::mem::replace(&mut self.overflowed, false);
        let intervals = std::mem::take(&mut self.intervals);
        self.samples = 0;
        let Some(track_id) = track_id else {
            return Ok((0, target, Vec::new()));
        };
        if overflowed {
            return Err("Automation write was too long and was discarded".to_owned());
        }
        Ok((
            track_id,
            target,
            intervals
                .into_iter()
                .filter(|samples| samples.len() >= 2)
                .map(|samples| {
                    let start = samples[0].0;
                    let end = samples[samples.len() - 1].0;
                    (start, end, samples)
                })
                .collect(),
        ))
    }

    pub(super) fn cancel(&mut self) {
        self.track_id = None;
        self.target = None;
        self.intervals.clear();
        self.samples = 0;
        self.overflowed = false;
    }

    pub(super) fn is_active(&self) -> bool {
        self.track_id.is_some()
    }
}

const TRANSPORT_KEY: &str = "automation-transport";
const WRITE_KEY: &str = "automation-write-capture";
const WRITE_FINISHED_KEY: &str = "automation-write-finished";
const WRITE_STARTED_KEY: &str = "automation-write-started";
const WRITE_POINTS_KEY: &str = "automation-write-points";

/// Publishes the transport state the faders need for write capture.
pub(super) fn publish_transport(ui: &egui::Ui, playing: bool, position_beats: f64) {
    ui.ctx()
        .data_mut(|data| data.insert_temp(Id::new(TRANSPORT_KEY), (playing, position_beats)));
}

fn write_capture(ui: &egui::Ui, update: impl FnOnce(&mut WriteCapture)) {
    ui.ctx().data_mut(|data| {
        let mut capture = data
            .get_temp::<WriteCapture>(Id::new(WRITE_KEY))
            .unwrap_or_default();
        update(&mut capture);
        data.insert_temp(Id::new(WRITE_KEY), capture);
    });
}

/// Captures explicit fader movement for a Write-enabled track while playing.
///
/// Values are timestamped with the transport position on the control thread;
/// the audio callback is never involved and no project data is touched here.
pub(super) fn capture_fader_write(
    ui: &egui::Ui,
    track_id: u64,
    response: &egui::Response,
    gain: f32,
) {
    capture_control_write(ui, track_id, AutomationTarget::TrackVolume, response, gain);
}

pub(super) fn capture_channel_fader_write(
    ui: &egui::Ui,
    channel_id: u64,
    response: &egui::Response,
    gain: f32,
) {
    let target = match channel_id {
        opentrack_core::project::INPUT_BUS_ID => AutomationTarget::InputBusVolume,
        opentrack_core::project::OUTPUT_BUS_ID => AutomationTarget::OutputBusVolume,
        _ => AutomationTarget::TrackVolume,
    };
    capture_control_write(ui, channel_id, target, response, gain);
}

pub(super) fn capture_channel_pan_write(
    ui: &egui::Ui,
    channel_id: u64,
    response: &egui::Response,
    pan: f32,
) {
    let target = match channel_id {
        opentrack_core::project::INPUT_BUS_ID => AutomationTarget::InputBusPan,
        opentrack_core::project::OUTPUT_BUS_ID => AutomationTarget::OutputBusPan,
        _ => AutomationTarget::TrackPan,
    };
    capture_control_write(ui, channel_id, target, response, pan);
}

pub(super) fn capture_pan_write(ui: &egui::Ui, track_id: u64, response: &egui::Response, pan: f32) {
    capture_control_write(ui, track_id, AutomationTarget::TrackPan, response, pan);
}

pub(super) fn capture_send_level_write(
    ui: &egui::Ui,
    track_id: u64,
    slot: u8,
    response: &egui::Response,
    level_db: f32,
) {
    let gain = if level_db <= -90.0 {
        0.0
    } else {
        10.0_f32.powf(level_db.clamp(-90.0, 12.0) / 20.0)
    };
    capture_control_write(
        ui,
        track_id,
        AutomationTarget::SendLevel(slot),
        response,
        gain,
    );
}

pub(super) fn capture_send_pan_write(
    ui: &egui::Ui,
    track_id: u64,
    slot: u8,
    response: &egui::Response,
    pan: f32,
) {
    capture_control_write(ui, track_id, AutomationTarget::SendPan(slot), response, pan);
}

fn capture_control_write(
    ui: &egui::Ui,
    track_id: u64,
    target: AutomationTarget,
    response: &egui::Response,
    value: f32,
) {
    let Some((playing, position)) =
        ui.data(|data| data.get_temp::<(bool, f64)>(Id::new(TRANSPORT_KEY)))
    else {
        return;
    };
    if !playing {
        return;
    }
    if response.drag_started() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(Id::new(WRITE_STARTED_KEY), true));
        write_capture(ui, |capture| {
            capture.start(track_id, target, position, value)
        });
    } else if response.dragged() {
        write_capture(ui, |capture| capture.record(position, value));
    } else if response.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(Id::new(WRITE_FINISHED_KEY), true));
    }
}

pub(super) fn take_write_started(ctx: &egui::Context) -> bool {
    ctx.data_mut(|data| {
        data.remove_temp::<bool>(Id::new(WRITE_STARTED_KEY))
            .unwrap_or(false)
    })
}

pub(super) fn cancel_write(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        data.remove_temp::<WriteCapture>(Id::new(WRITE_KEY));
        data.remove_temp::<bool>(Id::new(WRITE_FINISHED_KEY));
        data.remove_temp::<bool>(Id::new(WRITE_STARTED_KEY));
    });
}

pub(super) fn capture_target_write(
    ui: &egui::Ui,
    track_id: u64,
    target: AutomationTarget,
    response: &egui::Response,
    value: f32,
) {
    capture_control_write(ui, track_id, target, response, value);
}

/// Queue a discrete mute control change during playback Write mode. Applying
/// the point is deferred to the app's project/history phase, off the draw path.
pub(super) fn queue_mute_write(ui: &egui::Ui, track_id: u64, muted: bool) {
    let Some((playing, beat)) =
        ui.data(|data| data.get_temp::<(bool, f64)>(Id::new(TRANSPORT_KEY)))
    else {
        return;
    };
    if !playing || !beat.is_finite() || beat < 0.0 {
        return;
    }
    ui.ctx().data_mut(|data| {
        let mut points = data
            .get_temp::<Vec<(u64, f64, f32)>>(Id::new(WRITE_POINTS_KEY))
            .unwrap_or_default();
        points.push((track_id, beat, if muted { 1.0 } else { 0.0 }));
        data.insert_temp(Id::new(WRITE_POINTS_KEY), points);
    });
}

pub(super) fn take_mute_writes(ctx: &egui::Context) -> Vec<(u64, f64, f32)> {
    ctx.data_mut(|data| {
        data.remove_temp::<Vec<(u64, f64, f32)>>(Id::new(WRITE_POINTS_KEY))
            .unwrap_or_default()
    })
}

/// Takes a finished capture so the caller can apply it with project history.
pub(super) fn take_finished_write(ctx: &egui::Context) -> Option<WriteCapture> {
    let finished = ctx.data_mut(|data| data.remove_temp::<bool>(Id::new(WRITE_FINISHED_KEY)));
    finished?;
    ctx.data_mut(|data| data.remove_temp::<WriteCapture>(Id::new(WRITE_KEY)))
}

const FOLLOW_KEY: &str = "automation-follow-gains";

/// Gain a read-enabled track displays at `position_beats`.
///
/// The engine evaluates the same lane per sample; the fader only needs the
/// control-rate value at the current transport position, so it reads the lane
/// directly instead of sampling the callback.
pub(super) fn follow_gain(track: &Track, position_beats: f64) -> Option<f32> {
    if !track.automation_read {
        return None;
    }
    let target = match track.id {
        opentrack_core::project::INPUT_BUS_ID => AutomationTarget::InputBusVolume,
        opentrack_core::project::OUTPUT_BUS_ID => AutomationTarget::OutputBusVolume,
        _ => AutomationTarget::TrackVolume,
    };
    let lane = track
        .automation_lanes
        .iter()
        .find(|lane| lane.target == target && lane.is_valid() && !lane.points.is_empty())?;
    lane.value_at(position_beats)
        .filter(|value| value.is_finite())
}

/// Publishes the follow gains for the faders drawn this frame.
pub(super) fn publish_follow_gains(
    ui: &egui::Ui,
    project: &opentrack_core::project::Project,
    playing: bool,
    position_beats: f64,
) {
    let mut gains = std::collections::HashMap::new();
    let mut pans = std::collections::HashMap::new();
    let mut mutes = std::collections::HashMap::new();
    if playing {
        for track in project.channels() {
            if let Some(gain) = follow_gain(track, position_beats) {
                gains.insert(track.id, gain);
            }
            let target = match track.id {
                opentrack_core::project::INPUT_BUS_ID => AutomationTarget::InputBusPan,
                opentrack_core::project::OUTPUT_BUS_ID => AutomationTarget::OutputBusPan,
                _ => AutomationTarget::TrackPan,
            };
            if track.automation_read
                && let Some(value) = track
                    .automation_lanes
                    .iter()
                    .find(|lane| lane.target == target && lane.is_valid())
                    .and_then(|lane| lane.value_at(position_beats))
            {
                pans.insert(track.id, value);
            }
            let target = match track.id {
                opentrack_core::project::INPUT_BUS_ID => AutomationTarget::InputBusMute,
                opentrack_core::project::OUTPUT_BUS_ID => AutomationTarget::OutputBusMute,
                _ => AutomationTarget::TrackMute,
            };
            if track.automation_read
                && let Some(value) = track
                    .automation_lanes
                    .iter()
                    .find(|lane| lane.target == target && lane.is_valid())
                    .and_then(|lane| lane.value_at(position_beats))
            {
                mutes.insert(track.id, value);
            }
        }
    }
    ui.ctx().data_mut(|data| {
        data.insert_temp(Id::new(FOLLOW_KEY), gains);
        data.insert_temp(Id::new("automation-follow-pans"), pans);
        data.insert_temp(Id::new("automation-follow-mutes"), mutes);
    });
}

/// Gain a fader should draw: the automation value, unless the pointer is in use.
///
/// Grabbing a fader switches that strip to manual control for the gesture; the
/// base gain is what the gesture edits, and automation resumes on release.
pub(super) fn fader_gain(ui: &egui::Ui, track_id: u64, base: f32) -> f32 {
    if ui.ctx().egui_is_using_pointer() {
        return base;
    }
    ui.data(|data| data.get_temp::<std::collections::HashMap<u64, f32>>(Id::new(FOLLOW_KEY)))
        .and_then(|gains| gains.get(&track_id).copied())
        .unwrap_or(base)
}

pub(super) fn followed_pan(ui: &egui::Ui, track_id: u64, base: f32) -> f32 {
    ui.data(|data| {
        data.get_temp::<std::collections::HashMap<u64, f32>>(Id::new("automation-follow-pans"))
    })
    .and_then(|values| values.get(&track_id).copied())
    .unwrap_or(base)
}

pub(super) fn followed_mute(ui: &egui::Ui, track_id: u64, base: bool) -> bool {
    ui.data(|data| {
        data.get_temp::<std::collections::HashMap<u64, f32>>(Id::new("automation-follow-mutes"))
    })
    .and_then(|values| values.get(&track_id).copied())
    .map_or(base, |value| value >= 0.5)
}

pub(super) fn plugin_follow_value(
    channel: Option<&Track>,
    target: AutomationTarget,
    beat: f64,
    base: f32,
) -> f32 {
    channel
        .filter(|track| track.automation_read)
        .and_then(|track| {
            track
                .automation_lanes
                .iter()
                .find(|lane| lane.target == target && lane.is_valid())
                .and_then(|lane| lane.value_at(beat))
        })
        .unwrap_or(base)
}

pub(super) fn apply_lane_action(
    project: &mut opentrack_core::project::Project,
    action: LaneAction,
) -> Result<bool, String> {
    use opentrack_core::project::AutomationPoint;
    let track_id = match &action {
        LaneAction::Set { track_id, .. }
        | LaneAction::Move { track_id, .. }
        | LaneAction::Remove { track_id, .. }
        | LaneAction::CancelDrag { track_id } => *track_id,
    };
    if matches!(
        track_id,
        opentrack_core::project::INPUT_BUS_ID | opentrack_core::project::OUTPUT_BUS_ID
    ) {
        return apply_bus_lane_action(project, action);
    }
    let result = match action {
        LaneAction::Set {
            track_id,
            target,
            beat,
            value,
        } => set_target_point(project, track_id, target, AutomationPoint { beat, value }),
        LaneAction::Move {
            track_id,
            target,
            index,
            beat,
            value,
        } => move_target_point(
            project,
            track_id,
            target,
            index,
            AutomationPoint { beat, value },
        ),
        LaneAction::Remove {
            track_id,
            target,
            index,
        } => remove_target_point(project, track_id, target, index),
        // Restoring the pre-gesture curve is the caller's job: it owns history.
        LaneAction::CancelDrag { .. } => return Ok(false),
    };
    result.map_err(|error| error.to_string())
}

fn apply_bus_lane_action(
    project: &mut opentrack_core::project::Project,
    action: LaneAction,
) -> Result<bool, String> {
    use opentrack_core::project::AutomationPoint;
    let (track_id, target) = match &action {
        LaneAction::Set {
            track_id, target, ..
        }
        | LaneAction::Move {
            track_id, target, ..
        }
        | LaneAction::Remove {
            track_id, target, ..
        } => (*track_id, *target),
        LaneAction::CancelDrag { .. } => return Ok(false),
    };
    let expected = if track_id == opentrack_core::project::INPUT_BUS_ID {
        matches!(
            target,
            AutomationTarget::InputBusVolume
                | AutomationTarget::InputBusPan
                | AutomationTarget::InputBusMute
        )
    } else {
        matches!(
            target,
            AutomationTarget::OutputBusVolume
                | AutomationTarget::OutputBusPan
                | AutomationTarget::OutputBusMute
        )
    };
    if !expected {
        return Err("Automation target does not belong to this device bus".to_owned());
    }
    let result = match action {
        LaneAction::Set { beat, value, .. } => opentrack_core::automation::set_target_point(
            project,
            track_id,
            target,
            AutomationPoint { beat, value },
        ),
        LaneAction::Move {
            index, beat, value, ..
        } => opentrack_core::automation::move_target_point(
            project,
            track_id,
            target,
            index,
            AutomationPoint { beat, value },
        ),
        LaneAction::Remove { index, .. } => {
            opentrack_core::automation::remove_target_point(project, track_id, target, index)
        }
        LaneAction::CancelDrag { .. } => return Ok(false),
    };
    result.map_err(|error| error.to_string())
}

pub(super) fn draw_lane(
    ui: &mut egui::Ui,
    rect: Rect,
    track_id: u64,
    points: &[(f64, f32)],
    grid_left: f32,
    pixels_per_beat: f32,
    snap: Option<f64>,
) -> Option<LaneAction> {
    draw_target_lane(
        ui,
        rect,
        track_id,
        AutomationTarget::TrackVolume,
        points,
        grid_left,
        pixels_per_beat,
        snap,
    )
}

pub(super) fn draw_target_lane(
    ui: &mut egui::Ui,
    rect: Rect,
    track_id: u64,
    target: AutomationTarget,
    points: &[(f64, f32)],
    grid_left: f32,
    pixels_per_beat: f32,
    snap: Option<f64>,
) -> Option<LaneAction> {
    let id = if target == AutomationTarget::TrackVolume {
        Id::new(("volume-automation-lane", track_id))
    } else {
        Id::new(("automation-lane", track_id, target))
    };
    let is_volume = matches!(
        target,
        AutomationTarget::TrackVolume
            | AutomationTarget::InputBusVolume
            | AutomationTarget::OutputBusVolume
            | AutomationTarget::SendLevel(_)
    );
    let is_pan = matches!(
        target,
        AutomationTarget::TrackPan
            | AutomationTarget::InputBusPan
            | AutomationTarget::OutputBusPan
            | AutomationTarget::SendPan(_)
    );
    let is_mute = matches!(
        target,
        AutomationTarget::TrackMute
            | AutomationTarget::InputBusMute
            | AutomationTarget::OutputBusMute
    );
    let value_to_position = |value: f32| {
        if is_volume {
            super::channel_gain::gain_to_position(value)
        } else if is_pan {
            (1.0 - value.clamp(-1.0, 1.0)) * 0.5
        } else if is_mute {
            if value >= 0.5 { 0.15 } else { 0.85 }
        } else {
            value.clamp(0.0, 1.0)
        }
    };
    let position_to_value = |position: f32| {
        if is_volume {
            super::channel_gain::position_to_gain(position)
        } else if is_pan {
            1.0 - 2.0 * position
        } else if is_mute {
            if position < 0.5 { 1.0 } else { 0.0 }
        } else {
            position.clamp(0.0, 1.0)
        }
    };
    let response = ui.interact(rect.expand(8.0), id, Sense::click_and_drag());
    let position = |pos: Pos2| {
        let beat = f64::from((pos.x - grid_left) / pixels_per_beat.max(1.0)).max(0.0);
        let beat = snap.map_or(beat, |step| crate::timeline::snap_beats_to_grid(beat, step));
        let position = ((pos.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        (beat, position_to_value(position))
    };
    let painter = ui.painter().with_clip_rect(rect);
    for index in 1..points.len() {
        let (beat_a, value_a) = points[index - 1];
        let (beat_b, value_b) = points[index];
        let point_pos = |beat: f64, value: f32| {
            Pos2::new(
                grid_left + beat as f32 * pixels_per_beat,
                rect.top() + value_to_position(value) * rect.height(),
            )
        };
        if is_mute {
            let left = point_pos(beat_a, value_a);
            let right = point_pos(beat_b, value_b);
            let step = Pos2::new(right.x, left.y);
            painter.line_segment(
                [left, step],
                Stroke::new(2.0, Color32::from_rgb(100, 200, 140)),
            );
            painter.line_segment(
                [step, right],
                Stroke::new(1.0, Color32::from_rgb(100, 200, 140)),
            );
        } else {
            painter.line_segment(
                [point_pos(beat_a, value_a), point_pos(beat_b, value_b)],
                Stroke::new(2.0, Color32::from_rgb(100, 200, 140)),
            );
        }
    }
    for (beat, value) in points {
        painter.circle_filled(
            Pos2::new(
                grid_left + *beat as f32 * pixels_per_beat,
                rect.top() + value_to_position(*value) * rect.height(),
            ),
            4.0,
            Color32::from_rgb(120, 225, 160),
        );
    }
    if ui.input(|input| input.key_pressed(egui::Key::Escape))
        && ui
            .memory(|memory| memory.data.get_temp::<usize>(id.with("dragging")))
            .is_some()
    {
        ui.memory_mut(|memory| {
            memory.data.remove::<usize>(id.with("dragging"));
            memory.data.remove::<usize>(id.with("selected"));
        });
        return Some(LaneAction::CancelDrag { track_id });
    }
    let selected = ui.memory(|memory| memory.data.get_temp::<usize>(id.with("selected")));
    if let Some((beat, value)) = selected.and_then(|index| points.get(index)) {
        painter.circle_stroke(
            Pos2::new(
                grid_left + *beat as f32 * pixels_per_beat,
                rect.top() + value_to_position(*value) * rect.height(),
            ),
            7.0,
            Stroke::new(1.5, Color32::WHITE),
        );
    }
    if response.drag_started()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let radius = 9.0;
        if let Some((index, _)) = points.iter().enumerate().find(|(_, (beat, value))| {
            let point_pos = Pos2::new(
                grid_left + *beat as f32 * pixels_per_beat,
                rect.top() + value_to_position(*value) * rect.height(),
            );
            point_pos.distance(pos) <= radius
        }) {
            ui.memory_mut(|memory| memory.data.insert_temp(id.with("dragging"), index));
            ui.memory_mut(|memory| memory.data.insert_temp(id.with("selected"), index));
        }
    }
    if response.dragged()
        && let (Some(index), Some(pos)) = (
            ui.memory(|memory| memory.data.get_temp::<usize>(id.with("dragging"))),
            response.interact_pointer_pos(),
        )
        && index < points.len()
    {
        let (mut beat, value) = position(pos);
        let lower = index
            .checked_sub(1)
            .and_then(|previous| points.get(previous))
            .map(|point| point.0 + 1e-6)
            .unwrap_or(0.0);
        let upper = points
            .get(index + 1)
            .map(|point| point.0 - 1e-6)
            .unwrap_or(f64::MAX);
        if lower <= upper {
            beat = beat.clamp(lower, upper);
        } else {
            beat = points[index].0;
        }
        if (points[index].0 - beat).abs() > 1e-9 || (points[index].1 - value).abs() > 1e-5 {
            return Some(LaneAction::Move {
                track_id,
                target,
                index,
                beat,
                value,
            });
        }
    }
    if response.drag_stopped() {
        ui.memory_mut(|memory| memory.data.remove::<usize>(id.with("dragging")));
    }
    if response.clicked_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let radius = 9.0;
        if let Some((index, _)) = points.iter().enumerate().find(|(_, (beat, value))| {
            let point_pos = Pos2::new(
                grid_left + *beat as f32 * pixels_per_beat,
                rect.top() + value_to_position(*value) * rect.height(),
            );
            point_pos.distance(pos) <= radius
        }) {
            ui.memory_mut(|memory| memory.data.insert_temp(id.with("selected"), index));
        } else {
            ui.memory_mut(|memory| memory.data.remove::<usize>(id.with("selected")));
            let (beat, value) = position(pos);
            if !points
                .iter()
                .any(|(existing, _)| (*existing - beat).abs() < 1e-9)
            {
                return Some(LaneAction::Set {
                    track_id,
                    target,
                    beat,
                    value,
                });
            }
        }
    }
    if response.clicked_by(egui::PointerButton::Secondary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let radius = 9.0;
        if let Some((index, _)) = points.iter().enumerate().find(|(_, (beat, value))| {
            let point_pos = Pos2::new(
                grid_left + *beat as f32 * pixels_per_beat,
                rect.top() + super::channel_gain::gain_to_position(*value) * rect.height(),
            );
            point_pos.distance(pos) <= radius
        }) {
            return Some(LaneAction::Remove {
                track_id,
                target,
                index,
            });
        }
    }
    if selected.is_some() {
        response.on_hover_text("Drag to move · Right-click a point to remove");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::project::{AutomationPoint, Project, TrackKind};

    #[test]
    fn read_flag_and_valid_nonempty_lane_control_playback_snapshot() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track = &mut project.tracks[0];
        track.gain = 0.3;
        track.automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![AutomationPoint {
                beat: 1.0,
                value: 0.6,
            }],
        });
        assert!(playback_volume_automation(track).is_none());
        track.automation_read = true;
        assert_eq!(
            playback_volume_automation(track).unwrap().value_at(1.0),
            Some(0.6)
        );
        assert_eq!(track.gain, 0.3);
        track.automation_lanes[0].points[0].value = f32::NAN;
        assert!(playback_volume_automation(track).is_none());
    }

    #[test]
    fn read_publishes_pan_and_mute_lanes_too() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track = &mut project.tracks[0];
        track.automation_lanes = vec![
            AutomationLane {
                target: AutomationTarget::TrackPan,
                points: vec![AutomationPoint {
                    beat: 1.0,
                    value: -0.75,
                }],
            },
            AutomationLane {
                target: AutomationTarget::TrackMute,
                points: vec![AutomationPoint {
                    beat: 2.0,
                    value: 1.0,
                }],
            },
        ];
        assert!(playback_target_automation(track, AutomationTarget::TrackPan).is_none());
        track.automation_read = true;
        assert_eq!(
            playback_target_automation(track, AutomationTarget::TrackPan)
                .unwrap()
                .value_at(1.0),
            Some(-0.75)
        );
        assert_eq!(
            playback_target_automation(track, AutomationTarget::TrackMute)
                .unwrap()
                .value_at(2.0),
            Some(1.0)
        );
    }

    #[test]
    fn isolated_lane_click_requests_snapped_point_insert() {
        let ctx = egui::Context::default();
        let track_id = 7;
        let mut action = None;
        let rect = Rect::from_min_size(Pos2::new(20.0, 20.0), egui::vec2(300.0, 80.0));
        let mut run = |ctx: &egui::Context,
                       events: Vec<egui::Event>,
                       action: &mut Option<LaneAction>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 200.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    *action = draw_lane(ui, rect, track_id, &[], 20.0, 10.0, Some(1.0));
                },
            );
            output.textures_delta.clear();
        };
        let pos = Pos2::new(75.0, 55.0);
        run(&ctx, vec![egui::Event::PointerMoved(pos)], &mut action);
        run(
            &ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut action,
        );
        run(
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut action,
        );
        assert!(matches!(
            action,
            Some(LaneAction::Set {
                track_id: 7,
                beat: 6.0,
                ..
            })
        ));
    }

    #[test]
    fn write_capture_splits_looped_gestures_into_intervals() {
        let mut capture = WriteCapture::default();
        capture.start(7, AutomationTarget::TrackVolume, 1.0, 0.5);
        capture.record(2.0, 0.6);
        capture.record(3.0, 0.7);
        // Transport loops back to the start of the cycle.
        capture.record(1.0, 0.8);
        capture.record(2.0, 0.9);
        let (track_id, target, intervals) = capture.finish().unwrap();
        assert_eq!(track_id, 7);
        assert_eq!(target, AutomationTarget::TrackVolume);
        assert_eq!(intervals.len(), 2);
        assert_eq!(intervals[0].0, 1.0);
        assert_eq!(intervals[0].1, 3.0);
        assert_eq!(intervals[1].0, 1.0);
        assert_eq!(intervals[1].1, 2.0);
        assert!(!capture.is_active());
    }

    #[test]
    fn write_capture_retains_the_parameter_target() {
        let mut capture = WriteCapture::default();
        capture.start(7, AutomationTarget::TrackPan, 1.0, -0.5);
        capture.record(2.0, 0.75);
        let (track_id, target, intervals) = capture.finish().unwrap();
        assert_eq!(track_id, 7);
        assert_eq!(target, AutomationTarget::TrackPan);
        assert_eq!(intervals[0].2, vec![(1.0, -0.5), (2.0, 0.75)]);
    }

    #[test]
    fn playing_mute_write_queues_a_discrete_point() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0))),
                ..Default::default()
            },
            |ui| {
                publish_transport(ui, true, 3.5);
                queue_mute_write(ui, 9, true);
            },
        );
        output.textures_delta.clear();
        assert_eq!(take_mute_writes(&ctx), vec![(9, 3.5, 1.0)]);
    }

    #[test]
    fn write_capture_discards_a_gesture_that_exceeds_its_budget() {
        let mut capture = WriteCapture::default();
        capture.start(7, AutomationTarget::TrackVolume, 0.0, 0.5);
        for index in 0..=MAX_WRITE_SAMPLES {
            capture.record(index as f64, 0.5);
        }
        assert!(capture.finish().is_err());
        assert!(!capture.is_active());
    }

    #[test]
    fn write_capture_ignores_single_sample_intervals_and_cancel() {
        let mut capture = WriteCapture::default();
        capture.start(7, AutomationTarget::TrackVolume, 4.0, 0.5);
        assert!(capture.finish().unwrap().2.is_empty());
        capture.start(7, AutomationTarget::TrackVolume, 4.0, 0.5);
        capture.record(5.0, 0.6);
        capture.cancel();
        assert!(!capture.is_active());
        assert!(capture.finish().unwrap().2.is_empty());
    }

    #[test]
    fn fader_follow_reads_the_curve_only_while_read_is_enabled() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track = &mut project.tracks[0];
        track.automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points: vec![
                AutomationPoint {
                    beat: 0.0,
                    value: 0.5,
                },
                AutomationPoint {
                    beat: 4.0,
                    value: 1.0,
                },
            ],
        });
        assert_eq!(follow_gain(track, 2.0), None);
        track.automation_read = true;
        assert_eq!(follow_gain(track, 2.0), Some(0.75));
        assert_eq!(follow_gain(track, 99.0), Some(1.0));
        track.automation_lanes[0].points[0].value = f32::NAN;
        assert_eq!(follow_gain(track, 2.0), None);
    }

    #[test]
    fn escape_cancels_an_active_point_drag() {
        let ctx = egui::Context::default();
        let track_id = 7;
        let rect = Rect::from_min_size(Pos2::new(20.0, 20.0), egui::vec2(300.0, 80.0));
        let points = [(4.0_f64, 0.5_f32)];
        let drag_id = Id::new(("volume-automation-lane", track_id)).with("dragging");
        let mut action = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 200.0))),
                events: vec![
                    egui::Event::PointerMoved(Pos2::new(60.0, 60.0)),
                    egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                ui.memory_mut(|memory| memory.data.insert_temp(drag_id, 0usize));
                action = draw_lane(ui, rect, track_id, &points, 20.0, 10.0, Some(1.0));
            },
        );
        output.textures_delta.clear();
        assert_eq!(action, Some(LaneAction::CancelDrag { track_id }));
        assert!(
            ctx.memory(|memory| memory.data.get_temp::<usize>(drag_id).is_none()),
            "the drag must stop after it is cancelled"
        );
    }

    #[test]
    fn rejected_lane_edit_does_not_mutate_project() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track_id = project.tracks[0].id;
        let before = project.clone();
        let result = apply_lane_action(
            &mut project,
            LaneAction::Set {
                track_id,
                target: AutomationTarget::TrackVolume,
                beat: f64::NAN,
                value: 0.5,
            },
        );
        assert!(result.is_err());
        assert_eq!(project, before);
    }

    #[test]
    fn move_and_remove_lane_actions_mutate_only_the_requested_points() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track_id = project.tracks[0].id;
        for (beat, value) in [(1.0, 0.2), (2.0, 0.5), (3.0, 0.9)] {
            apply_lane_action(
                &mut project,
                LaneAction::Set {
                    track_id,
                    target: AutomationTarget::TrackVolume,
                    beat,
                    value,
                },
            )
            .unwrap();
        }
        assert!(
            apply_lane_action(
                &mut project,
                LaneAction::Move {
                    track_id,
                    target: AutomationTarget::TrackVolume,
                    index: 1,
                    beat: 2.5,
                    value: 0.7,
                },
            )
            .unwrap()
        );
        assert_eq!(
            project.tracks[0].automation_lanes[0].points[1],
            AutomationPoint {
                beat: 2.5,
                value: 0.7
            }
        );
        assert!(
            apply_lane_action(
                &mut project,
                LaneAction::Remove {
                    track_id,
                    target: AutomationTarget::TrackVolume,
                    index: 1
                },
            )
            .unwrap()
        );
        assert_eq!(
            project.tracks[0].automation_lanes[0]
                .points
                .iter()
                .map(|point| point.beat)
                .collect::<Vec<_>>(),
            vec![1.0, 3.0]
        );
    }

    #[test]
    fn automation_point_edit_round_trips_through_project_undo_history() {
        let mut history = super::super::ProjectHistory::default();
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track_id = project.tracks[0].id;
        apply_lane_action(
            &mut project,
            LaneAction::Set {
                track_id,
                target: AutomationTarget::TrackVolume,
                beat: 1.0,
                value: 0.4,
            },
        )
        .unwrap();
        let before_edit = project.clone();
        assert!(history.begin(&project, false));
        apply_lane_action(
            &mut project,
            LaneAction::Move {
                track_id,
                target: AutomationTarget::TrackVolume,
                index: 0,
                beat: 2.0,
                value: 0.8,
            },
        )
        .unwrap();
        assert!(history.finish_frame(&project, true));
        let undone = history.undo(&project).expect("automation edit should undo");
        assert_eq!(undone, before_edit);
        let redone = history.redo(&undone).expect("automation edit should redo");
        assert_eq!(redone, project);
    }
}
