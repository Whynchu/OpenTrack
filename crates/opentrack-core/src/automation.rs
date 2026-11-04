//! Track automation data and validated edit commands.
use crate::project::{MAX_CHANNEL_GAIN, Project};
use serde::{Deserialize, Serialize};

/// Bounded project automation data; audio snapshots may copy these points.
pub const MAX_AUTOMATION_LANES_PER_TRACK: usize = 16;
pub const MAX_AUTOMATION_POINTS_PER_LANE: usize = 100_000;
pub const MAX_AUTOMATION_POINTS_PER_PROJECT: usize = 1_000_000;
pub const MAX_AUTOMATION_BEAT: f64 = 1_000_000.0;

/// Unknown targets are rejected during decoding rather than treated as volume.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationTarget {
    TrackVolume,
    TrackPan,
    TrackMute,
    /// Stereo In channel fader and pan controls.
    InputBusVolume,
    InputBusPan,
    InputBusMute,
    /// Stereo Out channel fader and pan controls.
    OutputBusVolume,
    OutputBusPan,
    OutputBusMute,
    /// Linear send gain (0..=3.981072, corresponding to -90..+12 dB).
    SendLevel(u8),
    SendPan(u8),
    /// VST3 normalized parameter value (0..=1). Owner identity is the track
    /// or device-bus id; `insert_id` and `parameter_id` are stable identifiers.
    PluginParameter {
        insert_id: u64,
        parameter_id: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationPoint {
    /// Position in quarter-note beats on the project timeline.
    pub beat: f64,
    /// Target-specific value: gain (0..=2), pan (-1..=1), or mute (0/1).
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationLane {
    pub target: AutomationTarget,
    pub points: Vec<AutomationPoint>,
}

impl AutomationLane {
    /// Validate a lane once on the control thread before publishing it.
    pub fn is_valid(&self) -> bool {
        self.points.len() <= MAX_AUTOMATION_POINTS_PER_LANE
            && self
                .points
                .iter()
                .all(|point| valid_target_point(self.target, *point))
            && self
                .points
                .windows(2)
                .all(|pair| pair[0].beat < pair[1].beat)
    }

    /// Hold the end points; continuous controls interpolate linearly and mute
    /// changes stepwise at the point beat.
    /// Points must be ordered and validated before publication to playback.
    pub fn value_at(&self, beat: f64) -> Option<f32> {
        self.value_at_cached(beat, &mut 0)
    }

    /// Reuse the next-point index during sequential playback. Seeks, loops and
    /// arbitrary backwards jumps fall back to a bounded binary search.
    pub fn value_at_cached(&self, beat: f64, cursor: &mut usize) -> Option<f32> {
        if !beat.is_finite() || self.points.is_empty() {
            return None;
        }
        let count = self.points.len();
        let mut next = (*cursor).min(count);
        let after_left = next == 0 || self.points[next - 1].beat <= beat;
        let before_right = next == count || beat < self.points[next].beat;
        if !(after_left && before_right) {
            if next < count
                && self.points[next].beat <= beat
                && (next + 1 == count || beat < self.points[next + 1].beat)
            {
                next += 1;
            } else {
                next = self.points.partition_point(|point| point.beat <= beat);
            }
        }
        *cursor = next;
        let valid_value = |value: f32| {
            let point = AutomationPoint { beat: 0.0, value };
            valid_target_point(self.target, point).then_some(value)
        };
        if next == 0 {
            return valid_value(self.points[0].value);
        }
        if next == count {
            return valid_value(self.points[next - 1].value);
        }
        let left = self.points[next - 1];
        let right = self.points[next];
        if matches!(
            self.target,
            AutomationTarget::TrackMute
                | AutomationTarget::InputBusMute
                | AutomationTarget::OutputBusMute
        ) {
            return valid_value(left.value);
        }
        let width = right.beat - left.beat;
        if !width.is_finite() || width <= 0.0 {
            return None;
        }
        let left_value = valid_value(left.value)?;
        let right_value = valid_value(right.value)?;
        let fraction = (beat - left.beat) / width;
        Some(
            (f64::from(left_value) + (f64::from(right_value) - f64::from(left_value)) * fraction)
                as f32,
        )
    }
}

/// A rejected edit never mutates project data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutomationEditError {
    TrackUnavailable,
    LaneUnavailable,
    PointUnavailable,
    InvalidPoint,
    InvalidRange,
    WouldCrossNeighbor,
    LaneLimit,
    PointLimit,
    ProjectLimit,
}

impl std::fmt::Display for AutomationEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TrackUnavailable => "The track is unavailable",
            Self::LaneUnavailable => "The automation lane is unavailable",
            Self::PointUnavailable => "The automation point is unavailable",
            Self::InvalidPoint => "Automation position or gain is outside the supported range",
            Self::InvalidRange => "The automation range is empty or outside the timeline",
            Self::WouldCrossNeighbor => "Automation points cannot cross or overlap",
            Self::LaneLimit => "The track has too many automation lanes",
            Self::PointLimit => "The automation lane has too many points",
            Self::ProjectLimit => "The project has too many automation points",
        })
    }
}

