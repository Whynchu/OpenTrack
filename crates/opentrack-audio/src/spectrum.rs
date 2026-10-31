//! Bounded sample capture on the callback; calibrated stereo FFT on the reader thread.
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
// Double the prior FFT window to give low frequencies denser bins. The capture
// FFT work remains on the UI/control side, outside the audio callback.
pub const FFT_SIZE: usize = 4096;
/// Long-window tonal analysis uses the tap's 8192-point window and a denser
/// normalized log-frequency profile for a smoother target/comparison curve.
pub const TONAL_FFT_SIZE: usize = FFT_SIZE * 2;
pub const TONAL_PROFILE_BANDS: usize = 96;
const CAPACITY: usize = 8192;
const TONAL_HOP_SIZE: usize = TONAL_FFT_SIZE / 2;
// RMS gate is measured from unwindowed samples, so equal-level tones at
// different frequencies receive the same silence decision.
const TONAL_SILENCE_GATE_DBFS: f32 = -90.0;
const TONAL_SILENCE_GATE_POWER: f64 = 1.0e-9;
const MAX_REFERENCE_DECODED_BYTES: u64 = 512 * 1024 * 1024;
const MIN_REFERENCE_SECONDS: f64 = 1.0;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum SpectrumResolution {
    Fast = FFT_SIZE / 2,
    #[default]
    Balanced = FFT_SIZE,
    Fine = FFT_SIZE * 2,
}
impl SpectrumResolution {
    pub const fn fft_size(self) -> usize {
        self as usize
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpectrumStage {
    Pre,
    #[default]
    Post,
}
struct Slot {
    sequence: AtomicU64,
    samples: [AtomicU32; 4],
}
pub(crate) struct SpectrumTap {
    enabled: AtomicBool,
    written: AtomicU64,
    start: AtomicU64,
    rate: AtomicU32,
    slots: Box<[Slot]>,
    windows: [Vec<f32>; 3],
    window_sums: [f32; 3],
}
#[derive(Clone, Debug)]
pub struct SpectrumFrame {
    pub sequence: u64,
    pub sample_rate: u32,
    pub fft_size: usize,
    pub rms_dbfs: f32,
    pub bins_dbfs: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TonalProfile {
    pub sample_rate: u32,
    pub duration_seconds: f64,
    pub analyzed_windows: u64,
    pub band_powers: Vec<f32>,
}

/// Reads a mono/stereo PCM or float WAV and creates the same normalized
/// 96-band power profile used by the live tonal analyzer. Decoding and FFT work
/// run on the caller's worker thread; cancellation is checked while reading.
pub fn analyze_tonal_reference_wav(
    path: &Path,
    cancelled: &AtomicBool,
) -> Result<TonalProfile, String> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|error| format!("could not open reference WAV: {error}"))?;
    let spec = reader.spec();
    if !(1..=2).contains(&spec.channels) || !(8_000..=384_000).contains(&spec.sample_rate) {
        return Err("reference WAV must be mono or stereo at 8–384 kHz".to_owned());
    }
    if !matches!(
        spec.sample_format,
        hound::SampleFormat::Int | hound::SampleFormat::Float
    ) || !(8..=32).contains(&spec.bits_per_sample)
        || (spec.sample_format == hound::SampleFormat::Float && spec.bits_per_sample != 32)
    {
        return Err("reference WAV must use 8–32 bit PCM or 32-bit float samples".to_owned());
    }
    let frame_count = u64::from(reader.duration());
    let duration_seconds = frame_count as f64 / f64::from(spec.sample_rate);
    if duration_seconds < MIN_REFERENCE_SECONDS {
        return Err("reference audio must contain at least one second".to_owned());
    }
    let decoded_bytes = frame_count
        .checked_mul(u64::from(spec.channels))
        .and_then(|count| count.checked_mul(std::mem::size_of::<f32>() as u64))
        .ok_or_else(|| "reference WAV is too large to analyze".to_owned())?;
    if decoded_bytes > MAX_REFERENCE_DECODED_BYTES {
        return Err("reference WAV exceeds the 512 MiB analysis limit".to_owned());
    }
    if cancelled.load(Ordering::Acquire) {
        return Err("reference analysis cancelled".to_owned());
    }

    let mut accumulator = TonalAccumulator::new(spec.sample_rate);
    match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = 2.0_f64.powi(i32::from(spec.bits_per_sample) - 1);
            let mut samples = reader.samples::<i32>();
            for frame in 0..frame_count {
                if frame & 0x3fff == 0 && cancelled.load(Ordering::Acquire) {
                    return Err("reference analysis cancelled".to_owned());
                }
                let left = samples
                    .next()
                    .transpose()
                    .map_err(|error| format!("could not decode reference WAV: {error}"))?
                    .ok_or_else(|| "reference WAV ended before its header length".to_owned())?;
                let right = if spec.channels == 2 {
                    samples
                        .next()
                        .transpose()
                        .map_err(|error| format!("could not decode reference WAV: {error}"))?
                        .ok_or_else(|| "reference WAV ended before its header length".to_owned())?
                } else {
                    left
                };
                accumulator.push(
                    (f64::from(left) / scale).clamp(-1.0, 1.0) as f32,
                    (f64::from(right) / scale).clamp(-1.0, 1.0) as f32,
                );
            }
        }
        hound::SampleFormat::Float => {
            let mut samples = reader.samples::<f32>();
            for frame in 0..frame_count {
                if frame & 0x3fff == 0 && cancelled.load(Ordering::Acquire) {
                    return Err("reference analysis cancelled".to_owned());
                }
                let left = samples
                    .next()
                    .transpose()
                    .map_err(|error| format!("could not decode reference WAV: {error}"))?
                    .ok_or_else(|| "reference WAV ended before its header length".to_owned())?;
                let right = if spec.channels == 2 {
                    samples
                        .next()
                        .transpose()
                        .map_err(|error| format!("could not decode reference WAV: {error}"))?
                        .ok_or_else(|| "reference WAV ended before its header length".to_owned())?
                } else {
                    left
                };
                accumulator.push(
                    if left.is_finite() {
                        left.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    },
                    if right.is_finite() {
                        right.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    },
                );
            }
        }
    }
    if cancelled.load(Ordering::Acquire) {
        return Err("reference analysis cancelled".to_owned());
    }
    let analyzed_windows = accumulator.windows_analyzed;
    let band_powers = accumulator.finish()?;
    Ok(TonalProfile {
        sample_rate: spec.sample_rate,
        duration_seconds,
        analyzed_windows,
        band_powers,
    })
}

