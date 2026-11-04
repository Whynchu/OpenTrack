//! Device bus processing shared by capture and render. Construct DSP off the audio thread.
use super::*;
use opentrack_core::project::Track;
use std::cell::UnsafeCell;
use std::sync::Mutex;

pub(crate) struct BusRuntime {
    settings: Track,
    processor: UnsafeCell<ChannelProcessor>,
    pre_gain: f32,
    mix: super::MixControl,
    equalizer_control: Arc<super::EqualizerControl>,
    automation_volume: Option<Arc<opentrack_core::project::AutomationLane>>,
    automation_pan: Option<Arc<opentrack_core::project::AutomationLane>>,
    automation_mute: Option<Arc<opentrack_core::project::AutomationLane>>,
    automation_volume_cursor: UnsafeCell<usize>,
    automation_pan_cursor: UnsafeCell<usize>,
    automation_mute_cursor: UnsafeCell<usize>,
    tempo_bpm: f64,
    sample_rate: u32,
    read_enabled: bool,
}
// SAFETY: callers serialize process() with the capture gate or output callback gate.
unsafe impl Sync for BusRuntime {}
impl BusRuntime {
    pub(crate) fn new(settings: &Track, rate: u32) -> Self {
        Self::new_at_tempo(settings, rate, 120.0, true)
    }
    pub(crate) fn new_at_tempo(settings: &Track, rate: u32, tempo_bpm: f64, read: bool) -> Self {
        let equalizer_control = super::EqualizerControl::new(settings.equalizer.clone());
        let target = |expected| {
            settings
                .automation_lanes
                .iter()
                .find(|lane| lane.target == expected && lane.is_valid() && !lane.points.is_empty())
                .cloned()
                .map(Arc::new)
        };
        let (automation_volume, automation_pan, automation_mute) = if read {
            if settings.id == opentrack_core::project::INPUT_BUS_ID {
                (
                    target(opentrack_core::automation::AutomationTarget::InputBusVolume),
                    target(opentrack_core::automation::AutomationTarget::InputBusPan),
                    target(opentrack_core::automation::AutomationTarget::InputBusMute),
                )
            } else {
                (
                    target(opentrack_core::automation::AutomationTarget::OutputBusVolume),
                    target(opentrack_core::automation::AutomationTarget::OutputBusPan),
                    target(opentrack_core::automation::AutomationTarget::OutputBusMute),
                )
            }
        } else {
            (None, None, None)
        };
        Self {
            pre_gain: 10.0_f32.powf(settings.input_gain_db / 20.0)
                * if settings.input_phase_inverted {
                    -1.0
                } else {
                    1.0
                },
            mix: super::MixControl::new(settings.gain, settings.pan, rate),
            settings: settings.clone(),
            processor: UnsafeCell::new(ChannelProcessor::new(
                &settings.equalizer,
                Arc::clone(&equalizer_control),
                &settings.inserts,
                settings.inserts_bypassed,
                &[],
                &[],
                std::sync::Arc::new(super::MidiEventControls::new(&[])),
                oscillator_instrument_factory(InstrumentWaveform::default()).as_ref(),
                120.0,
                rate,
            )),
            equalizer_control,
            automation_volume,
            automation_pan,
            automation_mute,
            automation_volume_cursor: UnsafeCell::new(0),
            automation_pan_cursor: UnsafeCell::new(0),
            automation_mute_cursor: UnsafeCell::new(0),
            tempo_bpm: if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
                tempo_bpm
            } else {
                120.0
            },
            sample_rate: rate.max(1),
            read_enabled: read,
        }
    }
    pub(crate) fn set_channel_parameters(
        &self,
        gain: f32,
        pan: f32,
        equalizer: &opentrack_core::project::ChannelEqualizer,
    ) {
        self.mix.set(gain, pan);
        self.equalizer_control.set(equalizer);
    }

    pub(crate) fn set_eq_solo_band(&self, band: Option<usize>) {
        self.equalizer_control.set_solo_band(band);
    }

    pub(crate) fn reclaim_retired(&self) {
        self.equalizer_control.reclaim_retired();
    }
    pub(crate) fn process(&self, left: f32, right: f32) -> (f32, f32) {
        self.process_inner(left, right, None, 0)
    }
    pub(crate) fn process_unanalyzed(&self, left: f32, right: f32) -> (f32, f32) {
        self.process(left, right)
    }
    pub(crate) fn process_unanalyzed_at(&self, left: f32, right: f32, frame: u64) -> (f32, f32) {
        self.process_inner(left, right, None, frame)
    }
    pub(crate) fn process_analyzed(
        &self,
        left: f32,
        right: f32,
        meter: &StereoMeter,
        rate: u32,
        frame: u64,
    ) -> (f32, f32) {
        self.process_inner(left, right, Some((meter, rate)), frame)
    }
    fn process_inner(
        &self,
        mut left: f32,
        mut right: f32,
        analysis: Option<(&StereoMeter, u32)>,
        frame: u64,
    ) -> (f32, f32) {
        let s = &self.settings;
        let beat = frame as f64 * self.tempo_bpm / (f64::from(self.sample_rate) * 60.0);
        let mute = self
            .automation_mute
            .as_ref()
            .and_then(|lane| {
                lane.value_at_cached(beat, unsafe { &mut *self.automation_mute_cursor.get() })
            })
            .map_or(s.muted, |value| value >= 0.5);
        if mute && !analysis.is_some_and(|(meter, _)| meter.spectrum_enabled()) {
            return (0.0, 0.0);
        }
        left *= self.pre_gain;
        right *= self.pre_gain;
        let pre = (left, right);
        // SAFETY: only the serialized audio callback accesses processor state.
        unsafe {
            let processor = &mut *self.processor.get();
            processor.process(&mut left, &mut right);
            if let Some((meter, _)) = analysis {
                processor.publish_dynamic_updates(meter);
            }
        }
        if let Some((meter, rate)) = analysis {
            meter.record_spectrum(pre, (left, right), rate);
        }
        if mute {
            return (0.0, 0.0);
        }
        let (base_gain, base_pan) = self.mix.next();
        let gain = self
            .automation_volume
            .as_ref()
            .and_then(|lane| {
                lane.value_at_cached(beat, unsafe { &mut *self.automation_volume_cursor.get() })
            })
            .unwrap_or(base_gain);
        let pan = self
            .automation_pan
            .as_ref()
            .and_then(|lane| {
                lane.value_at_cached(beat, unsafe { &mut *self.automation_pan_cursor.get() })
            })
            .unwrap_or(base_pan);
        (
            left * gain * (1.0 - pan.max(0.0)),
            right * gain * (1.0 + pan.min(0.0)),
        )
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
        Self::new_at_tempo(
            rate,
            120.0,
            &opentrack_core::project::BusChannel::input().strip,
            false,
        )
    }
    pub(crate) fn new_at_tempo(rate: u32, tempo_bpm: f64, settings: &Track, read: bool) -> Self {
        Self {
            current: ArcSwap::from_pointee(BusRuntime::new_at_tempo(
                settings, rate, tempo_bpm, read,
            )),
            retired: Mutex::new(Vec::new()),
            active: AtomicBool::new(false),
            meter: Arc::new(StereoMeter::default()),
            rate,
        }
    }
    pub(crate) fn set(&self, settings: &Track) {
        self.update_settings(settings, self.current.load().tempo_bpm, true);
    }
    pub(crate) fn set_at_tempo(&self, settings: &Track, tempo_bpm: f64, read: bool) {
        let next = Arc::new(BusRuntime::new_at_tempo(
            settings, self.rate, tempo_bpm, read,
        ));
        let mut retired = self.retired.lock().unwrap_or_else(|e| e.into_inner());
        retired.push(self.current.swap(next));
        if !self.active.load(Ordering::SeqCst) {
            retired.clear();
        }
    }
    pub(crate) fn update_settings(&self, settings: &Track, tempo_bpm: f64, read: bool) {
        if self.current.load().settings == *settings
            && self.current.load().tempo_bpm == tempo_bpm
            && self.current.load().read_enabled == read
        {
            return;
        }
        self.set_at_tempo(settings, tempo_bpm, read);
    }
    pub(crate) fn process_input(&self, left: f32, right: f32) -> (f32, f32) {
        self.process_at(left, right, 0)
    }
    pub(crate) fn process(&self, left: f32, right: f32) -> (f32, f32) {
        self.process_at(left, right, self.rate as u64)
    }
    pub(crate) fn process_at(&self, left: f32, right: f32, frame: u64) -> (f32, f32) {
        if self
            .active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return (0.0, 0.0);
        }
        let state = self.current.load();
        let result = state.process_analyzed(left, right, &self.meter, self.rate, frame);
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
    use opentrack_core::project::{AutomationLane, AutomationPoint, AutomationTarget};
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
    fn device_bus_read_applies_gain_pan_and_mute_on_project_clock() {
        let mut settings = opentrack_core::project::BusChannel::output().strip;
        settings.muted = false;
        settings.automation_lanes = vec![
            AutomationLane {
                target: AutomationTarget::OutputBusVolume,
                points: vec![AutomationPoint {
                    beat: 0.0,
                    value: 0.5,
                }],
            },
            AutomationLane {
                target: AutomationTarget::OutputBusPan,
                points: vec![AutomationPoint {
                    beat: 0.0,
                    value: 1.0,
                }],
            },
            AutomationLane {
                target: AutomationTarget::OutputBusMute,
                points: vec![
                    AutomationPoint {
                        beat: 0.0,
                        value: 0.0,
                    },
                    AutomationPoint {
                        beat: 1.0,
                        value: 1.0,
                    },
                ],
            },
        ];
        let read = BusRuntime::new_at_tempo(&settings, 48_000, 120.0, true);
        assert_eq!(read.process_unanalyzed_at(0.4, 0.4, 0), (0.0, 0.2));
        assert_eq!(read.process_unanalyzed_at(0.4, 0.4, 24_000), (0.0, 0.0));
        let base = BusRuntime::new_at_tempo(&settings, 48_000, 120.0, false);
        assert_eq!(base.process_unanalyzed_at(0.4, 0.4, 0), (0.4, 0.4));
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
                bus.process_analyzed(sample, sample, &meter, 48000, i as u64),
                (0.0, 0.0)
            );
        }
        let pre = meter.spectrum_frame(SpectrumStage::Pre).unwrap();
        let post = meter.spectrum_frame(SpectrumStage::Post).unwrap();
        // Balanced analysis now uses a 4096-sample window: 1500 Hz is bin 128.
        assert!((pre.bins_dbfs[128] + 12.0412).abs() < 0.05);
        assert!((post.bins_dbfs[128] - pre.bins_dbfs[128] - 6.0).abs() < 0.1);
    }

    #[test]
    fn analyzed_bus_publishes_dynamic_eq_movement_to_its_meter() {
        let mut settings = opentrack_core::project::BusChannel::output().strip;
        settings.equalizer.bands[1].enabled = true;
        settings.equalizer.bands[1].frequency_hz = 1_000.0;
        settings.equalizer.dynamic_bands[1] = opentrack_core::project::DynamicEqBand {
            enabled: true,
            threshold_db: -36.0,
            range_db: -6.0,
            attack_ms: 1.0,
            release_ms: 100.0,
        };
        let bus = BusRuntime::new(&settings, 48_000);
        let meter = StereoMeter::default();
        for index in 0..24_000 {
            let sample = (std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0).sin() * 0.5;
            bus.process_analyzed(sample, sample, &meter, 48_000, index as u64);
        }
        assert!(meter.dynamic_eq_gains_db()[1] < -4.0);
    }

    #[test]
    fn output_bus_eq_solo_auditions_the_32nd_band() {
        fn render(input_frequency: f32) -> f64 {
            let mut settings = opentrack_core::project::BusChannel::output().strip;
            let band = settings.equalizer.band_mut(31).unwrap();
            band.frequency_hz = 8_000.0;
            band.q = 1.0;
            let bus = BusRuntime::new(&settings, 48_000);
            bus.set_eq_solo_band(Some(31));
            let mut energy = 0.0_f64;
            for index in 0..48_000 {
                let sample =
                    (std::f32::consts::TAU * input_frequency * index as f32 / 48_000.0).sin() * 0.5;
                let output = bus.process(sample, sample);
                if index >= 24_000 {
                    energy += f64::from(output.0 * output.0);
                }
            }
            energy
        }

        let center_energy = render(8_000.0);
        let off_band_energy = render(100.0);
        assert!(
            center_energy > off_band_energy * 20.0,
            "Stereo Out band 32 solo isolation ratio was {:.1} dB",
            10.0 * (center_energy / off_band_energy).log10()
        );
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
            id: 0,
            enabled: true,
            bypassed: false,
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