impl std::error::Error for AutomationEditError {}

fn valid_target_point(target: AutomationTarget, point: AutomationPoint) -> bool {
    if !point.beat.is_finite() || !(0.0..=MAX_AUTOMATION_BEAT).contains(&point.beat) {
        return false;
    }
    match target {
        AutomationTarget::TrackVolume
        | AutomationTarget::InputBusVolume
        | AutomationTarget::OutputBusVolume => {
            point.value.is_finite() && (0.0..=MAX_CHANNEL_GAIN).contains(&point.value)
        }
        AutomationTarget::TrackPan
        | AutomationTarget::InputBusPan
        | AutomationTarget::OutputBusPan
        | AutomationTarget::SendPan(_) => {
            point.value.is_finite() && (-1.0..=1.0).contains(&point.value)
        }
        AutomationTarget::TrackMute
        | AutomationTarget::InputBusMute
        | AutomationTarget::OutputBusMute => point.value == 0.0 || point.value == 1.0,
        AutomationTarget::SendLevel(_) => {
            point.value.is_finite() && (0.0..=3.981_072).contains(&point.value)
        }
        AutomationTarget::PluginParameter { .. } => {
            point.value.is_finite() && (0.0..=1.0).contains(&point.value)
        }
    }
}

/// Insert a point in beat order, or replace the value at exactly that beat.
pub fn set_volume_point(
    project: &mut Project,
    track_id: u64,
    point: AutomationPoint,
) -> Result<bool, AutomationEditError> {
    set_target_point(project, track_id, AutomationTarget::TrackVolume, point)
}

pub fn set_target_point(
    project: &mut Project,
    track_id: u64,
    target: AutomationTarget,
    point: AutomationPoint,
) -> Result<bool, AutomationEditError> {
    if !valid_target_point(target, point) {
        return Err(AutomationEditError::InvalidPoint);
    }
    let track = project
        .channels()
        .find(|track| track.id == track_id)
        .ok_or(AutomationEditError::TrackUnavailable)?;
    let lane_index = track
        .automation_lanes
        .iter()
        .position(|lane| lane.target == target);
    let lane = lane_index.map(|index| &track.automation_lanes[index]);
    let position = lane.map_or(0, |lane| {
        lane.points
            .partition_point(|candidate| candidate.beat < point.beat)
    });
    let replacing = lane.is_some_and(|lane| {
        lane.points
            .get(position)
            .is_some_and(|candidate| candidate.beat == point.beat)
    });
    if replacing {
        let lane = &mut project
            .channel_by_id_mut(track_id)
            .ok_or(AutomationEditError::TrackUnavailable)?
            .automation_lanes[lane_index.expect("existing point has lane")];
        if lane.points[position] == point {
            return Ok(false);
        }
        lane.points[position] = point;
        return Ok(true);
    }
    if lane.is_some_and(|lane| lane.points.len() >= MAX_AUTOMATION_POINTS_PER_LANE) {
        return Err(AutomationEditError::PointLimit);
    }
    if lane_index.is_none() && track.automation_lanes.len() >= MAX_AUTOMATION_LANES_PER_TRACK {
        return Err(AutomationEditError::LaneLimit);
    }
    let total = project
        .channels()
        .flat_map(|track| &track.automation_lanes)
        .fold(0usize, |total, lane| {
            total.saturating_add(lane.points.len())
        });
    if total >= MAX_AUTOMATION_POINTS_PER_PROJECT {
        return Err(AutomationEditError::ProjectLimit);
    }
    let track = project
        .channel_by_id_mut(track_id)
        .ok_or(AutomationEditError::TrackUnavailable)?;
    let index = lane_index.unwrap_or_else(|| {
        track.automation_lanes.push(AutomationLane {
            target,
            points: Vec::new(),
        });
        track.automation_lanes.len() - 1
    });
    track.automation_lanes[index].points.insert(position, point);
    Ok(true)
}

