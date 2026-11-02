//! Routing decisions shared by Channel Settings and playback plan preparation.
use opentrack_audio::{PlaybackInputChannel, PlaybackOutputRoute, PlaybackSend};
use opentrack_core::project::{Track, TrackKind};
use opentrack_core::routing::{InputRoute, OutputRoute};

/// The channel's input route, with the historical default for its width.
pub(super) fn input_route(track: &Track) -> InputRoute {
    match track.input_routing.as_deref().map(InputRoute::parse) {
        Some(Ok(route)) => route,
        _ => {
            if track.audio_channels == 1 {
                InputRoute::MonoIn { channel: 1 }
            } else {
                InputRoute::StereoIn
            }
        }
    }
}

/// The channel's output route, defaulting to the stereo master like the engine.
pub(super) fn output_route(track: &Track) -> OutputRoute {
    match track.output_routing.as_deref().map(OutputRoute::parse) {
        Some(Ok(route)) => route,
        _ => OutputRoute::StereoOut,
    }
}

/// Resolves a persisted output route into the playback representation.
///
/// An unparseable route becomes [`PlaybackOutputRoute::Disconnected`] rather than
/// the master: a channel that stores something this build cannot route must not
/// silently join the stereo mix, and [`opentrack_core::project::Project::validate_routing`]
/// reports the same value at load time.
pub(super) fn playback_output_route(route: Option<&str>, tracks: &[Track]) -> PlaybackOutputRoute {
    match route.map(OutputRoute::parse) {
        None => PlaybackOutputRoute::Stereo,
        Some(Ok(OutputRoute::StereoOut)) => PlaybackOutputRoute::Stereo,
        Some(Ok(OutputRoute::Left)) => PlaybackOutputRoute::Left,
        Some(Ok(OutputRoute::Right)) => PlaybackOutputRoute::Right,
        Some(Ok(OutputRoute::NoBus)) => PlaybackOutputRoute::Disconnected,
        Some(Ok(OutputRoute::Group(group_id))) => tracks
            .iter()
            .position(|candidate| candidate.id == group_id && candidate.kind == TrackKind::Group)
            .map_or(
                PlaybackOutputRoute::Disconnected,
                PlaybackOutputRoute::Group,
            ),
        // FX channels are returns for sends; a main output here is disconnected
        // rather than silently falling back to the stereo master.
        Some(Ok(OutputRoute::Fx(_))) => PlaybackOutputRoute::Disconnected,
        Some(Err(_)) => PlaybackOutputRoute::Disconnected,
    }
}

/// Resolves a persisted input route into the live-monitor channel.
pub(super) fn playback_input_channel(route: Option<&str>) -> Option<PlaybackInputChannel> {
    match route.map(InputRoute::parse) {
        None => Some(PlaybackInputChannel::Stereo),
        Some(Ok(InputRoute::StereoIn)) => Some(PlaybackInputChannel::Stereo),
        Some(Ok(InputRoute::MonoIn { channel: 1 })) => Some(PlaybackInputChannel::Left),
        Some(Ok(InputRoute::MonoIn { channel: 2 })) => Some(PlaybackInputChannel::Right),
        // A mono route on a wider model has no monitor tap on a stereo bus, and
        // No Bus has none by definition.
        Some(Ok(InputRoute::MonoIn { .. })) | Some(Ok(InputRoute::NoBus)) => None,
        Some(Err(_)) => None,
    }
}

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
            let destination = match OutputRoute::parse(route.as_deref()?).ok()? {
                OutputRoute::StereoOut => PlaybackOutputRoute::Stereo,
                OutputRoute::Left => PlaybackOutputRoute::Left,
                OutputRoute::Right => PlaybackOutputRoute::Right,
                OutputRoute::NoBus => return None,
                OutputRoute::Group(id) => {
                    let index = tracks.iter().position(|candidate| {
                        candidate.id == id
                            && candidate.id != track.id
                            && candidate.kind == TrackKind::Group
                    })?;
                    PlaybackOutputRoute::Group(index)
                }
                OutputRoute::Fx(id) => {
                    let index = tracks.iter().position(|candidate| {
                        candidate.id == id
                            && candidate.id != track.id
                            && candidate.kind == TrackKind::Fx
                    })?;
                    PlaybackOutputRoute::Group(index)
                }
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
            if let Some(id) = OutputRoute::parse(destination)
                .ok()
                .and_then(OutputRoute::bus)
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
