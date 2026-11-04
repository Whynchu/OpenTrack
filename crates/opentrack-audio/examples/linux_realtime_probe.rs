//! Run a muted CPAL callback-load probe against a local WAV file.
//!
//! Build with `cargo build --locked --offline -p opentrack-audio --example
//! linux_realtime_probe --features linux-realtime`, then run with
//! `OPENTRACK_REALTIME_DIAGNOSTICS=1 target/debug/examples/linux_realtime_probe
//! DEVICE_ID WAV_PATH [TRACKS] [SECONDS]`. Every generated track reads the same
//! WAV through 12 EQ bands. Stereo Out gain is zero so this probe does not send
//! audio to the selected device.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use opentrack_audio::{
    AudioEngine, DecodedWav, PlaybackClip, PlaybackOutputRoute, PlaybackSource, StereoMeter,
    oscillator_instrument_factory,
};
use opentrack_core::project::{BusChannel, ChannelEqualizer, CycleMode, InstrumentWaveform};

fn track_settings(source: Arc<DecodedWav>, index: usize) -> opentrack_audio::PlaybackTrackSettings {
    let mut equalizer = ChannelEqualizer::default();
    for band_index in 0..12 {
        if let Some(band) = equalizer.band_mut(band_index) {
            band.enabled = true;
            band.frequency_hz = (80.0 * 2.0_f64.powf(band_index as f64 / 2.0)) as f32;
            band.q = 0.7 + index as f32 * 0.01;
        }
    }

    opentrack_audio::PlaybackTrackSettings {
        is_group: false,
        audio_channels: 2,
        clips: vec![PlaybackClip {
            source_fades: Vec::new(),
            source: PlaybackSource::Decoded(Arc::clone(&source)),
            start_beats: 0.0,
            source_offset_seconds: 0.0,
            duration_seconds: source.duration_seconds(),
            fade_in_seconds: 0.0,
            fade_out_seconds: 0.0,
            gain: 0.5,
            muted: false,
            solo: false,
        }],
        midi_notes: Vec::new(),
        midi_controllers: Vec::new(),
        instrument_factory: oscillator_instrument_factory(InstrumentWaveform::Sine),
        meter_peak: Arc::new(StereoMeter::default()),
        gain: 0.8,
        pan: 0.0,
        input_gain_db: 0.0,
        input_phase_inverted: false,
        track_delay_ms: 0.0,
        muted: false,
        solo: false,
        solo_defeat: false,
        equalizer,
        inserts: Vec::new(),
        inserts_bypassed: false,
        hosted_inserts: Vec::new(),
        hosted_instrument: None,
        instrument_output: None,
        monitor_input: None,
        output_route: PlaybackOutputRoute::Stereo,
        volume_automation: None,
        pan_automation: None,
        mute_automation: None,
        sends: Vec::new(),
    }
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let device_id = args
        .next()
        .ok_or("usage: linux_realtime_probe DEVICE_ID WAV_PATH [TRACKS] [SECONDS]")?;
    let wav_path = args
        .next()
        .ok_or("usage: linux_realtime_probe DEVICE_ID WAV_PATH [TRACKS] [SECONDS]")?;
    let tracks = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| String::from("TRACKS must be a positive integer"))?
        .unwrap_or(1)
        .clamp(1, 32);
    let seconds = args
        .next()
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| String::from("SECONDS must be a positive integer"))?
        .unwrap_or(10)
        .clamp(1, 120);
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }

    let source = Arc::new(DecodedWav::open(Path::new(&wav_path))?);
    let mut engine = AudioEngine::open_output_device(Some(&device_id), Some(48_000), Some(512))?;
    let settings = (0..tracks)
        .map(|index| track_settings(Arc::clone(&source), index))
        .collect();
    let mut output = BusChannel::output().strip;
    output.gain = 0.0;
    engine.set_playback_plan_with_output(120.0, settings, None, CycleMode::Loop, &output);
    engine.set_playing(true);
    std::thread::sleep(Duration::from_secs(seconds));

    println!(
        "OpenTrack muted callback probe: device={} id={} rate={} buffer={} tracks={} source_seconds={:.2} duration_seconds={} policy={} denials={} xruns={} callback={:?} timing={:?}",
        engine.device_name,
        engine.device_id,
        engine.sample_rate,
        engine.output_buffer_frames,
        tracks,
        source.duration_seconds(),
        seconds,
        engine.output_thread_scheduling(),
        engine.realtime_priority_denial_count(),
        engine.output_xrun_count(),
        engine.callback_budget(),
        engine.callback_timing_summary(),
    );
    engine.set_playing(false);
    Ok(())
}