/// Replace the curve inside `[start, end]` with `samples`, keeping points outside.
///
/// A write gesture produces the captured samples. The interval edges always
/// become points, so the written section joins the untouched curve on both
/// sides instead of ending in an implied value.
pub fn replace_volume_points_in_range(
    project: &mut Project,
    track_id: u64,
    start: f64,
    end: f64,
    samples: &[AutomationPoint],
) -> Result<bool, AutomationEditError> {
    replace_target_points_in_range(
        project,
        track_id,
        AutomationTarget::TrackVolume,
        start,
        end,
        samples,
    )
}

pub fn replace_target_points_in_range(
    project: &mut Project,
    track_id: u64,
    target: AutomationTarget,
    start: f64,
    end: f64,
    samples: &[AutomationPoint],
) -> Result<bool, AutomationEditError> {
    if !start.is_finite()
        || !end.is_finite()
        || start < 0.0
        || end <= start
        || end > MAX_AUTOMATION_BEAT
    {
        return Err(AutomationEditError::InvalidRange);
    }
    if samples.is_empty() {
        return Err(AutomationEditError::InvalidPoint);
    }
    // Captured samples arrive in timeline order; an out-of-order capture means
    // the caller mixed intervals and must not be written as one range.
    let mut captured = samples.to_vec();
    let mut previous: Option<f64> = None;
    for point in &captured {
        if !valid_target_point(target, *point)
            || point.beat < start - 1e-9
            || point.beat > end + 1e-9
        {
            return Err(AutomationEditError::InvalidPoint);
        }
        if previous.is_some_and(|beat| point.beat <= beat + 1e-9) {
            return Err(AutomationEditError::WouldCrossNeighbor);
        }
        previous = Some(point.beat);
    }
    if captured[0].beat > start + 1e-9 {
        captured.insert(
            0,
            AutomationPoint {
                beat: start,
                value: captured[0].value,
            },
        );
    }
    if let Some(last) = captured.last()
        && last.beat < end - 1e-9
    {
        captured.push(AutomationPoint {
            beat: end,
            value: last.value,
        });
    }

    let track = project
        .channels()
        .find(|track| track.id == track_id)
        .ok_or(AutomationEditError::TrackUnavailable)?;
    let lane_index = track
        .automation_lanes
        .iter()
        .position(|lane| lane.target == target);
    let existing: &[AutomationPoint] = lane_index
        .map(|index| track.automation_lanes[index].points.as_slice())
        .unwrap_or(&[]);

    let mut points = existing
        .iter()
        .copied()
        .filter(|point| point.beat < start - 1e-9 || point.beat > end + 1e-9)
        .collect::<Vec<_>>();
    points.extend(captured);
    points.sort_by(|left, right| left.beat.total_cmp(&right.beat));

    if points.len() > MAX_AUTOMATION_POINTS_PER_LANE {
        return Err(AutomationEditError::PointLimit);
    }
    if lane_index.is_none() && track.automation_lanes.len() >= MAX_AUTOMATION_LANES_PER_TRACK {
        return Err(AutomationEditError::LaneLimit);
    }
    let total = project
        .channels()
        .flat_map(|track| &track.automation_lanes)
        .fold(0usize, |total, lane| {
            total.saturating_add(lane.points.len())
        });
    if total - existing.len() + points.len() > MAX_AUTOMATION_POINTS_PER_PROJECT {
        return Err(AutomationEditError::ProjectLimit);
    }
    if points == existing {
        return Ok(false);
    }

    let track = project
        .channel_by_id_mut(track_id)
        .ok_or(AutomationEditError::TrackUnavailable)?;
    let index = lane_index.unwrap_or_else(|| {
        track.automation_lanes.push(AutomationLane {
            target,
            points: Vec::new(),
        });
        track.automation_lanes.len() - 1
    });
    track.automation_lanes[index].points = points;
    Ok(true)
}

