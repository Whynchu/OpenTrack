//! Original short pop click, evaluated against the transport sample clock.
#[derive(Clone, Copy, Debug)]
pub struct MetronomeSettings {
    pub enabled: bool,
    pub level: f32,
    pub beats_per_bar: u8,
    pub beat_unit: u8,
    /// MIDI note for the downbeat; other beats sound one octave lower.
    pub downbeat_note: u8,
}
impl Default for MetronomeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            level: 0.35,
            beats_per_bar: 4,
            beat_unit: 4,
            downbeat_note: 87,
        }
    }
}

impl MetronomeSettings {
    pub(crate) fn downbeat_frequency(self) -> f64 {
        midi_frequency(self.downbeat_note.clamp(24, 108))
    }

    #[cfg(test)]
    fn sample(self, frame: u64, rate: u32, tempo: f64, playing: bool) -> f32 {
        self.sample_with_frequency(frame, rate, tempo, playing, self.downbeat_frequency())
    }

    pub(crate) fn sample_with_frequency(
        self,
        frame: u64,
        rate: u32,
        tempo: f64,
        playing: bool,
        downbeat_frequency: f64,
    ) -> f32 {
        if !playing || !self.enabled || rate == 0 || !tempo.is_finite() || tempo <= 0.0 {
            return 0.0;
        }
        let beat_seconds = 60.0 / tempo * 4.0 / f64::from(self.beat_unit.max(1));
        let position = frame as f64 / f64::from(rate) / beat_seconds;
        let beat = position.floor();
        let age = (position - beat) * beat_seconds;
        if age >= 0.024 {
            return 0.0;
        }
        let accent = beat as u64 % u64::from(self.beats_per_bar.max(1)) == 0;
        let frequency = downbeat_frequency / if accent { 1.0 } else { 2.0 };
        let phase = std::f64::consts::TAU * frequency * age;
        let n = (age * f64::from(rate)).round() as u64;
        let hash = n
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add(0xD1B54A32D192ED03);
        let noise = ((hash ^ (hash >> 31)) >> 40) as f64 / 8_388_608.0 - 1.0;
        let attack = (age / 0.0004).min(1.0);
        let tail = ((0.024 - age) / 0.004).min(1.0);
        let body = (phase.sin() * 0.72 + (phase * 2.7).sin() * 0.13) * (-age / 0.005).exp();
        let transient = noise * 0.15 * (-age / 0.0012).exp();
        let level = if self.level.is_finite() {
            self.level.clamp(0.0, 1.0)
        } else {
            0.0
        };
        ((body + transient) * attack * tail) as f32 * level * if accent { 1.0 } else { 0.8 }
    }
}

fn midi_frequency(note: u8) -> f64 {
    440.0 * 2.0_f64.powf((f64::from(note) - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_click_is_short_accented_and_silent_when_stopped_or_disabled() {
        let click = MetronomeSettings {
            enabled: true,
            ..Default::default()
        };
        for rate in [44_100, 48_000, 96_000] {
            assert_eq!(click.sample(0, rate, 120.0, true), 0.0);
            assert!(click.sample(rate as u64 / 1000, rate, 120.0, true).abs() > 0.001);
            assert_eq!(click.sample(rate as u64 / 10, rate, 120.0, true), 0.0);
            assert_eq!(click.sample(100, rate, 120.0, false), 0.0);
            assert_eq!(
                MetronomeSettings::default().sample(100, rate, 120.0, true),
                0.0
            );
            let bar = u64::from(rate) * 2;
            for offset in 0..1000 {
                assert!(
                    (click.sample(offset, rate, 120.0, true)
                        - click.sample(bar + offset, rate, 120.0, true))
                    .abs()
                        < 0.0001
                );
            }
            let downbeat: Vec<_> = (0..rate / 40)
                .map(|n| click.sample(n as u64, rate, 120.0, true))
                .collect();
            let beat: Vec<_> = (0..rate / 40)
                .map(|n| click.sample((rate / 2 + n) as u64, rate, 120.0, true))
                .collect();
            assert_ne!(downbeat, beat);
            assert!(downbeat.iter().all(|v| v.is_finite() && v.abs() <= 0.35));
        }
    }

    #[test]
    fn fractional_tempo_and_signature_follow_project_time_without_block_drift() {
        let click = MetronomeSettings {
            enabled: true,
            beats_per_bar: 6,
            beat_unit: 8,
            ..Default::default()
        };
        let rate = 48_000;
        let tempo = 123.45;
        let beat_frames = f64::from(rate) * 60.0 / tempo / 2.0;
        for beat in [1, 5, 6, 101, 10001] {
            let boundary = (beat_frames * beat as f64).ceil() as u64;
            assert_eq!(click.sample(boundary - 1, rate, tempo, true), 0.0);
            assert!((1..40).any(|n| click.sample(boundary + n, rate, tempo, true).abs() > 0.001));
        }
    }

    #[test]
    fn selected_downbeat_pitch_sets_other_beats_one_octave_lower() {
        let click = MetronomeSettings {
            enabled: true,
            downbeat_note: 72,
            ..Default::default()
        };
        assert!(click.sample(200, 48_000, 120.0, true).abs() > 0.0);
        let default = MetronomeSettings {
            enabled: true,
            ..Default::default()
        };
        assert_ne!(
            click.sample(200, 48_000, 120.0, true),
            default.sample(200, 48_000, 120.0, true)
        );
        assert!((midi_frequency(72) / midi_frequency(60) - 2.0).abs() < 1e-12);
    }
}
