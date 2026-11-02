// SPDX-License-Identifier: Apache-2.0
//! First-party EQ parameter model and real-time-safe stereo DSP.
//!
//! This crate has no GUI or VST3 ABI dependency. The plug-in adapter owns the
//! host boundary and passes validated parameter snapshots into [`Equalizer`].

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub const BAND_COUNT: usize = 8;
pub const MIN_FREQUENCY_HZ: f64 = 20.0;
pub const MAX_FREQUENCY_HZ: f64 = 20_000.0;
pub const MIN_Q: f64 = 0.1;
pub const MAX_Q: f64 = 18.0;
pub const MAX_GAIN_DB: f64 = 24.0;
pub const PARAM_BYPASS: u32 = 0;
pub const PARAMETER_COUNT: usize = 1 + BAND_COUNT * 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BandParameter {
    Enabled,
    Shape,
    Frequency,
    Gain,
    Q,
}

/// Stable parameter numbering: 0 is global bypass; each band's five
/// parameters follow in enabled, shape, frequency, gain, Q order.
pub const fn band_parameter_id(band: usize, parameter: BandParameter) -> Option<u32> {
    if band >= BAND_COUNT {
        return None;
    }
    let offset = match parameter {
        BandParameter::Enabled => 0,
        BandParameter::Shape => 1,
        BandParameter::Frequency => 2,
        BandParameter::Gain => 3,
        BandParameter::Q => 4,
    };
    Some(1 + (band as u32) * 5 + offset)
}

/// Convert a host normalized value into a parameter value. Frequencies and Q
/// use logarithmic scaling; gain is centered at normalized 0.5.
pub fn normalized_to_parameter(parameter: BandParameter, normalized: f64) -> f64 {
    let n = finite_or(normalized, 0.0).clamp(0.0, 1.0);
    match parameter {
        BandParameter::Enabled => f64::from(n >= 0.5),
        BandParameter::Shape => (n * 5.0).round(),
        BandParameter::Frequency => {
            MIN_FREQUENCY_HZ * (MAX_FREQUENCY_HZ / MIN_FREQUENCY_HZ).powf(n)
        }
        BandParameter::Gain => (n * 2.0 - 1.0) * MAX_GAIN_DB,
        BandParameter::Q => MIN_Q * (MAX_Q / MIN_Q).powf(n),
    }
}

