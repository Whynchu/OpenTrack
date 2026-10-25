//! Shared channel fader taper: useful dB travel plus an explicit bottom mute zone.
pub(super) const MIN_DB: f32 = -30.0;
pub(super) const MAX_DB: f32 = 6.0206;
pub(super) const MUTE_DB: f32 = -31.0;
const AUDIBLE_END: f32 = 0.98;
pub(super) fn gain_to_position(gain: f32) -> f32 {
    if gain <= 0.0 {
        1.0
    } else {
        ((MAX_DB - 20.0 * gain.log10()) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0) * AUDIBLE_END
    }
}
pub(super) fn position_to_gain(position: f32) -> f32 {
    position_to_gain_with_precision(position, false)
}
pub(super) fn position_to_gain_fine(position: f32) -> f32 {
    position_to_gain_with_precision(position, true)
}
fn position_to_gain_with_precision(position: f32, fine: bool) -> f32 {
    if position >= 0.995 {
        return 0.0;
    }
    let mut db = MAX_DB - (position / AUDIBLE_END).clamp(0.0, 1.0) * (MAX_DB - MIN_DB);
    if !fine && db.abs() < 0.12 {
        db = 0.0;
    }
    10.0_f32
        .powf(db / 20.0)
        .min(opentrack_core::project::MAX_CHANNEL_GAIN)
}
pub(super) fn inspector_db(gain: f32) -> f32 {
    if gain <= 0.0 {
        MUTE_DB
    } else {
        20.0 * gain.log10()
    }
}
pub(super) fn from_inspector_db(db: f32) -> f32 {
    if db <= MUTE_DB + 0.1 {
        0.0
    } else {
        10.0_f32
            .powf(db.max(MIN_DB) / 20.0)
            .min(opentrack_core::project::MAX_CHANNEL_GAIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fader_has_finite_minus_thirty_floor_positive_headroom_and_distinct_mute_endpoint() {
        assert!((20.0 * position_to_gain(0.98).log10() + 30.0).abs() < 0.001);
        assert_eq!(position_to_gain(1.0), 0.0);
        assert!((position_to_gain(0.0) - 2.0).abs() < 0.0001);
        for db in [-30.0_f32, -24.0, -12.0, -6.0, 0.0, 3.0, 6.0] {
            let gain = 10.0_f32.powf(db / 20.0);
            assert!((position_to_gain(gain_to_position(gain)) - gain).abs() < 0.0001);
        }
        assert_eq!(from_inspector_db(MUTE_DB), 0.0);
        assert_eq!(inspector_db(0.0), MUTE_DB);
        assert!(from_inspector_db(3.0) > 1.0);
        let near = 10.0_f32.powf(0.03 / 20.0);
        assert_eq!(position_to_gain(gain_to_position(near)), 1.0);
        assert!(
            (position_to_gain_fine(gain_to_position(near)) - near).abs() < 0.00001,
            "Shift must bypass the unity detent for fine adjustment"
        );
        // Legacy quieter values remain data, not silently converted to mute.
        assert!((inspector_db(0.001) + 60.0).abs() < 0.001);
    }
}
