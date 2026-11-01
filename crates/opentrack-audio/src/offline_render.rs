//! Isolated, device-free rendering through the playback DSP graph.

use super::*;

/// Failure to prepare an offline render range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OfflineRenderError {
    InvalidTempo,
    InvalidSampleRate,
    InvalidRange,
    RangeTooLarge,
    MediaReader(String),
    MissingHostedEffects { required: usize, available: usize },
    HostedProcessingFailed,
}

impl std::fmt::Display for OfflineRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTempo => f.write_str("tempo must be finite and greater than zero"),
            Self::InvalidSampleRate => f.write_str("sample rate must be greater than zero"),
            Self::InvalidRange => f.write_str("render range must be finite and non-empty"),
            Self::RangeTooLarge => f.write_str("render range exceeds the supported frame limit"),
            Self::MediaReader(error) => {
                write!(f, "could not open an independent media reader: {error}")
            }
            Self::MissingHostedEffects {
                required,
                available,
            } => write!(
                f,
                "offline plug-in graph is incomplete ({available} of {required} hosted effects are ready)"
            ),
            Self::HostedProcessingFailed => {
                f.write_str("a hosted effect failed while rendering offline")
            }
        }
    }
}

impl std::error::Error for OfflineRenderError {}

/// Owns a separate playback graph and renders stereo samples without opening
/// an audio device. Callers provide fresh media readers and hosted effect
/// instances so rendering cannot change the live playhead or processor state.
pub struct OfflineRenderer {
    plan: ArcSwap<PlaybackPlan>,
    sample_rate: u32,
    start_frame: u64,
    end_frame: u64,
    rendered_frames: u64,
    tone: AtomicBool,
    playing: AtomicBool,
    playhead: AtomicU64,
    callback_generation: AtomicU64,
    callback_active: AtomicBool,
    input_monitor: Arc<ArcSwapOption<AudioMonitorBuffer>>,
    monitor_reader: AudioMonitorReader,
    phase: f32,
}

