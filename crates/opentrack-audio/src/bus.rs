//! Device bus processing shared by capture and render. Construct DSP off the audio thread.
use super::*;
use opentrack_core::project::Track;
use std::cell::UnsafeCell;
use std::sync::Mutex;

pub(crate) struct BusRuntime {
    settings: Track,
    processor: UnsafeCell<ChannelProcessor>,
    pre_gain: f32,
    left_gain: f32,
    right_gain: f32,
}
// SAFETY: callers serialize process() with the capture gate or output callback gate.
unsafe impl Sync for BusRuntime {}
impl BusRuntime {
    pub(crate) fn new(settings: &Track, rate: u32) -> Self {
        Self {
            pre_gain: 10.0_f32.powf(settings.input_gain_db / 20.0)
                * if settings.input_phase_inverted {
                    -1.0
                } else {
                    1.0
                },
            left_gain: settings.gain * (1.0 - settings.pan.max(0.0)),
            right_gain: settings.gain * (1.0 + settings.pan.min(0.0)),
            settings: settings.clone(),
            processor: UnsafeCell::new(ChannelProcessor::new(
                &settings.equalizer,
                &settings.inserts,
                settings.inserts_bypassed,
                &[],
                &[],
                oscillator_instrument_factory(InstrumentWaveform::default()).as_ref(),
                120.0,
                rate,
            )),
        }
    }
    pub(crate) fn process(&self, left: f32, right: f32) -> (f32, f32) {
        self.process_inner(left, right, None)
    }
    pub(crate) fn process_unanalyzed(&self, left: f32, right: f32) -> (f32, f32) {
        self.process(left, right)
    }
    pub(crate) fn process_analyzed(
        &self,
        left: f32,
        right: f32,
        meter: &StereoMeter,
        rate: u32,
    ) -> (f32, f32) {
        self.process_inner(left, right, Some((meter, rate)))
    }
    fn process_inner(
        &self,
        mut left: f32,
        mut right: f32,
        analysis: Option<(&StereoMeter, u32)>,
    ) -> (f32, f32) {
        let s = &self.settings;
        if s.muted && !analysis.is_some_and(|(meter, _)| meter.spectrum_enabled()) {
            return (0.0, 0.0);
        }
        left *= self.pre_gain;
        right *= self.pre_gain;
        let pre = (left, right);
        // SAFETY: only the serialized audio callback accesses processor state.
        unsafe {
            (&mut *self.processor.get()).process(&mut left, &mut right);
        }
        if let Some((meter, rate)) = analysis {
            meter.record_spectrum(pre, (left, right), rate);
        }
        if s.muted {
            return (0.0, 0.0);
        }
        (left * self.left_gain, right * self.right_gain)
    }
}