struct TonalAccumulator {
    sample_rate: u32,
    window: Vec<f32>,
    window_sum: f32,
    raw_left: Vec<f32>,
    raw_right: Vec<f32>,
    filled: usize,
    fft_left: Vec<Complex>,
    fft_right: Vec<Complex>,
    band_sums: [f64; TONAL_PROFILE_BANDS],
    windows_seen: u64,
    windows_analyzed: u64,
}

impl TonalAccumulator {
    fn new(sample_rate: u32) -> Self {
        let window = (0..TONAL_FFT_SIZE)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / TONAL_FFT_SIZE as f32).cos())
            .collect::<Vec<_>>();
        let window_sum = window.iter().sum();
        Self {
            sample_rate,
            window,
            window_sum,
            raw_left: vec![0.0; TONAL_FFT_SIZE],
            raw_right: vec![0.0; TONAL_FFT_SIZE],
            filled: 0,
            fft_left: vec![Complex::default(); TONAL_FFT_SIZE],
            fft_right: vec![Complex::default(); TONAL_FFT_SIZE],
            band_sums: [0.0; TONAL_PROFILE_BANDS],
            windows_seen: 0,
            windows_analyzed: 0,
        }
    }

    fn push(&mut self, left: f32, right: f32) {
        self.raw_left[self.filled] = left;
        self.raw_right[self.filled] = right;
        self.filled += 1;
        if self.filled != TONAL_FFT_SIZE {
            return;
        }
        self.windows_seen += 1;
        let mut mean_power = 0.0_f64;
        for (left, right) in self.raw_left.iter().zip(&self.raw_right) {
            mean_power +=
                (f64::from(*left) * f64::from(*left) + f64::from(*right) * f64::from(*right)) * 0.5;
        }
        mean_power /= TONAL_FFT_SIZE as f64;
        if mean_power >= TONAL_SILENCE_GATE_POWER {
            for i in 0..TONAL_FFT_SIZE {
                self.fft_left[i].re = self.raw_left[i] * self.window[i];
                self.fft_left[i].im = 0.0;
                self.fft_right[i].re = self.raw_right[i] * self.window[i];
                self.fft_right[i].im = 0.0;
            }
            fft(&mut self.fft_left);
            fft(&mut self.fft_right);
            let profile = normalized_fft_band_powers(
                &self.fft_left,
                &self.fft_right,
                self.sample_rate,
                self.window_sum,
            );
            for (sum, power) in self.band_sums.iter_mut().zip(profile) {
                *sum += f64::from(power);
            }
            self.windows_analyzed += 1;
        }
        self.raw_left.copy_within(TONAL_HOP_SIZE.., 0);
        self.raw_right.copy_within(TONAL_HOP_SIZE.., 0);
        self.filled = TONAL_FFT_SIZE - TONAL_HOP_SIZE;
    }

    fn finish(self) -> Result<Vec<f32>, String> {
        if self.windows_seen == 0 {
            return Err("reference WAV is too short to analyze".to_owned());
        }
        if self.windows_analyzed == 0 {
            return Err("reference WAV is silent or too quiet to analyze".to_owned());
        }
        let mut powers = self
            .band_sums
            .iter()
            .map(|sum| sum / self.windows_analyzed as f64)
            .collect::<Vec<_>>();
        let total = powers.iter().sum::<f64>();
        if total <= 0.0 {
            return Err("reference WAV is silent or too quiet to analyze".to_owned());
        }
        powers.iter_mut().for_each(|power| *power /= total);
        Ok(powers.into_iter().map(|power| power as f32).collect())
    }
}

