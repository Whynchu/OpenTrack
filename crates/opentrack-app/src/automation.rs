//! Application preparation of immutable playback automation.
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
}