/// Move a point without crossing its neighbors; its index remains stable.
pub fn move_volume_point(
    project: &mut Project,
    track_id: u64,
    index: usize,
    point: AutomationPoint,
) -> Result<bool, AutomationEditError> {
    move_target_point(
        project,
        track_id,
        AutomationTarget::TrackVolume,
        index,
        point,
    )
}

pub fn move_target_point(
    project: &mut Project,
    track_id: u64,
    target: AutomationTarget,
    index: usize,
    point: AutomationPoint,
) -> Result<bool, AutomationEditError> {
    if !valid_target_point(target, point) {
        return Err(AutomationEditError::InvalidPoint);
    }
    let track = project
        .channel_by_id_mut(track_id)
        .ok_or(AutomationEditError::TrackUnavailable)?;
    let lane = track
        .automation_lanes
        .iter_mut()
        .find(|lane| lane.target == target)
        .ok_or(AutomationEditError::LaneUnavailable)?;
    let previous = *lane
        .points
        .get(index)
        .ok_or(AutomationEditError::PointUnavailable)?;
    if index > 0 && point.beat <= lane.points[index - 1].beat
        || index + 1 < lane.points.len() && point.beat >= lane.points[index + 1].beat
    {
        return Err(AutomationEditError::WouldCrossNeighbor);
    }
    if previous == point {
        return Ok(false);
    }
    lane.points[index] = point;
    Ok(true)
}

/// Remove a point and remove its lane when it becomes empty.
pub fn remove_volume_point(
    project: &mut Project,
    track_id: u64,
    index: usize,
) -> Result<bool, AutomationEditError> {
    remove_target_point(project, track_id, AutomationTarget::TrackVolume, index)
}