pub(crate) struct InputBusProcessor {
    current: ArcSwap<BusRuntime>,
    retired: Mutex<Vec<Arc<BusRuntime>>>,
    active: AtomicBool,
    meter: Arc<StereoMeter>,
    rate: u32,
}
impl InputBusProcessor {
    pub(crate) fn new(rate: u32) -> Self {
        Self {
            current: ArcSwap::from_pointee(BusRuntime::new(
                &opentrack_core::project::BusChannel::input().strip,
                rate,
            )),
            retired: Mutex::new(Vec::new()),
            active: AtomicBool::new(false),
            meter: Arc::new(StereoMeter::default()),
            rate,
        }
    }
    pub(crate) fn set(&self, settings: &Track) {
        if self.current.load().settings == *settings {
            return;
        }
        let next = Arc::new(BusRuntime::new(settings, self.rate));
        // Control-thread-only retirement: the callback never frees DSP graphs or locks.
        let mut retired = self.retired.lock().unwrap_or_else(|e| e.into_inner());
        retired.push(self.current.swap(next));
        if !self.active.load(Ordering::SeqCst) {
            retired.clear();
        }
    }
    pub(crate) fn process(&self, left: f32, right: f32) -> (f32, f32) {
        if self
            .active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return (0.0, 0.0);
        }
        let state = self.current.load();
        let result = state.process_analyzed(left, right, &self.meter, self.rate);
        self.meter.publish([result.0.abs(), result.1.abs()]);
        drop(state);
        self.active.store(false, Ordering::SeqCst);
        result
    }
    pub(crate) fn meter_tap(&self) -> Arc<StereoMeter> {
        Arc::clone(&self.meter)
    }
    pub(crate) fn take_peaks(&self) -> [f32; 2] {
        self.meter.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_output_bus_scales_and_mutes_live_midi_and_test_tone_with_matching_peaks() {
        fn render(gain: f32, muted: bool, tone_enabled: bool) -> (Vec<f32>, [f32; 2]) {
            let mut settings = opentrack_core::project::BusChannel::output().strip;
            settings.gain = gain;
            settings.muted = muted;
            let mut plan = PlaybackPlan::empty();
            plan.output_bus = BusRuntime::new(&settings, 48000);
            let plan = ArcSwap::from_pointee(plan);
            let queue = Arc::new(LiveMidiQueue::new());
            assert!(queue.push(MidiInputMessage::NoteOn {
                channel: 0,
                key: 69,
                velocity: 100
            }));
            let mut midi = LiveMidiRenderer::new(
                queue,
                Arc::new(AtomicBool::new(true)),
                Arc::new(AtomicU8::new(0)),
                Arc::new(AtomicU32::new(1.0_f32.to_bits())),
            );
            let mut samples = vec![0.0; 512];
            let mut reader = AudioMonitorReader::default();
            let mut phase = 0.0;
            render_f32_with_midi(
                &mut samples,
                2,
                48000,
                &AtomicBool::new(tone_enabled),
                &AtomicBool::new(false),
                &AtomicU64::new(0),
                &plan,
                &AtomicU64::new(0),
                &AtomicBool::new(false),
                &Arc::new(ArcSwapOption::empty()),
                &mut reader,
                &mut phase,
                Some(&mut midi),
            );
            let peaks = plan.load().output_meter.take();
            (samples, peaks)
        }
        for tone in [false, true] {
            let (unity, peaks) = render(1.0, false, tone);
            assert!(peaks[0] > 0.001);
            let (half, half_peaks) = render(0.5, false, tone);
            for (a, b) in unity.iter().zip(half) {
                assert!((a * 0.5 - b).abs() < 0.00001);
            }
            assert!((peaks[0] * 0.5 - half_peaks[0]).abs() < 0.00001);
            let (silent, peaks) = render(1.0, true, tone);
            assert!(silent.iter().all(|s| *s == 0.0));
            assert_eq!(peaks, [0.0; 2]);
        }
    }
    #[test]
    fn bus_gain_balance_polarity_and_mute_change_samples_and_meters() {
        let bus = InputBusProcessor::new(48000);
        let mut settings = opentrack_core::project::BusChannel::input().strip;
        settings.gain = 0.5;
        settings.input_gain_db = 6.0206;
        settings.input_phase_inverted = true;
        settings.pan = -0.5;
        bus.set(&settings);
        let out = bus.process(0.25, -0.5);
        assert!((out.0 + 0.25).abs() < 0.0001 && (out.1 - 0.25).abs() < 0.0001);
        let peaks = bus.take_peaks();
        assert!((peaks[0] - 0.25).abs() < 0.0001 && (peaks[1] - 0.25).abs() < 0.0001);
        settings.muted = true;
        bus.set(&settings);
        assert_eq!(bus.process(1.0, 1.0), (0.0, 0.0));
        assert_eq!(bus.take_peaks(), [0.0; 2]);
    }
    #[test]
    fn analyzed_bus_reports_real_pre_post_eq_without_unmuting_audio() {
        let mut settings = opentrack_core::project::BusChannel::output().strip;
        settings.muted = true;
        settings.equalizer.bands[1].enabled = true;
        settings.equalizer.bands[1].frequency_hz = 1500.0;
        settings.equalizer.bands[1].gain_db = 6.0;
        let bus = BusRuntime::new(&settings, 48000);
        let meter = StereoMeter::default();
        meter.enable_spectrum(true);
        for i in 0..8192 {
            let sample = (i as f32 * std::f32::consts::TAU * 1500.0 / 48000.0).sin() * 0.25;
            assert_eq!(
                bus.process_analyzed(sample, sample, &meter, 48000),
                (0.0, 0.0)
            );
        }
        let pre = meter.spectrum_frame(SpectrumStage::Pre).unwrap();
        let post = meter.spectrum_frame(SpectrumStage::Post).unwrap();
        assert!((pre.bins_dbfs[64] + 12.0412).abs() < 0.05);
        assert!((post.bins_dbfs[64] - pre.bins_dbfs[64] - 6.0).abs() < 0.1);
    }
    #[test]
    fn bus_eq_and_insert_bypass_restore_unprocessed_samples() {
        let mut settings = opentrack_core::project::BusChannel::output().strip;
        settings.equalizer.bands[1].enabled = true;
        settings.equalizer.bands[1].frequency_hz = 1000.0;
        settings.equalizer.bands[1].gain_db = 6.0;
        let eq = BusRuntime::new(&settings, 48000);
        let mut input_energy = 0.0;
        let mut output_energy = 0.0;
        for i in 0..4800 {
            let sample = (i as f32 * std::f32::consts::TAU * 1000.0 / 48000.0).sin() * 0.1;
            let out = eq.process(sample, sample);
            if i > 2400 {
                input_energy += sample * sample;
                output_energy += out.0 * out.0;
            }
        }
        assert!(output_energy / input_energy > 3.7);
        settings.equalizer.bypassed = true;
        assert_eq!(
            BusRuntime::new(&settings, 48000).process(0.25, -0.5),
            (0.25, -0.5)
        );
        settings.inserts.push(InsertEffect {
            enabled: true,
            kind: InsertEffectKind::StereoDelay {
                time_ms: 10.0,
                feedback: 0.0,
                mix: 1.0,
            },
        });
        let delay = BusRuntime::new(&settings, 48000);
        assert_eq!(delay.process(1.0, -1.0), (0.0, 0.0));
        let mut delayed = (0.0, 0.0);
        for _ in 0..480 {
            delayed = delay.process(0.0, 0.0);
        }
        assert_eq!(delayed, (1.0, -1.0));
        settings.inserts_bypassed = true;
        assert_eq!(
            BusRuntime::new(&settings, 48000).process(0.25, -0.5),
            (0.25, -0.5)
        );
    }
}
