//! Routing decisions shared by Channel Settings and playback plan preparation.
use opentrack_audio::{PlaybackOutputRoute, PlaybackSend};
use opentrack_core::project::{Track, TrackKind};

/// Resolve persisted sends into the bounded immutable playback representation.
pub(super) fn playback_sends(track: &Track, tracks: &[Track]) -> Vec<PlaybackSend> {
    if track.sends_bypassed {
        return Vec::new();
    }
    track
        .send_routing
        .iter()
        .take(opentrack_core::project::MAX_SEND_SLOTS)
        .enumerate()
        .filter_map(|(slot, route)| {
            let settings = track.send_parameters.get(slot).copied().unwrap_or_default();
            if !settings.enabled || !settings.level_db.is_finite() || !settings.pan.is_finite() {
                return None;
            }
            let destination = match route.as_deref()? {
                "Stereo Out" => PlaybackOutputRoute::Stereo,
                "Left" => PlaybackOutputRoute::Left,
                "Right" => PlaybackOutputRoute::Right,
                value if value.starts_with("Group:") => {
                    let id = value.strip_prefix("Group:")?.parse::<u64>().ok()?;
                    let index = tracks.iter().position(|candidate| {
                        candidate.id == id
                            && candidate.id != track.id
                            && candidate.kind == TrackKind::Group
                    })?;
                    PlaybackOutputRoute::Group(index)
                }
                _ => return None,
            };
            let db = settings.level_db.clamp(-90.0, 12.0);
            let gain = if db <= -90.0 {
                0.0
            } else {
                10.0_f32.powf(db / 20.0)
            };
            Some(PlaybackSend {
                destination,
                tap: settings.tap,
                gain,
                pan: settings.pan.clamp(-1.0, 1.0),
            })
        })
        .collect()
}

pub(super) fn group_route_would_cycle(
    tracks: &[Track],
    track_id: u64,
    target_group_id: u64,
) -> bool {
    let mut pending = vec![target_group_id];
    let mut visited = std::collections::HashSet::new();
    while let Some(current) = pending.pop() {
        if current == track_id {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        let Some(track) = tracks.iter().find(|track| track.id == current) else {
            continue;
        };
        let destinations = track
            .output_routing
            .iter()
            .map(String::as_str)
            .chain(track.send_routing.iter().flatten().map(String::as_str));
        for destination in destinations {
            if let Some(id) = destination
                .strip_prefix("Group:")
                .and_then(|id| id.parse::<u64>().ok())
            {
                pending.push(id);
            }
        }
    }
    false
}

pub(super) fn has_active_sends(track: &Track) -> bool {
    !track.sends_bypassed
        && track
            .send_routing
            .iter()
            .enumerate()
            .any(|(slot, destination)| {
                destination.is_some()
                    && track
                        .send_parameters
                        .get(slot)
                        .is_some_and(|settings| settings.enabled)
            })
}
