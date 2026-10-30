//! Windows render session using event-driven WASAPI.
//!
//! COM clients are created, used and released on the owning worker thread.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use std::str::FromStr;

use wasapi::{
    AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode, TimingMode, WaveFormat,
};
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
    AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR, IAudioCaptureClient,
};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Threading::{
    AVRT_PRIORITY_HIGH, AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW,
    AvSetMmThreadPriority,
};

use crate::{AudioDeviceMode, CallbackTimingTrace, StageTimingSummary, StageTimingTrace};

/// Audio thread timing metadata from the Windows capture engine.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CapturePacketInfo {
    pub device_position: u64,
    pub timestamp_hns: u64,
    pub discontinuity: bool,
    pub timestamp_error: bool,
}

#[derive(Clone)]
pub(crate) struct WasapiInputInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: u32,
    pub period_frames: u32,
    pub mode: AudioDeviceMode,
    pub detail: String,
}

pub(crate) struct WasapiInputStream {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    timing: Option<Arc<StageTimingTrace>>,
    pub info: WasapiInputInfo,
}

impl WasapiInputStream {
    pub fn open(
        endpoint_id: String,
        requested_mode: AudioDeviceMode,
        requested_rate: Option<u32>,
        requested_period_frames: Option<u32>,
        failed: Arc<AtomicBool>,
        consume: impl FnMut(&[f32], u16, CapturePacketInfo) + Send + 'static,
    ) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_failed = Arc::clone(&failed);
        let timing = StageTimingTrace::new().map(Arc::new);
        let worker_timing = timing.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("opentrack-wasapi-capture".to_owned())
            .spawn(move || {
                if let Err(error) = capture_worker(
                    &endpoint_id,
                    requested_mode,
                    requested_rate,
                    requested_period_frames,
                    worker_stop,
                    worker_failed.clone(),
                    worker_timing,
                    consume,
                    ready_tx.clone(),
                ) {
                    worker_failed.store(true, Ordering::Release);
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|error| format!("could not start the WASAPI capture thread: {error}"))?;
        match ready_rx.recv() {
            Ok(Ok(info)) => Ok(Self {
                stop,
                worker: Some(worker),
                timing,
                info,
            }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                let _ = worker.join();
                Err(format!(
                    "WASAPI capture thread failed during startup: {error}"
                ))
            }
        }
    }

    pub fn timing_summary(&self) -> Option<StageTimingSummary> {
        self.timing.as_ref()?.summary()
    }
}

impl Drop for WasapiInputStream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct ComMta;

impl ComMta {
    fn initialize() -> Result<Self, String> {
        wasapi::initialize_mta()
            .ok()
            .map(|()| Self)
            .map_err(|error| format!("could not initialize WASAPI COM: {error:?}"))
    }
}

impl Drop for ComMta {
    fn drop(&mut self) {
        wasapi::deinitialize();
    }
}

struct MmcssTask(HANDLE);

impl MmcssTask {
    fn enter() -> Option<Self> {
        let name: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
        let mut index = 0;
        // SAFETY: task name is nul-terminated UTF-16 and index stays valid for this call.
        let handle = unsafe { AvSetMmThreadCharacteristicsW(name.as_ptr(), &mut index) };
        if handle.is_null() {
            return None;
        }
        // SAFETY: handle was returned by AvSetMmThreadCharacteristicsW.
        let _ = unsafe { AvSetMmThreadPriority(handle, AVRT_PRIORITY_HIGH) };
        Some(Self(handle))
    }
}

impl Drop for MmcssTask {
    fn drop(&mut self) {
        // SAFETY: this handle was returned by AvSetMmThreadCharacteristicsW.
        let _ = unsafe { AvRevertMmThreadCharacteristics(self.0) };
    }
}

pub(crate) fn endpoint_id_from_cpal(device_id: &str) -> Result<String, String> {
    let id = cpal::DeviceId::from_str(device_id)
        .map_err(|error| format!("invalid audio endpoint identifier: {error}"))?;
    let host_name = id.host().name();
    if !host_name.eq_ignore_ascii_case("WASAPI") {
        return Err(format!(
            "{} is not a WASAPI endpoint; native Windows audio modes require WASAPI",
            host_name
        ));
    }
    Ok(id.id().to_owned())
}

pub(crate) fn default_endpoint_id(direction: Direction) -> Result<String, String> {
    thread::Builder::new()
        .name("opentrack-wasapi-default-query".to_owned())
        .spawn(move || {
            let _com = ComMta::initialize()?;
            let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
            let device = enumerator.get_default_device(&direction).map_err(|error| {
                format!("could not find the default WASAPI {direction} endpoint: {error}")
            })?;
            device
                .get_id()
                .map_err(|error| format!("could not read the default WASAPI endpoint ID: {error}"))
        })
        .map_err(|error| format!("could not start the WASAPI default query: {error}"))?
        .join()
        .map_err(|_| "WASAPI default query stopped unexpectedly".to_owned())?
}

pub(crate) fn endpoint_friendly_name(endpoint_id: String) -> Result<String, String> {
    thread::Builder::new()
        .name("opentrack-wasapi-name-query".to_owned())
        .spawn(move || {
            let _com = ComMta::initialize()?;
            let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
            let device = enumerator
                .get_device(&endpoint_id)
                .map_err(|error| format!("could not find WASAPI endpoint: {error}"))?;
            device
                .get_friendlyname()
                .map_err(|error| format!("could not read the WASAPI endpoint name: {error}"))
        })
        .map_err(|error| format!("could not start the WASAPI name query: {error}"))?
        .join()
        .map_err(|_| "WASAPI name query stopped unexpectedly".to_owned())?
}

pub(crate) fn enumerate_active_endpoints()
-> Result<(Vec<(String, String, bool, bool)>, Vec<String>), String> {
    thread::Builder::new()
        .name("opentrack-wasapi-enumeration".to_owned())
        .spawn(|| {
            let _com = ComMta::initialize()?;
            let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
            let mut endpoints: Vec<(String, String, bool, bool)> = Vec::new();
            let mut diagnostics = Vec::new();
            for (direction, input) in [(Direction::Capture, true), (Direction::Render, false)] {
                let collection = match enumerator.get_device_collection(&direction) {
                    Ok(collection) => collection,
                    Err(error) => {
                        diagnostics.push(format!("WASAPI {direction} endpoints: {error}"));
                        continue;
                    }
                };
                let count = match collection.get_nbr_devices() {
                    Ok(count) => count,
                    Err(error) => {
                        diagnostics.push(format!("WASAPI {direction} endpoint count: {error}"));
                        continue;
                    }
                };
                for index in 0..count {
                    let device = match collection.get_device_at_index(index) {
                        Ok(device) => device,
                        Err(error) => {
                            diagnostics
                                .push(format!("WASAPI {direction} endpoint {index}: {error}"));
                            continue;
                        }
                    };
                    let id = match device.get_id() {
                        Ok(id) => id,
                        Err(error) => {
                            diagnostics
                                .push(format!("WASAPI {direction} endpoint {index} ID: {error}"));
                            continue;
                        }
                    };
                    let name = device.get_friendlyname().unwrap_or_else(|error| {
                        diagnostics.push(format!("WASAPI endpoint {id} name: {error}"));
                        id.clone()
                    });
                    if let Some(existing) = endpoints.iter_mut().find(|entry| entry.0 == id) {
                        existing.2 |= input;
                        existing.3 |= !input;
                    } else {
                        endpoints.push((id, name, input, !input));
                    }
                }
            }
            endpoints
                .sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
            Ok((endpoints, diagnostics))
        })
        .map_err(|error| format!("could not start WASAPI endpoint enumeration: {error}"))?
        .join()
        .map_err(|_| "WASAPI endpoint enumeration stopped unexpectedly".to_owned())?
}

pub(crate) fn query_input_mix_format(endpoint_id: String) -> Result<(u32, u16), String> {
    thread::Builder::new()
        .name("opentrack-wasapi-format-query".to_owned())
        .spawn(move || {
            let _com = ComMta::initialize()?;
            let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
            let device = enumerator
                .get_device(&endpoint_id)
                .map_err(|error| format!("could not find WASAPI input endpoint: {error}"))?;
            let client = device
                .get_iaudioclient()
                .map_err(|error| format!("could not activate WASAPI input endpoint: {error}"))?;
            let format = client
                .get_mixformat()
                .map_err(|error| format!("could not read the WASAPI input mix format: {error}"))?;
            Ok((format.get_samplespersec(), format.get_nchannels()))
        })
        .map_err(|error| format!("could not start the WASAPI format query: {error}"))?
        .join()
        .map_err(|_| "WASAPI format query stopped unexpectedly".to_owned())?
}

#[derive(Clone)]
pub(crate) struct WasapiOutputInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: u32,
    pub period_frames: u32,
    pub period_choices: Vec<u32>,
    pub mode: AudioDeviceMode,
    pub detail: String,
}

