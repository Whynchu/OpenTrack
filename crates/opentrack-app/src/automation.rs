//! Application preparation of immutable playback automation.
use super::egui::{self, Color32, Id, Pos2, Rect, Sense, Stroke};
use opentrack_core::automation::{move_volume_point, remove_volume_point, set_volume_point};
use opentrack_core::project::{AutomationLane, AutomationTarget, Track};
use std::sync::Arc;

pub(super) fn playback_volume_automation(track: &Track) -> Option<Arc<AutomationLane>> {
    if !track.automation_read {
        return None;
    }
    track
        .automation_lanes
        .iter()
        .find(|lane| {
            lane.target == AutomationTarget::TrackVolume
                && !lane.points.is_empty()
                && lane.is_valid()
        })
        .map(|lane| Arc::new(lane.clone()))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum LaneAction {
    Set {
        track_id: u64,
        beat: f64,
        value: f32,
    },
    Move {
        track_id: u64,
        index: usize,
        beat: f64,
        value: f32,
    },
    Remove {
        track_id: u64,
        index: usize,
    },
}

pub(super) fn apply_lane_action(
    project: &mut opentrack_core::project::Project,
    action: LaneAction,
) -> Result<bool, String> {
    use opentrack_core::project::AutomationPoint;
    let result = match action {
        LaneAction::Set {
            track_id,
            beat,
            value,
        } => set_volume_point(project, track_id, AutomationPoint { beat, value }),
        LaneAction::Move {
            track_id,
            index,
            beat,
            value,
        } => move_volume_point(project, track_id, index, AutomationPoint { beat, value }),
        LaneAction::Remove { track_id, index } => remove_volume_point(project, track_id, index),
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
    let id = Id::new(("volume-automation-lane", track_id));
    let response = ui.interact(rect.expand(8.0), id, Sense::click_and_drag());
    let position = |pos: Pos2| {
        let beat = f64::from((pos.x - grid_left) / pixels_per_beat.max(1.0)).max(0.0);
        let beat = snap.map_or(beat, |step| crate::timeline::snap_beats_to_grid(beat, step));
        let position = ((pos.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        (beat, super::channel_gain::position_to_gain(position))
    };
    let painter = ui.painter().with_clip_rect(rect);
    for index in 1..points.len() {
        let (beat_a, value_a) = points[index - 1];
        let (beat_b, value_b) = points[index];
        let point_pos = |beat: f64, value: f32| {
            Pos2::new(
                grid_left + beat as f32 * pixels_per_beat,
                rect.top() + super::channel_gain::gain_to_position(value) * rect.height(),
            )
        };
        painter.line_segment(
            [point_pos(beat_a, value_a), point_pos(beat_b, value_b)],
            Stroke::new(2.0, Color32::from_rgb(100, 200, 140)),
        );
    }
    for (beat, value) in points {
        painter.circle_filled(
            Pos2::new(
                grid_left + *beat as f32 * pixels_per_beat,
                rect.top() + super::channel_gain::gain_to_position(*value) * rect.height(),
            ),
            4.0,
            Color32::from_rgb(120, 225, 160),
        );
    }
    let selected = ui.memory(|memory| memory.data.get_temp::<usize>(id.with("selected")));
    if let Some((beat, value)) = selected.and_then(|index| points.get(index)) {
        painter.circle_stroke(
            Pos2::new(
                grid_left + *beat as f32 * pixels_per_beat,
                rect.top() + super::channel_gain::gain_to_position(*value) * rect.height(),
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
                rect.top() + super::channel_gain::gain_to_position(*value) * rect.height(),
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
                rect.top() + super::channel_gain::gain_to_position(*value) * rect.height(),
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
            return Some(LaneAction::Remove { track_id, index });
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
    fn rejected_lane_edit_does_not_mutate_project() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let track_id = project.tracks[0].id;
        let before = project.clone();
        let result = apply_lane_action(
            &mut project,
            LaneAction::Set {
                track_id,
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
            apply_lane_action(&mut project, LaneAction::Remove { track_id, index: 1 },).unwrap()
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
}