fn normalized_fft_band_powers(
    left: &[Complex],
    right: &[Complex],
    sample_rate: u32,
    window_sum: f32,
) -> [f32; TONAL_PROFILE_BANDS] {
    let mut sums = [0.0_f64; TONAL_PROFILE_BANDS];
    let mut weights = [0.0_f64; TONAL_PROFILE_BANDS];
    let log_range = (20_000.0_f32 / 20.0).ln();
    for i in 1..=left.len() / 2 {
        let hz = i as f32 * sample_rate as f32 / left.len() as f32;
        if !(20.0..=20_000.0).contains(&hz) {
            continue;
        }
        let factor = if i == left.len() / 2 { 1.0 } else { 2.0 } / window_sum;
        let amplitude = ((left[i].power() + right[i].power()) * 0.5).sqrt() * factor;
        let db = (20.0 * amplitude.max(0.000001).log10()).max(-120.0);
        let position = ((hz / 20.0).ln() / log_range * (TONAL_PROFILE_BANDS - 1) as f32)
            .clamp(0.0, (TONAL_PROFILE_BANDS - 1) as f32);
        let lower = position.floor() as usize;
        let upper = (lower + 1).min(TONAL_PROFILE_BANDS - 1);
        let upper_weight = f64::from(position - lower as f32);
        let power = 10.0_f64.powf(f64::from(db) / 10.0);
        sums[lower] += power * (1.0 - upper_weight);
        weights[lower] += 1.0 - upper_weight;
        if upper != lower {
            sums[upper] += power * upper_weight;
            weights[upper] += upper_weight;
        }
    }
    for (sum, weight) in sums.iter_mut().zip(weights) {
        if weight > 0.0 {
            *sum /= weight;
        }
    }
    let total = sums.iter().sum::<f64>();
    if total <= 0.0 {
        return [0.0; TONAL_PROFILE_BANDS];
    }
    sums.map(|power| (power / total) as f32)
}
impl SpectrumFrame {
    pub fn frequency_hz(&self, bin: usize) -> f32 {
        bin as f32 * self.sample_rate as f32 / self.fft_size.max(1) as f32
    }

