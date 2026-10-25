//! Bounded sample capture on the callback; calibrated stereo FFT on the reader thread.
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
pub const FFT_SIZE: usize = 2048;
const CAPACITY: usize = 4096;
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
    window: Vec<f32>,
    window_sum: f32,
}
#[derive(Clone, Debug)]
pub struct SpectrumFrame {
    pub sequence: u64,
    pub sample_rate: u32,
    pub bins_dbfs: Vec<f32>,
}
impl SpectrumFrame {
    pub fn frequency_hz(&self, bin: usize) -> f32 {
        bin as f32 * self.sample_rate as f32 / FFT_SIZE as f32
    }
}
impl SpectrumTap {
    pub(crate) fn new() -> Self {
        let window = (0..FFT_SIZE)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FFT_SIZE as f32).cos())
            .collect::<Vec<_>>();
        let window_sum = window.iter().sum();
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
            window,
            window_sum,
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
    pub(crate) fn frame(&self, stage: SpectrumStage) -> Option<SpectrumFrame> {
        if !self.enabled() {
            return None;
        }
        let end = self.written.load(Ordering::Acquire);
        let start = self.start.load(Ordering::Acquire);
        if end.saturating_sub(start) < FFT_SIZE as u64 {
            return None;
        }
        let rate = self.rate.load(Ordering::Acquire);
        if rate == 0 {
            return None;
        }
        let first = end - FFT_SIZE as u64;
        let offset = if stage == SpectrumStage::Pre { 0 } else { 2 };
        let mut left = vec![Complex::default(); FFT_SIZE];
        let mut right = left.clone();
        for i in 0..FFT_SIZE {
            let n = first + i as u64;
            let slot = &self.slots[n as usize % CAPACITY];
            let expected = n.wrapping_mul(2).wrapping_add(2);
            if slot.sequence.load(Ordering::Acquire) != expected {
                return None;
            }
            left[i].re =
                f32::from_bits(slot.samples[offset].load(Ordering::Relaxed)) * self.window[i];
            right[i].re =
                f32::from_bits(slot.samples[offset + 1].load(Ordering::Relaxed)) * self.window[i];
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
        let bins_dbfs = (0..=FFT_SIZE / 2)
            .map(|i| {
                let factor = if i == 0 || i == FFT_SIZE / 2 {
                    1.0
                } else {
                    2.0
                } / self.window_sum;
                // Power-average L/R, preserving anti-phase and single-sided signals.
                let amplitude = ((left[i].power() + right[i].power()) * 0.5).sqrt() * factor;
                (20.0 * amplitude.max(0.000001).log10()).max(-120.0)
            })
            .collect();
        Some(SpectrumFrame {
            sequence: end,
            sample_rate: rate,
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
    #[test]
    fn calibrated_pre_post_fft_keeps_antiphase_stereo_and_discards_old_activation_frames() {
        let tap = SpectrumTap::new();
        tap.enable(true);
        for i in 0..FFT_SIZE {
            let s = (std::f32::consts::TAU * 64.0 * i as f32 / FFT_SIZE as f32).sin();
            tap.record((s, -s), (s * 0.25, -s * 0.25), 48000);
        }
        let pre = tap.frame(SpectrumStage::Pre).unwrap();
        let post = tap.frame(SpectrumStage::Post).unwrap();
        assert!((pre.bins_dbfs[64]).abs() < 0.02);
        assert!((post.bins_dbfs[64] + 12.0412).abs() < 0.02);
        assert_eq!(pre.frequency_hz(64), 1500.0);
        tap.enable(false);
        assert!(tap.frame(SpectrumStage::Post).is_none());
        tap.enable(true);
        assert!(tap.frame(SpectrumStage::Post).is_none());
        for _ in 0..FFT_SIZE {
            tap.record((0.0, 0.0), (0.0, 0.0), 44100);
        }
        let silent = tap.frame(SpectrumStage::Post).unwrap();
        assert_eq!(silent.sample_rate, 44100);
        assert!(silent.bins_dbfs.iter().all(|db| *db <= -119.0));
    }
}