pub fn remove_target_point(
    project: &mut Project,
    track_id: u64,
    target: AutomationTarget,
    index: usize,
) -> Result<bool, AutomationEditError> {
    let track = project
        .channel_by_id_mut(track_id)
        .ok_or(AutomationEditError::TrackUnavailable)?;
    let lane_index = track
        .automation_lanes
        .iter()
        .position(|lane| lane.target == target)
        .ok_or(AutomationEditError::LaneUnavailable)?;
    let lane = &mut track.automation_lanes[lane_index];
    if index >= lane.points.len() {
        return Err(AutomationEditError::PointUnavailable);
    }
    lane.points.remove(index);
    if lane.points.is_empty() {
        track.automation_lanes.remove(lane_index);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TrackKind;

    fn point(beat: f64, value: f32) -> AutomationPoint {
        AutomationPoint { beat, value }
    }

    #[test]
    fn write_range_replaces_interior_and_keeps_outside_points() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project.tracks[0].id;
        set_volume_point(&mut project, id, point(1.0, 0.25)).unwrap();
        set_volume_point(&mut project, id, point(5.0, 0.75)).unwrap();
        set_volume_point(&mut project, id, point(9.0, 0.5)).unwrap();

        let changed = replace_volume_points_in_range(
            &mut project,
            id,
            3.0,
            7.0,
            &[point(3.0, 0.4), point(5.0, 0.9), point(7.0, 0.6)],
        )
        .unwrap();
        assert!(changed);

        let points = &project.tracks[0].automation_lanes[0].points;
        assert_eq!(
            points,
            &[
                point(1.0, 0.25),
                point(3.0, 0.4),
                point(5.0, 0.9),
                point(7.0, 0.6),
                point(9.0, 0.5),
            ]
        );
    }

    #[test]
    fn write_range_adds_edge_points_when_capture_starts_late() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project.tracks[0].id;
        let changed =
            replace_volume_points_in_range(&mut project, id, 2.0, 6.0, &[point(3.0, 0.5)]).unwrap();
        assert!(changed);
        assert_eq!(
            project.tracks[0].automation_lanes[0].points,
            vec![point(2.0, 0.5), point(3.0, 0.5), point(6.0, 0.5)]
        );
    }

    #[test]
    fn write_range_rejects_empty_unsorted_and_out_of_range_captures() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project.tracks[0].id;
        assert_eq!(
            replace_volume_points_in_range(&mut project, id, 2.0, 6.0, &[]),
            Err(AutomationEditError::InvalidPoint)
        );
        assert_eq!(
            replace_volume_points_in_range(
                &mut project,
                id,
                2.0,
                6.0,
                &[point(4.0, 0.5), point(3.0, 0.5)]
            ),
            Err(AutomationEditError::WouldCrossNeighbor)
        );
        assert_eq!(
            replace_volume_points_in_range(&mut project, id, 2.0, 6.0, &[point(7.0, 0.5)]),
            Err(AutomationEditError::InvalidPoint)
        );
        assert_eq!(
            replace_volume_points_in_range(&mut project, id, 6.0, 2.0, &[point(4.0, 0.5)]),
            Err(AutomationEditError::InvalidRange)
        );
        assert!(project.tracks[0].automation_lanes.is_empty());
    }

    #[test]
    fn write_range_that_matches_the_curve_reports_no_change() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project.tracks[0].id;
        replace_volume_points_in_range(&mut project, id, 2.0, 4.0, &[point(3.0, 0.5)]).unwrap();
        assert_eq!(
            replace_volume_points_in_range(&mut project, id, 2.0, 4.0, &[point(3.0, 0.5)]),
            Ok(false)
        );
    }

    #[test]
    fn point_edits_keep_order_replace_exact_times_and_reject_crossing_atomically() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project.tracks[0].id;
        assert_eq!(
            set_volume_point(&mut project, id, point(2.0, 0.5)),
            Ok(true)
        );
        assert_eq!(
            set_volume_point(&mut project, id, point(1.0, 0.25)),
            Ok(true)
        );
        assert_eq!(
            set_volume_point(&mut project, id, point(3.0, 1.0)),
            Ok(true)
        );
        assert_eq!(
            set_volume_point(&mut project, id, point(2.0, 0.75)),
            Ok(true)
        );
        assert_eq!(
            set_volume_point(&mut project, id, point(2.0, 0.75)),
            Ok(false)
        );
        assert_eq!(
            project.tracks[0].automation_lanes[0].points,
            vec![point(1.0, 0.25), point(2.0, 0.75), point(3.0, 1.0)]
        );
        let before = project.clone();
        assert_eq!(
            move_volume_point(&mut project, id, 1, point(3.0, 0.7)),
            Err(AutomationEditError::WouldCrossNeighbor)
        );
        assert_eq!(project, before);
        assert_eq!(
            move_volume_point(&mut project, id, 1, point(2.5, 0.7)),
            Ok(true)
        );
        assert!(project.validate().is_ok());
        assert_eq!(remove_volume_point(&mut project, id, 2), Ok(true));
        assert_eq!(remove_volume_point(&mut project, id, 1), Ok(true));
        assert_eq!(remove_volume_point(&mut project, id, 0), Ok(true));
        assert!(project.tracks[0].automation_lanes.is_empty());
    }

    #[test]
    fn invalid_edits_and_point_budget_leave_projects_unchanged() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project.tracks[0].id;
        let before = project.clone();
        for point in [
            point(f64::NAN, 1.0),
            point(-0.1, 1.0),
            point(0.0, f32::INFINITY),
            point(0.0, 2.1),
        ] {
            assert_eq!(
                set_volume_point(&mut project, id, point),
                Err(AutomationEditError::InvalidPoint)
            );
            assert_eq!(project, before);
        }
        assert_eq!(
            set_volume_point(&mut project, id + 1, point(0.0, 1.0)),
            Err(AutomationEditError::TrackUnavailable)
        );
        let points: Vec<_> = (0..MAX_AUTOMATION_POINTS_PER_LANE)
            .map(|i| point(i as f64, 1.0))
            .collect();
        project.tracks[0].automation_lanes.push(AutomationLane {
            target: AutomationTarget::TrackVolume,
            points,
        });
        assert_eq!(
            set_volume_point(&mut project, id, point(100_000.0, 1.0)),
            Err(AutomationEditError::PointLimit)
        );
        assert_eq!(
            set_volume_point(&mut project, id, point(0.0, 0.5)),
            Ok(true)
        );
        assert_eq!(
            project.tracks[0].automation_lanes[0].points.len(),
            MAX_AUTOMATION_POINTS_PER_LANE
        );
        assert!(project.validate().is_ok());
    }

    #[test]
    fn project_budget_rejects_insertion_without_adding_an_empty_lane() {
        let mut project = Project::default();
        let points: Vec<_> = (0..MAX_AUTOMATION_POINTS_PER_LANE)
            .map(|i| point(i as f64, 1.0))
            .collect();
        for _ in 0..10 {
            project.add_track(TrackKind::Audio);
            project
                .tracks
                .last_mut()
                .unwrap()
                .automation_lanes
                .push(AutomationLane {
                    target: AutomationTarget::TrackVolume,
                    points: points.clone(),
                });
        }
        project.add_track(TrackKind::Audio);
        let id = project.tracks.last().unwrap().id;
        assert_eq!(
            set_volume_point(&mut project, id, point(1.0, 0.5)),
            Err(AutomationEditError::ProjectLimit)
        );
        assert!(project.tracks.last().unwrap().automation_lanes.is_empty());
        assert!(project.validate().is_ok());
    }

    #[test]
    fn pan_interpolates_in_stereo_balance_and_mute_changes_stepwise() {
        let pan = AutomationLane {
            target: AutomationTarget::TrackPan,
            points: vec![point(1.0, -1.0), point(3.0, 1.0)],
        };
        assert!(pan.is_valid());
        assert_eq!(pan.value_at(2.0), Some(0.0));
        assert!(
            !(AutomationLane {
                target: AutomationTarget::TrackPan,
                points: vec![point(1.0, 1.1)],
            })
            .is_valid()
        );

        let mute = AutomationLane {
            target: AutomationTarget::TrackMute,
            points: vec![point(1.0, 0.0), point(2.0, 1.0)],
        };
        assert!(mute.is_valid());
        assert_eq!(mute.value_at(1.999), Some(0.0));
        assert_eq!(mute.value_at(2.0), Some(1.0));
        assert!(
            !(AutomationLane {
                target: AutomationTarget::TrackMute,
                points: vec![point(1.0, 0.5)],
            })
            .is_valid()
        );
    }

    #[test]
    fn generic_target_edits_validate_pan_and_mute_values_and_keep_order() {
        let mut project = Project::default();
        project.add_track(TrackKind::Audio);
        let id = project.tracks[0].id;
        assert!(
            set_target_point(
                &mut project,
                id,
                AutomationTarget::TrackPan,
                point(1.0, -0.5)
            )
            .unwrap()
        );
        assert!(
            set_target_point(
                &mut project,
                id,
                AutomationTarget::TrackMute,
                point(2.0, 1.0)
            )
            .unwrap()
        );
        assert_eq!(
            move_target_point(
                &mut project,
                id,
                AutomationTarget::TrackPan,
                0,
                point(1.5, 0.75)
            ),
            Ok(true)
        );
        assert_eq!(
            set_target_point(
                &mut project,
                id,
                AutomationTarget::TrackMute,
                point(3.0, 0.5)
            ),
            Err(AutomationEditError::InvalidPoint)
        );
        assert!(remove_target_point(&mut project, id, AutomationTarget::TrackMute, 0).unwrap());
        assert_eq!(project.tracks[0].automation_lanes.len(), 1);
        assert_eq!(
            project.tracks[0].automation_lanes[0].target,
            AutomationTarget::TrackPan
        );
        assert_eq!(
            project.tracks[0].automation_lanes[0].points[0],
            point(1.5, 0.75)
        );
    }
}