pub(crate) struct WasapiOutputStream {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    pub info: WasapiOutputInfo,
}

impl WasapiOutputStream {
    pub fn open(
        endpoint_id: String,
        requested_mode: AudioDeviceMode,
        requested_rate: Option<u32>,
        requested_period_frames: Option<u32>,
        failed: Arc<AtomicBool>,
        callback_timing: Arc<CallbackTimingTrace>,
        make_renderer: impl FnOnce(u32, u16) -> Box<dyn FnMut(&mut [f32]) + Send> + Send + 'static,
    ) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_failed = Arc::clone(&failed);
        let worker_timing = Arc::clone(&callback_timing);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("opentrack-wasapi-render".to_owned())
            .spawn(move || {
                if let Err(error) = render_worker(
                    &endpoint_id,
                    requested_mode,
                    requested_rate,
                    requested_period_frames,
                    worker_stop,
                    worker_failed.clone(),
                    worker_timing,
                    make_renderer,
                    ready_tx.clone(),
                ) {
                    worker_failed.store(true, Ordering::Release);
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|error| format!("could not start the WASAPI render thread: {error}"))?;
        match ready_rx.recv() {
            Ok(Ok(info)) => Ok(Self {
                stop,
                worker: Some(worker),
                info,
            }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                let _ = worker.join();
                Err(format!(
                    "WASAPI render thread failed during startup: {error}"
                ))
            }
        }
    }
}

impl Drop for WasapiOutputStream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_worker(
    endpoint_id: &str,
    requested_mode: AudioDeviceMode,
    requested_rate: Option<u32>,
    requested_period_frames: Option<u32>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    callback_timing: Arc<CallbackTimingTrace>,
    make_renderer: impl FnOnce(u32, u16) -> Box<dyn FnMut(&mut [f32]) + Send>,
    ready: mpsc::SyncSender<Result<WasapiOutputInfo, String>>,
) -> Result<(), String> {
    let _com = ComMta::initialize()?;
    let _mmcss = MmcssTask::enter();
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let device = enumerator
        .get_device(endpoint_id)
        .map_err(|error| format!("could not find WASAPI endpoint: {error}"))?;
    let device_name = device
        .get_friendlyname()
        .unwrap_or_else(|_| "WASAPI output".to_owned());
    let mut client = device
        .get_iaudioclient()
        .map_err(|error| format!("could not activate WASAPI output: {error}"))?;
    let mix_format = client
        .get_mixformat()
        .map_err(|error| format!("could not read Windows output mix format: {error}"))?;
    if requested_mode == AudioDeviceMode::SharedLowLatency
        && requested_rate.is_some_and(|rate| rate != mix_format.get_samplespersec())
    {
        return Err(format!(
            "shared low latency uses the Windows mix rate ({} Hz); clear the requested rate or choose shared compatible mode",
            mix_format.get_samplespersec()
        ));
    }
    let (format, mode, period_frames, detail) = match requested_mode {
        AudioDeviceMode::SharedLowLatency => {
            match client.get_shared_mode_engine_period(&mix_format) {
                Ok(periods) => {
                    let requested = nearest_engine_period(
                        requested_period_frames.unwrap_or(periods.min),
                        periods,
                    );
                    if requested < periods.min || requested > periods.max {
                        return Err(
                            "Windows reported an invalid shared audio period range".to_owned()
                        );
                    }
                    match client.initialize_shared_audio_stream(
                        &mix_format,
                        TimingMode::Events,
                        requested,
                    ) {
                        Ok(()) => (
                            mix_format,
                            AudioDeviceMode::SharedLowLatency,
                            requested,
                            format!(
                                "Shared low latency · {} frame period · min {} / default {} frames",
                                requested, periods.min, periods.default
                            ),
                        ),
                        Err(low_latency_error) => {
                            client = device
                                .get_iaudioclient()
                                .map_err(|error| error.to_string())?;
                            let (_, minimum) = client
                                .get_device_period()
                                .map_err(|error| error.to_string())?;
                            client.initialize_client(
                                &mix_format,
                                &Direction::Render,
                                &StreamMode::EventsShared {
                                    autoconvert: false,
                                    buffer_duration_hns: minimum,
                                },
                            ).map_err(|fallback_error| format!("low-period shared mode failed ({low_latency_error}); standard shared fallback also failed ({fallback_error})"))?;
                            (
                                mix_format,
                                AudioDeviceMode::SharedCompatible,
                                0,
                                format!(
                                    "Shared compatible fallback · low-period initialization failed: {low_latency_error}"
                                ),
                            )
                        }
                    }
                }
                Err(low_latency_error) => {
                    let (_, minimum) = client
                        .get_device_period()
                        .map_err(|error| error.to_string())?;
                    client.initialize_client(
                        &mix_format,
                        &Direction::Render,
                        &StreamMode::EventsShared {
                            autoconvert: false,
                            buffer_duration_hns: minimum,
                        },
                    ).map_err(|fallback_error| format!("IAudioClient3 low-period shared mode unavailable ({low_latency_error}); standard shared fallback failed ({fallback_error})"))?;
                    (
                        mix_format,
                        AudioDeviceMode::SharedCompatible,
                        0,
                        format!(
                            "Shared compatible fallback · IAudioClient3 unavailable: {low_latency_error}"
                        ),
                    )
                }
            }
        }
        AudioDeviceMode::SharedCompatible => {
            let (default_period, minimum) = client
                .get_device_period()
                .map_err(|error| error.to_string())?;
            let channels = usize::from(mix_format.get_nchannels().clamp(1, 2));
            let format = requested_rate
                .filter(|rate| *rate > 0 && *rate != mix_format.get_samplespersec())
                .map(|rate| {
                    WaveFormat::new(32, 32, &SampleType::Float, rate as usize, channels, None)
                })
                .unwrap_or_else(|| mix_format.clone());
            let buffer_duration_hns = requested_period_frames
                .filter(|frames| *frames > 0)
                .map(|frames| {
                    i64::from(frames) * 10_000_000 / i64::from(format.get_samplespersec())
                })
                .unwrap_or(default_period)
                .max(minimum);
            client
                .initialize_client(
                    &format,
                    &Direction::Render,
                    &StreamMode::EventsShared {
                        autoconvert: true,
                        buffer_duration_hns,
                    },
                )
                .map_err(|error| format!("could not initialize shared WASAPI output: {error}"))?;
            (
                format,
                AudioDeviceMode::SharedCompatible,
                0,
                format!(
                    "Shared compatible · {} 100 ns buffer duration",
                    buffer_duration_hns
                ),
            )
        }
        AudioDeviceMode::Exclusive => {
            let rate = requested_rate.unwrap_or_else(|| mix_format.get_samplespersec());
            let format = choose_exclusive_format(&client, &mix_format, rate)?;
            let (_, minimum) = client
                .get_device_period()
                .map_err(|error| error.to_string())?;
            let requested_hns = requested_period_frames
                .filter(|frames| *frames > 0)
                .map(|frames| {
                    i64::from(frames) * 10_000_000 / i64::from(format.get_samplespersec())
                })
                .unwrap_or(minimum);
            let period = client
                .calculate_aligned_period_near(requested_hns, None, &format)
                .map_err(|error| format!("could not choose an exclusive WASAPI period: {error}"))?;
            client
                .initialize_client(
                    &format,
                    &Direction::Render,
                    &StreamMode::EventsExclusive { period_hns: period },
                )
                .map_err(|error| {
                    format!("could not initialize exclusive WASAPI output: {error}")
                })?;
            let frames = (period * i64::from(format.get_samplespersec()) / 10_000_000)
                .clamp(1, i64::from(u32::MAX)) as u32;
            (
                format,
                AudioDeviceMode::Exclusive,
                frames,
                "Exclusive · event driven".to_owned(),
            )
        }
    };

    let channels = format.get_nchannels();
    let sample_rate = format.get_samplespersec();
    let block_align = format.get_blockalign() as usize;
    if channels == 0 || sample_rate == 0 || block_align == 0 {
        return Err("WASAPI endpoint reported an invalid output format".to_owned());
    }
    let sample_bytes = block_align / usize::from(channels);
    if sample_bytes == 0 || sample_bytes * usize::from(channels) != block_align {
        return Err("WASAPI endpoint reported an unsupported channel layout".to_owned());
    }
    let buffer_frames = client
        .get_buffer_size()
        .map_err(|error| format!("could not read WASAPI output buffer size: {error}"))?;
    let event = client
        .set_get_eventhandle()
        .map_err(|error| format!("could not create the WASAPI output event: {error}"))?;
    let render_client = client
        .get_audiorenderclient()
        .map_err(|error| format!("could not create the WASAPI render client: {error}"))?;
    let mut renderer = make_renderer(sample_rate, channels);
    let mut samples = vec![0.0_f32; buffer_frames as usize * usize::from(channels)];
    let mut bytes = vec![0_u8; buffer_frames as usize * block_align];

    let fill = |frames: usize,
                samples: &mut [f32],
                bytes: &mut [u8],
                renderer: &mut Box<dyn FnMut(&mut [f32]) + Send>|
     -> Result<(), String> {
        let sample_count = frames
            .checked_mul(usize::from(channels))
            .ok_or_else(|| "WASAPI frame count overflow".to_owned())?;
        let byte_count = frames
            .checked_mul(block_align)
            .ok_or_else(|| "WASAPI buffer size overflow".to_owned())?;
        if sample_count > samples.len() || byte_count > bytes.len() {
            return Err("WASAPI requested more frames than its endpoint buffer".to_owned());
        }
        let samples = &mut samples[..sample_count];
        renderer(samples);
        encode_samples(&format, sample_bytes, samples, &mut bytes[..byte_count])
    };

    fill(
        buffer_frames as usize,
        &mut samples,
        &mut bytes,
        &mut renderer,
    )?;
    render_client
        .write_to_device(buffer_frames as usize, &bytes, None)
        .map_err(|error| format!("could not prime the WASAPI output buffer: {error}"))?;
    client
        .start_stream()
        .map_err(|error| format!("could not start WASAPI output: {error}"))?;
    let actual_period = if period_frames == 0 {
        client
            .get_current_shared_mode_engine_period()
            .ok()
            .map(|(_, value)| value)
            .unwrap_or(buffer_frames)
    } else {
        period_frames
    };
    let period_choices = if mode == AudioDeviceMode::Exclusive {
        Vec::new()
    } else {
        client
            .get_shared_mode_engine_period(&format)
            .ok()
            .map(shared_period_choices)
            .unwrap_or_default()
    };
    let info = WasapiOutputInfo {
        device_name,
        sample_rate,
        channels,
        buffer_frames,
        period_frames: actual_period,
        period_choices,
        mode,
        detail,
    };
    if ready.send(Ok(info)).is_err() {
        stop.store(true, Ordering::Release);
    }

    let mut last_callback_entry = None;
    while !stop.load(Ordering::Acquire) {
        if let Err(error) = event.wait_for_event(100) {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let _ = error;
            failed.store(true, Ordering::Release);
            break;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        let callback_started = callback_timing.enabled().then(Instant::now);
        let callback_interval = callback_started
            .zip(last_callback_entry)
            .map(|(now, previous)| now.duration_since(previous));
        if let Some(now) = callback_started {
            last_callback_entry = Some(now);
        }
        let available = match client.get_available_space_in_frames() {
            Ok(frames) => frames,
            Err(_) => {
                failed.store(true, Ordering::Release);
                break;
            }
        };
        if available == 0 {
            if let Some(started) = callback_started {
                callback_timing.record(callback_interval, started.elapsed());
            }
            continue;
        }
        if fill(available as usize, &mut samples, &mut bytes, &mut renderer).is_err()
            || render_client
                .write_to_device(
                    available as usize,
                    &bytes[..available as usize * block_align],
                    None,
                )
                .is_err()
        {
            failed.store(true, Ordering::Release);
            break;
        }
        if let Some(started) = callback_started {
            callback_timing.record(callback_interval, started.elapsed());
        }
    }
    let _ = client.stop_stream();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn capture_worker(
    endpoint_id: &str,
    requested_mode: AudioDeviceMode,
    requested_rate: Option<u32>,
    requested_period_frames: Option<u32>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    timing: Option<Arc<StageTimingTrace>>,
    mut consume: impl FnMut(&[f32], u16, CapturePacketInfo),
    ready: mpsc::SyncSender<Result<WasapiInputInfo, String>>,
) -> Result<(), String> {
    let _com = ComMta::initialize()?;
    let _mmcss = MmcssTask::enter();
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let device = enumerator
        .get_device(endpoint_id)
        .map_err(|error| format!("could not find WASAPI capture endpoint: {error}"))?;
    let device_name = device
        .get_friendlyname()
        .unwrap_or_else(|_| "WASAPI input".to_owned());
    let mut client = device
        .get_iaudioclient()
        .map_err(|error| format!("could not activate WASAPI capture: {error}"))?;
    let mix_format = client
        .get_mixformat()
        .map_err(|error| format!("could not read Windows input mix format: {error}"))?;
    let (format, mode, period_frames, detail) = match requested_mode {
        AudioDeviceMode::SharedLowLatency => {
            match client.get_shared_mode_engine_period(&mix_format) {
                Ok(periods) => {
                    let requested = ceil_to_multiple(periods.min, periods.fundamental)
                        .filter(|value| *value <= periods.max)
                        .ok_or_else(|| {
                            "Windows reported an invalid shared capture period range".to_owned()
                        })?;
                    match client.initialize_shared_audio_stream(
                        &mix_format,
                        TimingMode::Events,
                        requested,
                    ) {
                        Ok(()) => (
                            mix_format,
                            AudioDeviceMode::SharedLowLatency,
                            requested,
                            format!(
                                "Shared low latency · min {} / default {} frames",
                                periods.min, periods.default
                            ),
                        ),
                        Err(low_latency_error) => {
                            client = device
                                .get_iaudioclient()
                                .map_err(|error| error.to_string())?;
                            let (_, minimum) = client
                                .get_device_period()
                                .map_err(|error| error.to_string())?;
                            client
                            .initialize_client(
                                &mix_format,
                                &Direction::Capture,
                                &StreamMode::EventsShared {
                                    autoconvert: false,
                                    buffer_duration_hns: minimum,
                                },
                            )
                            .map_err(|fallback_error| {
                                format!(
                                    "low-period capture failed ({low_latency_error}); standard shared fallback also failed ({fallback_error})"
                                )
                            })?;
                            (
                                mix_format,
                                AudioDeviceMode::SharedCompatible,
                                0,
                                format!(
                                    "Shared compatible fallback · low-period initialization failed: {low_latency_error}"
                                ),
                            )
                        }
                    }
                }
                Err(low_latency_error) => {
                    let (_, minimum) = client
                        .get_device_period()
                        .map_err(|error| error.to_string())?;
                    client
                    .initialize_client(
                        &mix_format,
                        &Direction::Capture,
                        &StreamMode::EventsShared {
                            autoconvert: false,
                            buffer_duration_hns: minimum,
                        },
                    )
                    .map_err(|fallback_error| {
                        format!(
                            "IAudioClient3 capture unavailable ({low_latency_error}); standard shared fallback also failed ({fallback_error})"
                        )
                    })?;
                    (
                        mix_format,
                        AudioDeviceMode::SharedCompatible,
                        0,
                        format!(
                            "Shared compatible fallback · IAudioClient3 unavailable: {low_latency_error}"
                        ),
                    )
                }
            }
        }
        AudioDeviceMode::SharedCompatible => {
            let (default_period, _) = client
                .get_device_period()
                .map_err(|error| error.to_string())?;
            client
                .initialize_client(
                    &mix_format,
                    &Direction::Capture,
                    &StreamMode::EventsShared {
                        autoconvert: true,
                        buffer_duration_hns: default_period,
                    },
                )
                .map_err(|error| format!("could not initialize shared WASAPI capture: {error}"))?;
            (
                mix_format,
                AudioDeviceMode::SharedCompatible,
                0,
                "Shared compatible".to_owned(),
            )
        }
        AudioDeviceMode::Exclusive => {
            let rate = requested_rate.unwrap_or_else(|| mix_format.get_samplespersec());
            let format = choose_exclusive_capture_format(&client, &mix_format, rate)?;
            let (_, minimum) = client
                .get_device_period()
                .map_err(|error| error.to_string())?;
            let requested_hns = requested_period_frames
                .filter(|frames| *frames > 0)
                .map(|frames| {
                    i64::from(frames) * 10_000_000 / i64::from(format.get_samplespersec())
                })
                .unwrap_or(minimum);
            let period = client
                .calculate_aligned_period_near(requested_hns, None, &format)
                .map_err(|error| {
                    format!("could not choose an exclusive capture period: {error}")
                })?;
            client
                .initialize_client(
                    &format,
                    &Direction::Capture,
                    &StreamMode::EventsExclusive { period_hns: period },
                )
                .map_err(|error| {
                    format!("could not initialize exclusive WASAPI capture: {error}")
                })?;
            let frames = (period * i64::from(format.get_samplespersec()) / 10_000_000)
                .clamp(1, i64::from(u32::MAX)) as u32;
            (
                format,
                AudioDeviceMode::Exclusive,
                frames,
                "Exclusive · event driven".to_owned(),
            )
        }
    };
    let channels = format.get_nchannels();
    let sample_rate = format.get_samplespersec();
    let block_align = format.get_blockalign() as usize;
    if channels == 0 || sample_rate == 0 || block_align == 0 {
        return Err("WASAPI endpoint reported an invalid input format".to_owned());
    }
    let sample_bytes = block_align / usize::from(channels);
    if sample_bytes == 0 || sample_bytes * usize::from(channels) != block_align {
        return Err("WASAPI endpoint reported an unsupported input channel layout".to_owned());
    }
    let buffer_frames = client
        .get_buffer_size()
        .map_err(|error| format!("could not read WASAPI capture buffer size: {error}"))?;
    let event = client
        .set_get_eventhandle()
        .map_err(|error| format!("could not create the WASAPI capture event: {error}"))?;
    let capture: IAudioCaptureClient = unsafe { client.as_iaudioclient().GetService() }
        .map_err(|error| format!("could not create the WASAPI capture client: {error}"))?;
    let bytes = vec![0_u8; buffer_frames as usize * block_align];
    let mut samples = vec![0.0_f32; buffer_frames as usize * usize::from(channels)];
    client
        .start_stream()
        .map_err(|error| format!("could not start WASAPI capture: {error}"))?;
    let actual_period = if period_frames == 0 {
        client
            .get_current_shared_mode_engine_period()
            .ok()
            .map(|(_, value)| value)
            .unwrap_or(buffer_frames)
    } else {
        period_frames
    };
    if ready
        .send(Ok(WasapiInputInfo {
            device_name,
            sample_rate,
            channels,
            buffer_frames,
            period_frames: actual_period,
            mode,
            detail,
        }))
        .is_err()
    {
        stop.store(true, Ordering::Release);
    }
    let block_align = block_align as u32;
    while !stop.load(Ordering::Acquire) {
        if let Err(error) = event.wait_for_event(100) {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let _ = error;
            failed.store(true, Ordering::Release);
            break;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        loop {
            let packet_started = timing.as_ref().map(|_| Instant::now());
            let available = unsafe { capture.GetNextPacketSize() }.map_err(|error| {
                format!("could not query the next WASAPI capture packet: {error}")
            })?;
            if available == 0 {
                break;
            }
            if available > buffer_frames {
                failed.store(true, Ordering::Release);
                return Err(format!(
                    "WASAPI capture packet ({available} frames) exceeded its {}-frame buffer",
                    buffer_frames
                ));
            }
            let mut data = std::ptr::null_mut();
            let mut frames = 0_u32;
            let mut flags = 0_u32;
            let mut device_position = 0_u64;
            let mut timestamp_hns = 0_u64;
            unsafe {
                capture.GetBuffer(
                    &mut data,
                    &mut frames,
                    &mut flags,
                    Some(&mut device_position),
                    Some(&mut timestamp_hns),
                )
            }
            .map_err(|error| format!("could not read WASAPI capture data: {error}"))?;
            let packet_info = CapturePacketInfo {
                device_position,
                timestamp_hns,
                discontinuity: flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0,
                timestamp_error: flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 != 0,
            };
            let sample_count = frames as usize * usize::from(channels);
            let byte_count = frames as usize * block_align as usize;
            let process_result =
                if frames > 0 && sample_count <= samples.len() && byte_count <= bytes.len() {
                    if flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0 {
                        samples[..sample_count].fill(0.0);
                        Ok(())
                    } else if data.is_null() {
                        Err("WASAPI returned a null input buffer without a silence flag".to_owned())
                    } else {
                        let input = unsafe { std::slice::from_raw_parts(data, byte_count) };
                        decode_samples(&format, sample_bytes, input, &mut samples[..sample_count])
                    }
                } else if frames == 0 {
                    Ok(())
                } else {
                    Err("WASAPI returned an oversized capture packet".to_owned())
                };
            let release_result = unsafe { capture.ReleaseBuffer(frames) };
            process_result?;
            release_result
                .map_err(|error| format!("could not release WASAPI capture data: {error}"))?;
            if frames > 0 {
                consume(&samples[..sample_count], channels, packet_info);
            }
            if let (Some(started), Some(trace)) = (packet_started, timing.as_ref()) {
                trace.record(started);
            }
        }
    }
    let _ = client.stop_stream();
    Ok(())
}

fn choose_exclusive_capture_format(
    client: &AudioClient,
    mix_format: &WaveFormat,
    sample_rate: u32,
) -> Result<WaveFormat, String> {
    let channels = usize::from(mix_format.get_nchannels().clamp(1, 2));
    let candidates = [
        WaveFormat::new(
            32,
            32,
            &SampleType::Float,
            sample_rate as usize,
            channels,
            None,
        ),
        WaveFormat::new(
            32,
            24,
            &SampleType::Int,
            sample_rate as usize,
            channels,
            None,
        ),
        WaveFormat::new(
            24,
            24,
            &SampleType::Int,
            sample_rate as usize,
            channels,
            None,
        ),
        WaveFormat::new(
            16,
            16,
            &SampleType::Int,
            sample_rate as usize,
            channels,
            None,
        ),
    ];
    for candidate in candidates {
        if let Ok(format) = client.is_supported_exclusive_with_quirks(&candidate) {
            return Ok(format);
        }
    }
    Err(format!(
        "the endpoint does not support 16/24/32-bit PCM or 32-bit float capture at {sample_rate} Hz"
    ))
}

fn decode_samples(
    format: &WaveFormat,
    sample_bytes: usize,
    input: &[u8],
    output: &mut [f32],
) -> Result<(), String> {
    let sample_type = format
        .get_subformat()
        .map_err(|error| format!("unsupported WASAPI input subtype: {error}"))?;
    let bits = usize::from(format.get_bitspersample());
    let valid_bits = usize::from(format.get_validbitspersample()).min(bits);
    if input.len() != output.len() * sample_bytes || sample_bytes == 0 {
        return Err("WASAPI input packet has an invalid sample layout".to_owned());
    }
    match (sample_type, bits, valid_bits, sample_bytes) {
        (SampleType::Float, 32, 32, 4) => {
            for (sample, bytes) in output.iter_mut().zip(input.chunks_exact(4)) {
                *sample = f32::from_le_bytes(bytes.try_into().expect("four-byte sample"));
            }
        }
        (SampleType::Float, 64, 64, 8) => {
            for (sample, bytes) in output.iter_mut().zip(input.chunks_exact(8)) {
                *sample = f64::from_le_bytes(bytes.try_into().expect("eight-byte sample")) as f32;
            }
        }
        (SampleType::Int, 8, 8, 1) => {
            for (sample, byte) in output.iter_mut().zip(input) {
                *sample = (f32::from(*byte) - 128.0) / 128.0;
            }
        }
        (SampleType::Int, bits @ 16..=32, valid_bits, bytes) if bits.is_multiple_of(8) => {
            if !(16..=32).contains(&valid_bits) || bytes != bits / 8 {
                return Err(format!(
                    "unsupported WASAPI integer input encoding: {valid_bits}/{bits} bits in {bytes} bytes"
                ));
            }
            let padding = bits - valid_bits;
            let denominator = (1_i64 << (valid_bits - 1)) as f64;
            for (sample, raw) in output.iter_mut().zip(input.chunks_exact(bytes)) {
                let mut value = match bytes {
                    2 => i64::from(i16::from_le_bytes([raw[0], raw[1]])),
                    3 => {
                        let packed = i32::from(raw[0])
                            | (i32::from(raw[1]) << 8)
                            | (i32::from(raw[2]) << 16);
                        i64::from((packed << 8) >> 8)
                    }
                    4 => i64::from(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])),
                    _ => return Err("unsupported WASAPI integer sample width".to_owned()),
                };
                value >>= padding;
                *sample = (value as f64 / denominator).clamp(-1.0, 1.0) as f32;
            }
        }
        _ => {
            return Err(format!(
                "unsupported WASAPI input encoding: {sample_type:?}, {valid_bits}/{bits} bits"
            ));
        }
    }
    for sample in output {
        if !sample.is_finite() {
            *sample = 0.0;
        } else {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
    Ok(())
}

fn choose_exclusive_format(
    client: &AudioClient,
    mix_format: &WaveFormat,
    sample_rate: u32,
) -> Result<WaveFormat, String> {
    let channels = usize::from(mix_format.get_nchannels().clamp(1, 2));
    let candidates = [
        WaveFormat::new(
            32,
            32,
            &SampleType::Float,
            sample_rate as usize,
            channels,
            None,
        ),
        WaveFormat::new(
            32,
            24,
            &SampleType::Int,
            sample_rate as usize,
            channels,
            None,
        ),
        WaveFormat::new(
            24,
            24,
            &SampleType::Int,
            sample_rate as usize,
            channels,
            None,
        ),
        WaveFormat::new(
            16,
            16,
            &SampleType::Int,
            sample_rate as usize,
            channels,
            None,
        ),
    ];
    for candidate in candidates {
        if let Ok(format) = client.is_supported_exclusive_with_quirks(&candidate) {
            return Ok(format);
        }
    }
    Err(format!(
        "the endpoint does not support a 16/24/32-bit stereo PCM or 32-bit float stream at {sample_rate} Hz"
    ))
}

fn ceil_to_multiple(value: u32, multiple: u32) -> Option<u32> {
    if value == 0 || multiple == 0 {
        return None;
    }
    value
        .checked_add(multiple - 1)
        .map(|rounded| rounded / multiple * multiple)
}

fn nearest_engine_period(frames: u32, periods: wasapi::SharedModeEnginePeriod) -> u32 {
    let minimum = periods.min;
    let maximum = periods.max;
    if minimum == 0 || maximum < minimum || periods.fundamental == 0 {
        return minimum;
    }
    let target = frames.clamp(minimum, maximum);
    let lower = target / periods.fundamental * periods.fundamental;
    let upper = ceil_to_multiple(target, periods.fundamental).unwrap_or(maximum);
    [lower, upper]
        .into_iter()
        .filter(|value| minimum <= *value && *value <= maximum)
        .min_by_key(|value| value.abs_diff(target))
        .unwrap_or(minimum)
}

fn shared_period_choices(periods: wasapi::SharedModeEnginePeriod) -> Vec<u32> {
    if periods.fundamental == 0 || periods.max < periods.min {
        return Vec::new();
    }
    let mut choices = Vec::new();
    let mut period = periods.min;
    while period <= periods.max && choices.len() < 128 {
        choices.push(period);
        let Some(next) = period.checked_add(periods.fundamental) else {
            break;
        };
        period = next;
    }
    choices.extend([periods.default, periods.max]);
    choices.sort_unstable();
    choices.dedup();
    choices
}

fn encode_samples(
    format: &WaveFormat,
    sample_bytes: usize,
    samples: &[f32],
    output: &mut [u8],
) -> Result<(), String> {
    let sample_type = format
        .get_subformat()
        .map_err(|error| format!("unsupported WASAPI sample subtype: {error}"))?;
    let bits = usize::from(format.get_bitspersample());
    let valid_bits = usize::from(format.get_validbitspersample()).min(bits);
    if output.len() != samples.len() * sample_bytes || !matches!(sample_bytes, 2 | 3 | 4 | 8) {
        return Err("unsupported WASAPI sample storage size".to_owned());
    }
    match (sample_type, bits, sample_bytes) {
        (SampleType::Float, 32, 4) => {
            for (sample, bytes) in samples.iter().zip(output.chunks_exact_mut(4)) {
                bytes.copy_from_slice(&sample.to_le_bytes());
            }
        }
        (SampleType::Float, 64, 8) => {
            for (sample, bytes) in samples.iter().zip(output.chunks_exact_mut(8)) {
                bytes.copy_from_slice(&f64::from(*sample).to_le_bytes());
            }
        }
        (SampleType::Int, _, _) if (16..=32).contains(&valid_bits) && valid_bits <= bits => {
            let shift = bits - valid_bits;
            let maximum = (1_i64 << (valid_bits - 1)) - 1;
            let minimum = -(1_i64 << (valid_bits - 1));
            for (sample, bytes) in samples.iter().zip(output.chunks_exact_mut(sample_bytes)) {
                let quantized = (f64::from(sample.clamp(-1.0, 1.0)) * maximum as f64)
                    .round()
                    .clamp(minimum as f64, maximum as f64) as i64;
                let encoded = (quantized << shift).to_le_bytes();
                bytes.copy_from_slice(&encoded[..sample_bytes]);
            }
        }
        _ => {
            return Err(format!(
                "unsupported WASAPI output encoding: {sample_type:?}, {valid_bits}/{bits} bits"
            ));
        }
    }
    Ok(())
}
