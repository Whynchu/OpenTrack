//! Signed per-channel envelopes, built outside the realtime callback.
use std::sync::Arc;
pub const WAVEFORM_BLOCK_FRAMES: usize = 64;
const CHUNK: usize = 2048;
pub type WaveformBin = [[f32; 2]; 2]; // [channel][minimum, maximum]
#[derive(Clone, Debug, Default)]
pub struct Waveform {
    chunks: Vec<Arc<Vec<WaveformBin>>>,
    pub frames: u64,
    pub sample_rate: u32,
    pub channels: u16,
}
impl Waveform {
    pub fn duration_seconds(&self) -> f64 {
        self.frames as f64 / self.sample_rate.max(1) as f64
    }
    fn bin(&self, index: usize) -> Option<&WaveformBin> {
        self.chunks.get(index / CHUNK)?.get(index % CHUNK)
    }
    /// One signed range per visible column; aggregate every covered bin, never decimate peaks.
    pub fn columns(
        &self,
        start_seconds: f64,
        end_seconds: f64,
        count: usize,
        channel: usize,
    ) -> Vec<[f32; 2]> {
        if count == 0 || self.frames == 0 || end_seconds <= start_seconds {
            return Vec::new();
        }
        let start = (start_seconds.max(0.0) * self.sample_rate as f64).min(self.frames as f64);
        let end = (end_seconds.max(0.0) * self.sample_rate as f64).min(self.frames as f64);
        (0..count)
            .map(|i| {
                let a = (start + (end - start) * i as f64 / count as f64).floor() as usize
                    / WAVEFORM_BLOCK_FRAMES;
                let b = ((start + (end - start) * (i + 1) as f64 / count as f64).ceil() as usize)
                    .div_ceil(WAVEFORM_BLOCK_FRAMES)
                    .max(a + 1);
                let mut range = [f32::INFINITY, f32::NEG_INFINITY];
                for n in a..b {
                    if let Some(bin) = self.bin(n) {
                        let r = bin[channel.min(1)];
                        range[0] = range[0].min(r[0]);
                        range[1] = range[1].max(r[1]);
                    }
                }
                if range[0].is_finite() {
                    range
                } else {
                    [0.0; 2]
                }
            })
            .collect()
    }
}
pub(crate) struct WaveformBuilder {
    chunks: Vec<Arc<Vec<WaveformBin>>>,
    tail: Vec<WaveformBin>,
    current: WaveformBin,
    used: usize,
    frames: u64,
    rate: u32,
    channels: u16,
}
impl WaveformBuilder {
    pub(crate) fn new(rate: u32, channels: u16) -> Self {
        Self {
            chunks: Vec::new(),
            tail: Vec::new(),
            current: [[0.0; 2]; 2],
            used: 0,
            frames: 0,
            rate,
            channels,
        }
    }
    pub(crate) fn push(&mut self, left: f32, right: f32) {
        for (c, s) in [left, right].into_iter().enumerate() {
            let s = if s.is_finite() { s } else { 0.0 };
            if self.used == 0 {
                self.current[c] = [s, s];
            } else {
                self.current[c][0] = self.current[c][0].min(s);
                self.current[c][1] = self.current[c][1].max(s);
            }
        }
        self.frames += 1;
        self.used += 1;
        if self.used == WAVEFORM_BLOCK_FRAMES {
            self.tail.push(self.current);
            self.used = 0;
            if self.tail.len() == CHUNK {
                self.chunks.push(Arc::new(std::mem::take(&mut self.tail)));
            }
        }
    }
    pub(crate) fn snapshot(&self) -> Waveform {
        let mut chunks = self.chunks.clone();
        let mut tail = self.tail.clone();
        if self.used > 0 {
            tail.push(self.current);
        }
        if !tail.is_empty() {
            chunks.push(Arc::new(tail));
        }
        Waveform {
            chunks,
            frames: self.frames,
            sample_rate: self.rate,
            channels: self.channels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_stereo_peaks_and_partial_tail_survive_dense_and_coarse_views() {
        let mut b = WaveformBuilder::new(48000, 2);
        for i in 0..64001 {
            b.push(
                if i == 33000 { -0.9 } else { 0.2 },
                if i == 63999 { 0.8 } else { -0.1 },
            );
        }
        let w = b.snapshot();
        assert_eq!(w.frames, 64001);
        let left = w.columns(0.0, w.duration_seconds(), 1, 0);
        assert_eq!(left, vec![[-0.9, 0.2]]);
        assert_eq!(
            w.columns(0.0, w.duration_seconds(), 1, 1),
            vec![[-0.1, 0.8]]
        );
        assert!(
            w.columns(0.0, w.duration_seconds(), 2048, 0)
                .iter()
                .any(|p| p[0] == -0.9)
        );
        let old = w.frames;
        b.push(1.0, 1.0);
        assert_eq!(w.frames, old);
        assert_eq!(b.snapshot().frames, old + 1);
    }
}