    /// Mean power in 96 interpolated logarithmic frequency regions, normalized to sum to
    /// one. All-silent or unusably quiet frames return zeroes for every band.
    pub fn normalized_log_band_powers(&self) -> Vec<f32> {
        const BANDS: usize = TONAL_PROFILE_BANDS;
        if !self.rms_dbfs.is_finite() || self.rms_dbfs < TONAL_SILENCE_GATE_DBFS {
            return vec![0.0; BANDS];
        }
        let mut sums = [0.0_f64; BANDS];
        let log_range = (20_000.0_f32 / 20.0).ln();
        let mut weights = [0.0_f64; BANDS];
        for (index, db) in self.bins_dbfs.iter().enumerate().skip(1) {
            let hz = self.frequency_hz(index);
            if !(20.0..=20_000.0).contains(&hz) || !db.is_finite() {
                continue;
            }
            let position =
                ((hz / 20.0).ln() / log_range * (BANDS - 1) as f32).clamp(0.0, (BANDS - 1) as f32);
            let lower = position.floor() as usize;
            let upper = (lower + 1).min(BANDS - 1);
            let upper_weight = f64::from(position - lower as f32);
            let power = 10.0_f64.powf(f64::from(*db) / 10.0);
            sums[lower] += power * (1.0 - upper_weight);
            weights[lower] += 1.0 - upper_weight;
            if upper != lower {
                sums[upper] += power * upper_weight;
                weights[upper] += upper_weight;
            }
        }
        for (sum, weight) in sums.iter_mut().zip(weights) {
            if weight > 0.0 {
                *sum /= weight;
            }
        }
        let total: f64 = sums.iter().sum();
        if total <= 0.0 {
            return vec![0.0; BANDS];
        }
        sums.iter_mut().for_each(|power| *power /= total);
        sums.into_iter().map(|power| power as f32).collect()
    }
}