pub fn parameter_to_normalized(parameter: BandParameter, value: f64) -> f64 {
    let v = finite_or(value, 0.0);
    match parameter {
        BandParameter::Enabled => f64::from(v >= 0.5),
        BandParameter::Shape => (v.round().clamp(0.0, 5.0)) / 5.0,
        BandParameter::Frequency => {
            (v.clamp(MIN_FREQUENCY_HZ, MAX_FREQUENCY_HZ) / MIN_FREQUENCY_HZ).ln()
                / (MAX_FREQUENCY_HZ / MIN_FREQUENCY_HZ).ln()
        }
        BandParameter::Gain => (v.clamp(-MAX_GAIN_DB, MAX_GAIN_DB) / MAX_GAIN_DB + 1.0) * 0.5,
        BandParameter::Q => (v.clamp(MIN_Q, MAX_Q) / MIN_Q).ln() / (MAX_Q / MIN_Q).ln(),
    }
    .clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FilterShape {
    #[default]
    Bell = 0,
    LowShelf = 1,
    HighShelf = 2,
    LowPass = 3,
    HighPass = 4,
    Notch = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub enabled: bool,
    pub shape: FilterShape,
    pub frequency_hz: f64,
    pub gain_db: f64,
    pub q: f64,
}

impl Default for Band {
    fn default() -> Self {
        Self {
            enabled: false,
            shape: FilterShape::Bell,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: std::f64::consts::FRAC_1_SQRT_2,
        }
    }
}

impl Band {
    pub fn validated(mut self, sample_rate: f64) -> Self {
        let nyquist = (sample_rate.max(1.0) * 0.49).max(MIN_FREQUENCY_HZ);
        self.frequency_hz = finite_or(self.frequency_hz, 1_000.0)
            .clamp(MIN_FREQUENCY_HZ, MAX_FREQUENCY_HZ.min(nyquist));
        self.gain_db = finite_or(self.gain_db, 0.0).clamp(-MAX_GAIN_DB, MAX_GAIN_DB);
        self.q = finite_or(self.q, std::f64::consts::FRAC_1_SQRT_2).clamp(MIN_Q, MAX_Q);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameters {
    pub bypassed: bool,
    pub bands: [Band; BAND_COUNT],
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            bypassed: false,
            bands: [Band::default(); BAND_COUNT],
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Coefficients {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Coefficients {
    fn for_band(band: Band, sample_rate: f64) -> Self {
        let band = band.validated(sample_rate);
        let omega = std::f64::consts::TAU * band.frequency_hz / sample_rate.max(1.0);
        let (sin, cos) = omega.sin_cos();
        let alpha = sin / (2.0 * band.q);
        let a = 10_f64.powf(band.gain_db / 40.0);
        let (b0, b1, b2, a0, a1, a2) = match band.shape {
            FilterShape::Bell => (
                1.0 + alpha * a,
                -2.0 * cos,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos,
                1.0 - alpha / a,
            ),
            FilterShape::LowShelf | FilterShape::HighShelf => {
                let root_a = a.sqrt();
                let beta = 2.0 * root_a * alpha;
                let low = band.shape == FilterShape::LowShelf;
                let (b0, b1, b2) = if low {
                    (
                        a * ((a + 1.0) - (a - 1.0) * cos + beta),
                        2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                        a * ((a + 1.0) - (a - 1.0) * cos - beta),
                    )
                } else {
                    (
                        a * ((a + 1.0) + (a - 1.0) * cos + beta),
                        -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                        a * ((a + 1.0) + (a - 1.0) * cos - beta),
                    )
                };
                let (a0, a1, a2) = if low {
                    (
                        (a + 1.0) + (a - 1.0) * cos + beta,
                        -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                        (a + 1.0) + (a - 1.0) * cos - beta,
                    )
                } else {
                    (
                        (a + 1.0) - (a - 1.0) * cos + beta,
                        2.0 * ((a - 1.0) - (a + 1.0) * cos),
                        (a + 1.0) - (a - 1.0) * cos - beta,
                    )
                };
                (b0, b1, b2, a0, a1, a2)
            }
            FilterShape::LowPass => (
                (1.0 - cos) * 0.5,
                1.0 - cos,
                (1.0 - cos) * 0.5,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterShape::HighPass => (
                (1.0 + cos) * 0.5,
                -(1.0 + cos),
                (1.0 + cos) * 0.5,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterShape::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        };
        let inv = 1.0 / a0;
        Self {
            b0: b0 * inv,
            b1: b1 * inv,
            b2: b2 * inv,
            a1: a1 * inv,
            a2: a2 * inv,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct State {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl State {
    fn process(&mut self, c: Coefficients, x: f64) -> f64 {
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        if y.is_finite() {
            y
        } else {
            *self = Self::default();
            0.0
        }
    }
}

/// Prepared stereo equalizer. Reconfigure outside the audio callback; process
/// performs no allocation, locking, or file access.
pub struct Equalizer {
    parameters: Parameters,
    sample_rate: f64,
    coefficients: [Coefficients; BAND_COUNT],
    left: [State; BAND_COUNT],
    right: [State; BAND_COUNT],
    wet: f64,
    target_wet: f64,
    wet_step: f64,
}

impl Equalizer {
    pub fn new(sample_rate: f64) -> Self {
        let sample_rate = finite_or(sample_rate, 48_000.0).max(1.0);
        let parameters = Parameters::default();
        Self {
            parameters,
            sample_rate,
            coefficients: parameters
                .bands
                .map(|b| Coefficients::for_band(b, sample_rate)),
            left: [State::default(); BAND_COUNT],
            right: [State::default(); BAND_COUNT],
            wet: 1.0,
            target_wet: 1.0,
            wet_step: 0.0,
        }
    }

    /// Apply a complete parameter snapshot and smooth bypass over 5 ms.
    pub fn configure(&mut self, parameters: Parameters) {
        self.parameters = parameters;
        self.coefficients = parameters
            .bands
            .map(|b| Coefficients::for_band(b, self.sample_rate));
        self.target_wet = if parameters.bypassed { 0.0 } else { 1.0 };
        self.wet_step = (self.target_wet - self.wet) / (self.sample_rate * 0.005).max(1.0);
    }

    pub fn process(&mut self, left: &mut f32, right: &mut f32) {
        let dry_l = *left as f64;
        let dry_r = *right as f64;
        let mut wet_l = dry_l;
        let mut wet_r = dry_r;
        for i in 0..BAND_COUNT {
            if self.parameters.bands[i].enabled {
                wet_l = self.left[i].process(self.coefficients[i], wet_l);
                wet_r = self.right[i].process(self.coefficients[i], wet_r);
            }
        }
        if self.wet != self.target_wet {
            self.wet += self.wet_step;
            if (self.wet - self.target_wet).abs() < 1e-7 {
                self.wet = self.target_wet;
            }
        }
        *left = (dry_l * (1.0 - self.wet) + wet_l * self.wet) as f32;
        *right = (dry_r * (1.0 - self.wet) + wet_r * self.wet) as f32;
    }
}

/// Combined magnitude response in dB, using the exact biquad coefficients
/// used by processing. Bypassed or disabled bands contribute 0 dB.
pub fn response_db(parameters: &Parameters, frequency_hz: f64, sample_rate: f64) -> f64 {
    if parameters.bypassed || !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return 0.0;
    }
    let omega = std::f64::consts::TAU * frequency_hz / sample_rate.max(1.0);
    let (sin, cos) = omega.sin_cos();
    parameters
        .bands
        .iter()
        .filter(|band| band.enabled)
        .fold(0.0, |sum, band| {
            let c = Coefficients::for_band(*band, sample_rate);
            let nr = c.b0 + c.b1 * cos + c.b2 * (2.0 * omega).cos();
            let ni = -c.b1 * sin - c.b2 * (2.0 * omega).sin();
            let dr = 1.0 + c.a1 * cos + c.a2 * (2.0 * omega).cos();
            let di = -c.a1 * sin - c.a2 * (2.0 * omega).sin();
            let mag_sq = (nr * nr + ni * ni) / (dr * dr + di * di).max(1e-30);
            sum + 10.0 * mag_sq.max(1e-30).log10()
        })
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_rms(eq: &mut Equalizer, frequency: f64, rate: f64) -> f64 {
        let mut sum = 0.0;
        let count = (rate * 0.5) as usize;
        for n in 0..count {
            let x = (std::f64::consts::TAU * frequency * n as f64 / rate).sin() as f32 * 0.1;
            let (mut l, mut r) = (x, x);
            eq.process(&mut l, &mut r);
            if n > count / 2 {
                sum += f64::from(l * l);
            }
        }
        (sum / (count / 2) as f64).sqrt()
    }

    #[test]
    fn bell_response_matches_gain_and_bypass_is_flat() {
        let mut p = Parameters::default();
        p.bands[0] = Band {
            enabled: true,
            gain_db: 6.0,
            ..Band::default()
        };
        assert!((response_db(&p, 1_000.0, 48_000.0) - 6.0).abs() < 0.02);
        p.bypassed = true;
        assert_eq!(response_db(&p, 1_000.0, 48_000.0), 0.0);
    }

    #[test]
    fn bell_audio_gain_agrees_with_calculated_response() {
        let rate = 48_000.0;
        let p = Parameters {
            bands: {
                let mut b = [Band::default(); BAND_COUNT];
                b[0] = Band {
                    enabled: true,
                    gain_db: 6.0,
                    ..Band::default()
                };
                b
            },
            ..Parameters::default()
        };
        let mut eq = Equalizer::new(rate);
        eq.configure(p);
        let output = sine_rms(&mut eq, 1_000.0, rate);
        let gain_db = 20.0 * (output / (0.1_f64 / 2.0_f64.sqrt())).log10();
        assert!((gain_db - 6.0).abs() < 0.15, "measured {gain_db} dB");
    }

    #[test]
    fn invalid_parameter_values_are_bounded() {
        let b = Band {
            frequency_hz: f64::NAN,
            gain_db: f64::INFINITY,
            q: 0.0,
            ..Band::default()
        }
        .validated(48_000.0);
        assert_eq!(b.frequency_hz, 1_000.0);
        assert_eq!(b.gain_db, 0.0);
        assert_eq!(b.q, MIN_Q);
    }

    #[test]
    fn parameter_ids_are_stable_and_ranges_round_trip() {
        assert_eq!(PARAMETER_COUNT, 41);
        assert_eq!(band_parameter_id(0, BandParameter::Enabled), Some(1));
        assert_eq!(band_parameter_id(7, BandParameter::Q), Some(40));
        assert_eq!(band_parameter_id(8, BandParameter::Q), None);
        for (kind, value) in [
            (BandParameter::Frequency, 440.0),
            (BandParameter::Gain, -9.0),
            (BandParameter::Q, 2.5),
        ] {
            let restored = normalized_to_parameter(kind, parameter_to_normalized(kind, value));
            assert!(
                (restored - value).abs() < 1e-9,
                "{kind:?}: {value} -> {restored}"
            );
        }
    }

    #[test]
    fn every_filter_shape_has_finite_response_and_audio() {
        for shape in [
            FilterShape::Bell,
            FilterShape::LowShelf,
            FilterShape::HighShelf,
            FilterShape::LowPass,
            FilterShape::HighPass,
            FilterShape::Notch,
        ] {
            let mut p = Parameters::default();
            p.bands[0] = Band {
                enabled: true,
                shape,
                frequency_hz: 1200.0,
                gain_db: 6.0,
                q: 0.9,
            };
            for hz in [20.0, 120.0, 1200.0, 8000.0, 20000.0] {
                assert!(
                    response_db(&p, hz, 48000.0).is_finite(),
                    "{shape:?} at {hz} Hz"
                );
            }
            let mut eq = Equalizer::new(48000.0);
            eq.configure(p);
            let (mut l, mut r) = (0.0_f32, 0.0_f32);
            for n in 0..4096 {
                let x = (std::f32::consts::TAU * 1400.0 * n as f32 / 48000.0).sin() * 0.25;
                l = x;
                r = -x;
                eq.process(&mut l, &mut r);
                assert!(l.is_finite() && r.is_finite(), "{shape:?} at sample {n}");
            }
        }
    }

    #[test]
    fn parameter_state_round_trips_through_json() {
        let mut p = Parameters::default();
        p.bands[3] = Band {
            enabled: true,
            shape: FilterShape::HighPass,
            frequency_hz: 73.5,
            gain_db: 0.0,
            q: 1.2,
        };
        let encoded = serde_json::to_vec(&p).unwrap();
        let restored: Parameters = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, p);
    }
}
