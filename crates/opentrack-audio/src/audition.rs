//! Dedicated note-drag audition voice.
//!
//! The preview is prepared on the control thread from the current pitch/time
//! source. The callback only reads immutable sources and advances atomics, so a
//! drag never moves the project playhead or rebuilds a playback plan.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, AtomicU64, Ordering},
};

use arc_swap::{ArcSwap, ArcSwapOption};

use crate::streaming::PlaybackSource;

const FADE_SECONDS: f64 = 0.005;

#[derive(Debug)]
struct NoteLoop {
    source: PlaybackSource,
    start_frame: u64,
    end_frame: u64,
    level: f32,
    position: AtomicU64,
    target_gain: AtomicU32,
    current_gain: AtomicU32,
}

impl NoteLoop {
    fn new(
        source: PlaybackSource,
        source_offset_seconds: f64,
        start_seconds: f64,
        end_seconds: f64,
        level: f32,
    ) -> Result<Self, String> {
        let rate = f64::from(source.sample_rate());
        let frame_count = source.frame_count();
        let start = ((source_offset_seconds + start_seconds) * rate).round();
        let end = ((source_offset_seconds + end_seconds) * rate).round();
        if !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || end <= start
            || end > frame_count as f64
        {
            return Err("Note audition range is outside its source".into());
        }
        Ok(Self {
            source,
            start_frame: start as u64,
            end_frame: end as u64,
            level: level.clamp(0.0, 2.0),
            position: AtomicU64::new(0),
            target_gain: AtomicU32::new(1.0_f32.to_bits()),
            current_gain: AtomicU32::new(0.0_f32.to_bits()),
        })
    }

    fn set_target(&self, target: f32) {
        self.target_gain
            .store(target.clamp(0.0, 1.0).to_bits(), Ordering::Release);
    }

    fn gain(&self) -> f32 {
        f32::from_bits(self.current_gain.load(Ordering::Relaxed))
    }

    fn next_frame(&self, output_rate: u32) -> (f32, f32) {
        let output_rate = output_rate.max(1);
        let target = f32::from_bits(self.target_gain.load(Ordering::Acquire));
        let mut gain = self.gain();
        let step = 1.0 / (FADE_SECONDS * f64::from(output_rate)) as f32;
        if gain < target {
            gain = (gain + step).min(target);
        } else if gain > target {
            gain = (gain - step).max(target);
        }
        self.current_gain.store(gain.to_bits(), Ordering::Relaxed);
        if gain <= 1e-6 {
            return (0.0, 0.0);
        }

        let source_rate = f64::from(self.source.sample_rate());
        let span_frames = (((self.end_frame - self.start_frame) as f64 / source_rate)
            * f64::from(output_rate))
        .round()
        .max(1.0) as u64;
        let position = self.position.fetch_add(1, Ordering::Relaxed) % span_frames;
        let source_frame = self.start_frame
            + ((position as f64 / span_frames as f64) * (self.end_frame - self.start_frame) as f64)
                .floor() as u64;
        let Some((left, right)) = self.source.sample_frame(source_frame) else {
            return (0.0, 0.0);
        };
        let fade_frames = (FADE_SECONDS * f64::from(output_rate))
            .min(span_frames as f64 / 2.0)
            .max(1.0);
        let edge = (position.min(span_frames - position) as f64 / fade_frames).min(1.0) as f32;
        let envelope = edge * edge * (3.0 - 2.0 * edge);
        let level = gain * self.level * envelope;
        (left * level, right * level)
    }
}

/// Loopable preview of one edited note. Replacing the source crossfades the old
/// voice out while the new one fades in over the same five milliseconds.
#[derive(Debug)]
pub struct NoteAudition {
    current: ArcSwap<NoteLoop>,
    previous: ArcSwapOption<NoteLoop>,
    retired_previous: Mutex<Vec<Arc<NoteLoop>>>,
}

impl NoteAudition {
    pub fn new(
        source: PlaybackSource,
        source_offset_seconds: f64,
        start_seconds: f64,
        end_seconds: f64,
        level: f32,
    ) -> Result<Self, String> {
        Ok(Self {
            current: ArcSwap::from_pointee(NoteLoop::new(
                source,
                source_offset_seconds,
                start_seconds,
                end_seconds,
                level,
            )?),
            previous: ArcSwapOption::empty(),
            retired_previous: Mutex::new(Vec::new()),
        })
    }

    /// Replace the preview source without moving the loop position of the
    /// previous voice, so an update never steps the output.
    pub fn replace_with(&self, next: Self) {
        let next_current = next.current.load_full();
        let old = self.current.swap(next_current);
        if old.gain() > 1e-6 {
            old.set_target(0.0);
            if let Some(previous) = self.previous.swap(Some(old)) {
                self.retired_previous
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(previous);
            }
        } else {
            self.retired_previous
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(old);
        }
    }

    pub(crate) fn reclaim_retired(&self) {
        self.retired_previous
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    pub fn stop(&self) {
        self.current.load().set_target(0.0);
        if let Some(previous) = self.previous.load_full() {
            previous.set_target(0.0);
        }
    }

    pub(crate) fn next_frame(&self, output_rate: u32) -> (f32, f32) {
        let current = self.current.load_full();
        let (left, right) = current.next_frame(output_rate);
        let previous = self
            .previous
            .load_full()
            .map_or((0.0, 0.0), |voice| voice.next_frame(output_rate));
        (left + previous.0, right + previous.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DecodedWav;
    use std::sync::Arc;

    fn source(path: &std::path::Path) -> PlaybackSource {
        let mut writer = hound::WavWriter::create(
            path,
            hound::WavSpec {
                sample_rate: 16_000,
                channels: 1,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for frame in 0..16_000 {
            let value = (std::f32::consts::TAU * 220.0 * frame as f32 / 16_000.0).sin() * 0.5;
            writer.write_sample((value * 32767.0) as i16).unwrap();
        }
        writer.finalize().unwrap();
        PlaybackSource::Decoded(Arc::new(DecodedWav::open(path).unwrap()))
    }

    #[test]
    fn audition_voice_starts_fades_and_loops_without_transport() {
        let path =
            std::env::temp_dir().join(format!("opentrack-audition-{}.wav", std::process::id()));
        let audition = NoteAudition::new(source(&path), 0.0, 0.2, 0.4, 0.8).unwrap();
        let mut peak = 0.0_f32;
        for _ in 0..16_000 {
            let (left, right) = audition.next_frame(16_000);
            assert_eq!(left, right);
            peak = peak.max(left.abs());
        }
        assert!(peak > 0.1, "the preview must produce audible samples");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn replacing_the_preview_crossfades_without_a_sample_step() {
        let path = std::env::temp_dir().join(format!(
            "opentrack-audition-replace-{}.wav",
            std::process::id()
        ));
        let first = NoteAudition::new(source(&path), 0.0, 0.2, 0.4, 0.8).unwrap();
        for _ in 0..1_000 {
            first.next_frame(16_000);
        }
        let before = first.next_frame(16_000).0;
        let second = NoteAudition::new(source(&path), 0.0, 0.2, 0.4, 0.8).unwrap();
        first.replace_with(second);
        let after = first.next_frame(16_000).0;
        assert!(
            (after - before).abs() < 0.2,
            "preview replacement should blend rather than jump: {before} -> {after}"
        );
        let _ = std::fs::remove_file(path);
    }
}