/// Per-band tonal difference for two normalized profile inputs. Inputs are
/// normalized independently so program level does not affect the comparison.
pub fn normalized_log_band_difference_db(measured: &[f32], target: &[f32]) -> Option<Vec<f32>> {
    if measured.len() != target.len()
        || !matches!(measured.len(), 24 | 48 | TONAL_PROFILE_BANDS)
        || measured
            .iter()
            .chain(target)
            .any(|power| !power.is_finite() || *power < 0.0)
    {
        return None;
    }
    let measured_sum = measured.iter().map(|power| f64::from(*power)).sum::<f64>();
    let target_sum = target.iter().map(|power| f64::from(*power)).sum::<f64>();
    if measured_sum <= 0.0 || target_sum <= 0.0 {
        return None;
    }
    Some(
        measured
            .iter()
            .zip(target)
            .map(|(measured, target)| {
                let measured = (f64::from(*measured) / measured_sum).max(1.0e-8);
                let target = (f64::from(*target) / target_sum).max(1.0e-8);
                (10.0 * (measured / target).log10()).clamp(-24.0, 24.0) as f32
            })
            .collect(),
    )
}
impl SpectrumTap {
    pub(crate) fn new() -> Self {
        let windows = [2048, 4096, 8192].map(|size| {
            (0..size)
                .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / size as f32).cos())
                .collect::<Vec<_>>()
        });
        let window_sums = windows.each_ref().map(|window| window.iter().sum());
        Self {
            enabled: AtomicBool::new(false),
            written: AtomicU64::new(0),
            start: AtomicU64::new(0),
            rate: AtomicU32::new(0),
            slots: (0..CAPACITY)
                .map(|_| Slot {
                    sequence: AtomicU64::new(0),
                    samples: std::array::from_fn(|_| AtomicU32::new(0)),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            windows,
            window_sums,
        }
    }
    pub(crate) fn enable(&self, enabled: bool) {
        if enabled && !self.enabled.load(Ordering::Acquire) {
            self.start
                .store(self.written.load(Ordering::Acquire), Ordering::Release);
        }
        self.enabled.store(enabled, Ordering::Release);
    }
    pub(crate) fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
    pub(crate) fn record(&self, pre: (f32, f32), post: (f32, f32), rate: u32) {
        if !self.enabled() {
            return;
        }
        let n = self.written.load(Ordering::Relaxed);
        if self.rate.load(Ordering::Relaxed) != rate {
            self.start.store(n, Ordering::Release);
            self.rate.store(rate, Ordering::Release);
        }
        let slot = &self.slots[n as usize % CAPACITY];
        slot.sequence
            .store(n.wrapping_mul(2).wrapping_add(1), Ordering::Relaxed);
        std::sync::atomic::fence(Ordering::Release);
        for (value, sample) in slot.samples.iter().zip([pre.0, pre.1, post.0, post.1]) {
            value.store(
                if sample.is_finite() { sample } else { 0.0 }.to_bits(),
                Ordering::Relaxed,
            );
        }
        slot.sequence
            .store(n.wrapping_mul(2).wrapping_add(2), Ordering::Release);
        self.written.store(n.wrapping_add(1), Ordering::Release);
    }
    pub(crate) fn frame(
        &self,
        stage: SpectrumStage,
        resolution: SpectrumResolution,
    ) -> Option<SpectrumFrame> {
        if !self.enabled() {
            return None;
        }
        let end = self.written.load(Ordering::Acquire);
        let start = self.start.load(Ordering::Acquire);
        let fft_size = resolution.fft_size();
        if end.saturating_sub(start) < fft_size as u64 {
            return None;
        }
        let rate = self.rate.load(Ordering::Acquire);
        if rate == 0 {
            return None;
        }
        let first = end - fft_size as u64;
        let window_index = match resolution {
            SpectrumResolution::Fast => 0,
            SpectrumResolution::Balanced => 1,
            SpectrumResolution::Fine => 2,
        };
        let window = &self.windows[window_index];
        let window_sum = self.window_sums[window_index];
        let offset = if stage == SpectrumStage::Pre { 0 } else { 2 };
        let mut left = vec![Complex::default(); fft_size];
        let mut right = left.clone();
        let mut mean_power = 0.0_f64;
        for i in 0..fft_size {
            let n = first + i as u64;
            let slot = &self.slots[n as usize % CAPACITY];
            let expected = n.wrapping_mul(2).wrapping_add(2);
            if slot.sequence.load(Ordering::Acquire) != expected {
                return None;
            }
            let left_sample = f32::from_bits(slot.samples[offset].load(Ordering::Relaxed));
            let right_sample = f32::from_bits(slot.samples[offset + 1].load(Ordering::Relaxed));
            mean_power += (f64::from(left_sample) * f64::from(left_sample)
                + f64::from(right_sample) * f64::from(right_sample))
                * 0.5;
            left[i].re = left_sample * window[i];
            right[i].re = right_sample * window[i];
            std::sync::atomic::fence(Ordering::Acquire);
            if slot.sequence.load(Ordering::Acquire) != expected {
                return None;
            }
        }
        if self.rate.load(Ordering::Acquire) != rate || self.start.load(Ordering::Acquire) != start
        {
            return None;
        }
        fft(&mut left);
        fft(&mut right);
        let bins_dbfs = (0..=fft_size / 2)
            .map(|i| {
                let factor = if i == 0 || i == fft_size / 2 {
                    1.0
                } else {
                    2.0
                } / window_sum;
                // Power-average L/R, preserving anti-phase and single-sided signals.
                let amplitude = ((left[i].power() + right[i].power()) * 0.5).sqrt() * factor;
                (20.0 * amplitude.max(0.000001).log10()).max(-120.0)
            })
            .collect();
        Some(SpectrumFrame {
            sequence: end,
            sample_rate: rate,
            fft_size,
            rms_dbfs: if mean_power > 0.0 {
                (10.0 * (mean_power / fft_size as f64).log10()).max(-120.0) as f32
            } else {
                -120.0
            },
            bins_dbfs,
        })
    }
}
#[derive(Clone, Copy, Default)]
struct Complex {
    re: f32,
    im: f32,
}
impl Complex {
    fn power(self) -> f32 {
        self.re * self.re + self.im * self.im
    }
}
fn fft(data: &mut [Complex]) {
    let n = data.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            data.swap(i, j);
        }
    }
    let mut width = 2;
    while width <= n {
        let angle = -std::f32::consts::TAU / width as f32;
        let (sin, cos) = angle.sin_cos();
        for start in (0..n).step_by(width) {
            let mut w = Complex { re: 1.0, im: 0.0 };
            for i in 0..width / 2 {
                let a = data[start + i];
                let b = data[start + i + width / 2];
                let v = Complex {
                    re: b.re * w.re - b.im * w.im,
                    im: b.re * w.im + b.im * w.re,
                };
                data[start + i] = Complex {
                    re: a.re + v.re,
                    im: a.im + v.im,
                };
                data[start + i + width / 2] = Complex {
                    re: a.re - v.re,
                    im: a.im - v.im,
                };
                w = Complex {
                    re: w.re * cos - w.im * sin,
                    im: w.re * sin + w.im * cos,
                };
            }
        }
        width *= 2;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn temp_reference_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "opentrack-tonal-reference-{}-{label}.wav",
            std::process::id()
        ))
    }

    fn write_pcm_reference(path: &Path, sample_fn: impl FnMut(u64) -> (i16, i16)) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for (left, right) in (0_u64..48_000 * 2).map(sample_fn) {
            writer.write_sample(left).unwrap();
            writer.write_sample(right).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn imported_reference_averages_active_windows_and_preserves_stereo_energy() {
        let path = temp_reference_path("active");
        write_pcm_reference(&path, |frame| {
            if frame < 48_000 {
                (0, 0)
            } else {
                let sample = (std::f32::consts::TAU * 937.5 * frame as f32 / 48_000.0).sin();
                let sample = (sample * 12_000.0) as i16;
                (sample, -sample)
            }
        });

        let profile = analyze_tonal_reference_wav(&path, &AtomicBool::new(false)).unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(profile.sample_rate, 48_000);
        assert!((profile.duration_seconds - 2.0).abs() < 1.0e-9);
        assert!(profile.analyzed_windows > 0);
        assert_eq!(profile.band_powers.len(), TONAL_PROFILE_BANDS);
        assert!((profile.band_powers.iter().sum::<f32>() - 1.0).abs() < 1.0e-5);
        let peak_band = profile
            .band_powers
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .unwrap()
            .0;
        assert!(
            (52..=54).contains(&peak_band),
            "unexpected peak band {peak_band}"
        );
    }

    #[test]
    fn imported_reference_matches_live_tonal_profile_with_identical_analysis_settings() {
        let path = temp_reference_path("live-profile-match");
        write_pcm_reference(&path, |frame| {
            let value = (std::f32::consts::TAU * 750.0 * frame as f32 / 48_000.0).sin();
            let sample = (value * 16_000.0).round() as i16;
            (sample, -sample)
        });

        let imported = analyze_tonal_reference_wav(&path, &AtomicBool::new(false)).unwrap();
        let _ = std::fs::remove_file(path);

        let tap = SpectrumTap::new();
        tap.enable(true);
        for frame in 0_u64..CAPACITY as u64 {
            let value = (std::f32::consts::TAU * 750.0 * frame as f32 / 48_000.0).sin();
            let sample = (value * 16_000.0).round() as i16;
            let left = f32::from(sample) / 32768.0;
            let right = f32::from(-sample) / 32768.0;
            tap.record((left, right), (left, right), 48_000);
        }
        let live = tap
            .frame(SpectrumStage::Post, SpectrumResolution::Fine)
            .unwrap()
            .normalized_log_band_powers();

        assert_eq!(imported.band_powers.len(), live.len());
        assert!(
            imported
                .band_powers
                .iter()
                .zip(&live)
                .all(|(imported, live)| (imported - live).abs() < 1.0e-5)
        );
    }

    #[test]
    fn imported_reference_rejects_silence_and_honors_cancellation() {
        let path = temp_reference_path("silent");
        write_pcm_reference(&path, |_| (0, 0));
        let silent = analyze_tonal_reference_wav(&path, &AtomicBool::new(false));
        assert!(silent.unwrap_err().contains("silent"));
        let cancelled = analyze_tonal_reference_wav(&path, &AtomicBool::new(true));
        assert!(cancelled.unwrap_err().contains("cancelled"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn tonal_silence_gate_uses_window_rms_independent_of_tone_frequency() {
        let mut below_gate = TonalAccumulator::new(48_000);
        for frame in 0..TONAL_FFT_SIZE as u64 * 2 {
            let sample = (std::f32::consts::TAU * 1_000.0 * frame as f32 / 48_000.0).sin() * 1.0e-5;
            below_gate.push(sample, -sample);
        }
        assert_eq!(below_gate.windows_analyzed, 0);
        assert!(below_gate.finish().unwrap_err().contains("too quiet"));

        for frequency in [80.0, 10_000.0] {
            let mut accumulator = TonalAccumulator::new(48_000);
            for frame in 0..TONAL_FFT_SIZE as u64 * 2 {
                let sample =
                    (std::f32::consts::TAU * frequency * frame as f32 / 48_000.0).sin() * 5.0e-5;
                accumulator.push(sample, -sample);
            }
            assert!(accumulator.windows_analyzed > 0, "{frequency} Hz rejected");
            assert!(accumulator.finish().is_ok());
        }
    }

    #[test]
    fn calibrated_pre_post_fft_keeps_antiphase_stereo_and_discards_old_activation_frames() {
        let tap = SpectrumTap::new();
        tap.enable(true);
        for i in 0..FFT_SIZE {
            let s = (std::f32::consts::TAU * 1_500.0 * i as f32 / 48_000.0).sin();
            tap.record((s, -s), (s * 0.25, -s * 0.25), 48000);
        }
        let pre = tap
            .frame(SpectrumStage::Pre, SpectrumResolution::Balanced)
            .unwrap();
        let post = tap
            .frame(SpectrumStage::Post, SpectrumResolution::Balanced)
            .unwrap();
        assert!((pre.bins_dbfs[128]).abs() < 0.02);
        assert!((post.bins_dbfs[128] + 12.0412).abs() < 0.02);
        assert_eq!(pre.frequency_hz(128), 1500.0);
        tap.enable(false);
        assert!(
            tap.frame(SpectrumStage::Post, SpectrumResolution::Balanced)
                .is_none()
        );
        tap.enable(true);
        assert!(
            tap.frame(SpectrumStage::Post, SpectrumResolution::Balanced)
                .is_none()
        );
        for _ in 0..FFT_SIZE {
            tap.record((0.0, 0.0), (0.0, 0.0), 44100);
        }
        let silent = tap
            .frame(SpectrumStage::Post, SpectrumResolution::Balanced)
            .unwrap();
        assert_eq!(silent.sample_rate, 44100);
        assert!(silent.bins_dbfs.iter().all(|db| *db <= -119.0));
    }

    #[test]
    fn supported_fft_sizes_change_frequency_spacing_without_level_bias() {
        let tap = SpectrumTap::new();
        tap.enable(true);
        for i in 0..CAPACITY {
            let sample = (std::f32::consts::TAU * 1_500.0 * i as f32 / 48_000.0).sin();
            tap.record((sample, sample), (sample, sample), 48_000);
        }
        for (resolution, bin) in [
            (SpectrumResolution::Fast, 64),
            (SpectrumResolution::Balanced, 128),
            (SpectrumResolution::Fine, 256),
        ] {
            let frame = tap.frame(SpectrumStage::Post, resolution).unwrap();
            assert_eq!(frame.fft_size, resolution.fft_size());
            assert!((frame.frequency_hz(bin) - 1_500.0).abs() < 0.01);
            assert!(frame.bins_dbfs[bin].abs() < 0.05);
        }
    }

    #[test]
    fn tonal_band_power_is_normalized_and_silence_has_no_target_energy() {
        let mut bins = vec![-120.0; TONAL_FFT_SIZE / 2 + 1];
        bins[128] = -6.0;
        bins[1_024] = -12.0;
        let frame = SpectrumFrame {
            sequence: 1,
            sample_rate: 48_000,
            fft_size: TONAL_FFT_SIZE,
            rms_dbfs: -20.0,
            bins_dbfs: bins,
        };
        let profile = frame.normalized_log_band_powers();
        assert_eq!(profile.len(), TONAL_PROFILE_BANDS);
        assert!((profile.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        assert!(profile.iter().filter(|power| **power > 0.0).count() >= 2);
        let silence = SpectrumFrame {
            sequence: 2,
            sample_rate: 48_000,
            fft_size: TONAL_FFT_SIZE,
            rms_dbfs: -120.0,
            bins_dbfs: vec![-120.0; TONAL_FFT_SIZE / 2 + 1],
        };
        assert!(
            silence
                .normalized_log_band_powers()
                .iter()
                .all(|power| *power == 0.0)
        );
        let mut below_gate = vec![-120.0; TONAL_FFT_SIZE / 2 + 1];
        below_gate[64] = -100.0;
        let quiet = SpectrumFrame {
            sequence: 3,
            sample_rate: 48_000,
            fft_size: TONAL_FFT_SIZE,
            rms_dbfs: -100.0,
            bins_dbfs: below_gate,
        };
        assert!(
            quiet
                .normalized_log_band_powers()
                .iter()
                .all(|power| *power == 0.0)
        );
        let mut above_gate = vec![-120.0; TONAL_FFT_SIZE / 2 + 1];
        above_gate[128] = -70.0;
        let active = SpectrumFrame {
            sequence: 4,
            sample_rate: 48_000,
            fft_size: TONAL_FFT_SIZE,
            rms_dbfs: -70.0,
            bins_dbfs: above_gate,
        };
        assert!(
            active
                .normalized_log_band_powers()
                .iter()
                .any(|power| *power > 0.0)
        );
    }

    #[test]
    fn tonal_difference_is_gain_invariant_and_identical_profiles_match() {
        let mut measured = vec![0.0; 24];
        measured[2] = 0.7;
        measured[14] = 0.3;
        let target = measured.iter().map(|power| power * 4.0).collect::<Vec<_>>();
        let difference = normalized_log_band_difference_db(&measured, &target).unwrap();
        assert!(difference.iter().all(|db| db.abs() < 1.0e-5));
        let scaled = measured
            .iter()
            .map(|power| power * 0.125)
            .collect::<Vec<_>>();
        let scaled_difference = normalized_log_band_difference_db(&scaled, &target).unwrap();
        assert!(scaled_difference.iter().all(|db| db.abs() < 1.0e-5));
        assert!(normalized_log_band_difference_db(&[0.0; 24], &target).is_none());
    }
}
