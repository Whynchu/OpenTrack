//! Session meter ballistics, in dB per second; independent of display frame rate.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub peak_hold_ms: u32,
    pub slow_fallback: f32,
    pub fast_fallback: f32,
    pub spectrum_release_ms: u32,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            peak_hold_ms: 3000,
            slow_fallback: 20.0,
            fast_fallback: 40.0,
            spectrum_release_ms: 180,
        }
    }
}
impl Preferences {
    pub fn sanitize(&mut self) {
        self.peak_hold_ms = self.peak_hold_ms.min(10000);
        self.spectrum_release_ms = self.spectrum_release_ms.min(2000);
        for value in [&mut self.slow_fallback, &mut self.fast_fallback] {
            *value = if value.is_finite() {
                value.clamp(1.0, 120.0)
            } else {
                20.0
            };
        }
    }
}
#[derive(Clone, Debug, Default)]
pub(super) struct Hold {
    pub maximum: f32,
    pub markers: [f32; 2],
    remaining: [f32; 2],
}
fn fall(value: f32, db_per_second: f32, dt: f32) -> f32 {
    let value = value * 10.0_f32.powf(-db_per_second * dt / 20.0);
    if value < 0.001 { 0.0 } else { value }
}
impl Hold {
    pub fn update(
        &mut self,
        levels: &mut [f32; 2],
        raw: [f32; 2],
        dt: f32,
        settings: &Preferences,
    ) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        for i in 0..2 {
            let peak = if raw[i].is_finite() {
                raw[i].max(0.0)
            } else {
                0.0
            };
            self.maximum = self.maximum.max(peak);
            levels[i] = peak.max(fall(levels[i], settings.slow_fallback, dt));
            let falling_time = (dt - self.remaining[i]).max(0.0);
            self.remaining[i] = (self.remaining[i] - dt).max(0.0);
            self.markers[i] = fall(self.markers[i], settings.fast_fallback, falling_time);
            if peak > 0.0 && peak >= self.markers[i] {
                self.markers[i] = peak;
                self.remaining[i] = settings.peak_hold_ms as f32 / 1000.0;
            }
            self.markers[i] = self.markers[i].max(levels[i]);
        }
    }
}
/// Exponential release in dB, immediate attack. Zero gives the unsmoothed analyzer.
pub(super) fn smooth_spectrum(previous: &[f32], next: &mut [f32], dt: f32, release_ms: u32) {
    if release_ms == 0 {
        return;
    }
    let retain = (-dt.max(0.0) / (release_ms as f32 / 1000.0)).exp();
    for (old, new) in previous.iter().zip(next) {
        if *new < *old {
            *new = *new + (*old - *new) * retain;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timed_peak_and_bar_decay_match_db_rates_and_frame_partitions() {
        fn run(steps: usize) -> ([f32; 2], Hold) {
            let p = Preferences::default();
            let mut h = Hold::default();
            let mut levels = [0.0; 2];
            h.update(&mut levels, [1.0, 0.5], 0.0, &p);
            for _ in 0..steps {
                h.update(&mut levels, [0.0; 2], 1.0 / steps as f32, &p);
            }
            (levels, h)
        }
        let (one, mut h) = run(1);
        let (many, other) = run(100);
        assert!((20.0 * one[0].log10() + 20.0).abs() < 0.001);
        assert!((one[0] - many[0]).abs() < 0.0001);
        assert_eq!(h.markers, [1.0, 0.5]);
        assert_eq!(h.markers, other.markers);
        let mut levels = one;
        h.update(&mut levels, [0.0; 2], 2.5, &Preferences::default());
        assert!((20.0 * h.markers[0].log10() + 20.0).abs() < 0.001);
        assert_eq!(h.maximum, 1.0);
    }
    #[test]
    fn spectral_release_is_time_based_with_instant_attack_and_raw_option() {
        let mut next = vec![-60.0, 0.0];
        smooth_spectrum(&[0.0, -20.0], &mut next, 0.18, 180);
        assert!((next[0] - (-60.0 + 60.0 / std::f32::consts::E)).abs() < 0.001);
        assert_eq!(next[1], 0.0);
        let mut raw = vec![-60.0];
        smooth_spectrum(&[0.0], &mut raw, 0.1, 0);
        assert_eq!(raw[0], -60.0);
    }
}