impl OfflineRenderer {
    /// Builds an independent DSP graph for the half-open beat range
    /// `start_beats..end_beats`. The supplied hosted effects must belong to an
    /// offline runtime, not the live audio engine's VST3 instances.
    pub fn new(
        tempo_bpm: f64,
        mut tracks: Vec<PlaybackTrackSettings>,
        sample_rate: u32,
        start_beats: f64,
        end_beats: f64,
        output_channel: &opentrack_core::project::Track,
        output_hosted_inserts: Vec<HostedInsert>,
    ) -> Result<Self, OfflineRenderError> {
        if !tempo_bpm.is_finite() || tempo_bpm <= 0.0 {
            return Err(OfflineRenderError::InvalidTempo);
        }
        if sample_rate == 0 {
            return Err(OfflineRenderError::InvalidSampleRate);
        }
        if !start_beats.is_finite()
            || start_beats < 0.0
            || !end_beats.is_finite()
            || end_beats <= start_beats
        {
            return Err(OfflineRenderError::InvalidRange);
        }
        let frames_per_beat = f64::from(sample_rate) * 60.0 / tempo_bpm;
        let start_value = (start_beats * frames_per_beat).round();
        let end_value = (end_beats * frames_per_beat).round();
        if !start_value.is_finite()
            || !end_value.is_finite()
            || start_value < 0.0
            || end_value <= start_value
            || end_value >= u64::MAX as f64
        {
            return Err(OfflineRenderError::RangeTooLarge);
        }
        let start_frame = start_value as u64;
        let end_frame = end_value as u64;

        let required_hosted: usize = tracks
            .iter()
            .map(|track| {
                track
                    .inserts
                    .iter()
                    .filter(|insert| {
                        insert.enabled
                            && !insert.bypassed
                            && !track.inserts_bypassed
                            && matches!(insert.kind, InsertEffectKind::Vst3 { .. })
                    })
                    .count()
            })
            .sum::<usize>()
            + if output_channel.inserts_bypassed {
                0
            } else {
                output_channel
                    .inserts
                    .iter()
                    .filter(|insert| {
                        insert.enabled
                            && !insert.bypassed
                            && matches!(insert.kind, InsertEffectKind::Vst3 { .. })
                    })
                    .count()
            };
        let available_hosted = tracks
            .iter()
            .flat_map(|track| &track.hosted_inserts)
            .chain(output_hosted_inserts.iter())
            .filter(|insert| insert.enabled)
            .count();
        if required_hosted != available_hosted {
            return Err(OfflineRenderError::MissingHostedEffects {
                required: required_hosted,
                available: available_hosted,
            });
        }

        // Each streamed clip needs its own disk reader; the live plan's reader
        // has a moving request cursor and must not be advanced by export.
        for track in &mut tracks {
            for clip in &mut track.clips {
                if let PlaybackSource::Streamed { source, .. } = &clip.source {
                    let source = Arc::clone(source);
                    let reader = source
                        .open_playback()
                        .map_err(OfflineRenderError::MediaReader)?;
                    clip.source = PlaybackSource::Streamed { source, reader };
                }
            }
        }

        // The export graph publishes no UI telemetry and never reads live input.
        for track in &mut tracks {
            track.meter_peak = Arc::new(StereoMeter::default());
            track.monitor_input = None;
        }
        let mut plan = PlaybackPlan::new_with_output_hosted(
            tempo_bpm,
            tracks,
            1,
            sample_rate,
            None,
            output_hosted_inserts,
        );
        plan.output_bus = bus::BusRuntime::new(output_channel, sample_rate);
        plan.cycle_mode = CycleMode::Loop;

        Ok(Self {
            plan: ArcSwap::from_pointee(plan),
            sample_rate,
            start_frame,
            end_frame,
            rendered_frames: 0,
            tone: AtomicBool::new(false),
            playing: AtomicBool::new(true),
            playhead: AtomicU64::new(start_frame),
            callback_generation: AtomicU64::new(0),
            callback_active: AtomicBool::new(false),
            input_monitor: Arc::new(ArcSwapOption::empty()),
            monitor_reader: AudioMonitorReader::default(),
            phase: 0.0,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn total_frames(&self) -> u64 {
        self.end_frame - self.start_frame
    }

    pub fn rendered_frames(&self) -> u64 {
        self.rendered_frames
    }

    /// Renders up to `output.len() / 2` frames into interleaved stereo `f32`
    /// samples. The slice length must be even. Returns zero after completion.
    /// Callers can check cancellation between bounded calls.
    pub fn render_next(&mut self, output: &mut [f32]) -> Result<usize, OfflineRenderError> {
        if output.len() % 2 != 0 {
            return Err(OfflineRenderError::InvalidRange);
        }
        let remaining = self.total_frames().saturating_sub(self.rendered_frames);
        if remaining == 0 {
            return Ok(0);
        }
        let frames = (output.len() / 2).min(usize::try_from(remaining).unwrap_or(usize::MAX));
        if frames == 0 {
            return Ok(0);
        }
        let sample_count = frames * 2;
        let plan = &self.plan;
        render_frames(
            &mut output[..sample_count],
            2,
            self.sample_rate,
            &self.tone,
            &self.playing,
            &self.playhead,
            plan,
            &self.callback_generation,
            &self.callback_active,
            &self.input_monitor,
            &mut self.monitor_reader,
            &mut self.phase,
            |sample, channel, left, right| {
                *sample = if channel == 0 { left } else { right };
            },
            None,
        );
        if self.plan.load().hosted_error_count.load(Ordering::Relaxed) > 0 {
            return Err(OfflineRenderError::HostedProcessingFailed);
        }
        self.rendered_frames = self.rendered_frames.saturating_add(frames as u64);
        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer(start: f64, end: f64) -> Result<OfflineRenderer, OfflineRenderError> {
        let settings = PlaybackTrackSettings {
            is_group: false,
            audio_channels: 2,
            clips: Vec::new(),
            midi_notes: vec![PlaybackMidiNote {
                id: 1,
                start_beats: 0.0,
                duration_beats: 2.0,
                frequency_hz: 440.0,
                channel: 0,
                velocity: 1.0,
            }],
            midi_controllers: Vec::new(),
            instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
            meter_peak: Arc::new(StereoMeter::default()),
            gain: 1.0,
            pan: 0.0,
            input_gain_db: 0.0,
            input_phase_inverted: false,
            track_delay_ms: 0.0,
            muted: false,
            solo: false,
            solo_defeat: false,
            equalizer: ChannelEqualizer::default(),
            inserts: Vec::new(),
            inserts_bypassed: false,
            hosted_inserts: Vec::new(),
            monitor_input: None,
            output_route: PlaybackOutputRoute::Stereo,
        };
        OfflineRenderer::new(
            120.0,
            vec![settings],
            48_000,
            start,
            end,
            &opentrack_core::project::BusChannel::output().strip,
            Vec::new(),
        )
    }

    #[test]
    fn renders_the_requested_frame_range_in_bounded_stereo_chunks() {
        let mut renderer = renderer(0.0, 1.0).unwrap();
        assert_eq!(renderer.total_frames(), 24_000);
        let mut output = vec![0.0; 512];
        assert_eq!(renderer.render_next(&mut output).unwrap(), 256);
        assert_eq!(renderer.rendered_frames(), 256);
        assert!(output.iter().any(|sample| sample.abs() > 1.0e-6));
        assert_eq!(renderer.render_next(&mut output).unwrap(), 256);
    }

    #[test]
    fn clamps_final_chunk_to_the_requested_range() {
        let mut renderer = renderer(0.0, 1.0 / 100.0).unwrap();
        let mut output = vec![0.0; 512];
        assert_eq!(renderer.render_next(&mut output).unwrap(), 240);
        assert_eq!(renderer.render_next(&mut output).unwrap(), 0);
        assert_eq!(renderer.rendered_frames(), 240);
    }

    #[test]
    fn rejects_empty_or_nonfinite_ranges() {
        assert!(matches!(
            renderer(1.0, 1.0),
            Err(OfflineRenderError::InvalidRange)
        ));
        assert!(matches!(
            renderer(0.0, f64::NAN),
            Err(OfflineRenderError::InvalidRange)
        ));
    }
}
