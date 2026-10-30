//! Optional local ASIO adapter. One COM owner thread runs a duplex session.
use crate::{
    AsioMmcssTrace, AudioDeviceMode, CallbackTimingTrace, recording::AsioCapture,
    wasapi_backend::WasapiOutputInfo,
};
use maolan_asio::{Driver, StreamConfig};
use std::sync::{
    Arc, Mutex, OnceLock, Weak,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Threading::{
    AVRT_PRIORITY_HIGH, AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW,
    AvSetMmThreadPriority, GetCurrentThreadId,
};

use crate::AsioDeviceInfo;

static CAPTURE: OnceLock<Mutex<Option<(String, Weak<AsioCapture>)>>> = OnceLock::new();

struct AsioMmcssTask {
    handle: isize,
    thread_id: u32,
    priority_set: bool,
}

impl AsioMmcssTask {
    fn enter() -> Option<Self> {
        const PRO_AUDIO: [u16; 10] = [80, 114, 111, 32, 65, 117, 100, 105, 111, 0];
        let mut index = 0;
        // SAFETY: task name is nul-terminated static UTF-16 and index is valid.
        let handle = unsafe { AvSetMmThreadCharacteristicsW(PRO_AUDIO.as_ptr(), &mut index) };
        if handle.is_null() {
            return None;
        }
        // SAFETY: handle was returned by AvSetMmThreadCharacteristicsW above.
        let priority_set = unsafe { AvSetMmThreadPriority(handle, AVRT_PRIORITY_HIGH) } != 0;
        Some(Self {
            handle: handle as isize,
            // SAFETY: this call returns the ID of the current callback thread.
            thread_id: unsafe { GetCurrentThreadId() },
            priority_set,
        })
    }

    fn revert(self) -> bool {
        // SAFETY: callback teardown verifies the current thread matches the
        // thread that registered this AVRT handle.
        unsafe { AvRevertMmThreadCharacteristics(self.handle as HANDLE) != 0 }
    }
}

fn service_mmcss_setup(
    trace: &AsioMmcssTrace,
    task: &mut Option<AsioMmcssTask>,
    attempted: &mut bool,
) {
    if !trace.enabled || *attempted {
        return;
    }
    *attempted = true;
    if trace.begin_setup() {
        let started = std::time::Instant::now();
        *task = AsioMmcssTask::enter();
        let elapsed = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        trace.finish_setup(
            task.is_some(),
            task.as_ref().is_some_and(|task| task.priority_set),
            elapsed,
            task.as_ref().map_or(0, |task| task.thread_id),
        );
    }
}

fn service_mmcss_teardown(trace: &AsioMmcssTrace, task: &mut Option<AsioMmcssTask>) {
    if !trace.enabled || !trace.teardown_requested.load(Ordering::Acquire) {
        return;
    }
    let Some(registered) = task.as_ref() else {
        if trace.state.load(Ordering::Acquire) == AsioMmcssTrace::SETUP_FAILED {
            trace.teardown_complete.store(true, Ordering::Release);
        }
        return;
    };
    // SAFETY: this checks that AVRT reversion runs on its registering thread.
    if unsafe { GetCurrentThreadId() } != registered.thread_id {
        return;
    }
    let registered = task.take().expect("task checked above");
    trace.finish_teardown(registered.revert());
}

pub(crate) fn capture(id: &str) -> Result<Arc<AsioCapture>, String> {
    CAPTURE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "ASIO session state unavailable")?
        .as_ref()
        .filter(|(active, _)| active == id)
        .and_then(|(_, weak)| weak.upgrade())
        .ok_or_else(|| {
            "Open the same ASIO driver for playback before selecting its inputs".to_owned()
        })
}

fn descriptor(id: &str) -> Result<maolan_asio::DriverInfo, String> {
    maolan_asio::drivers()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|driver| driver_id(driver) == id)
        .ok_or_else(|| "The selected ASIO driver is no longer installed".to_owned())
}

pub(crate) fn driver_name(id: &str) -> Result<String, String> {
    Ok(descriptor(id)?.name)
}

pub(crate) fn driver_id(driver: &maolan_asio::DriverInfo) -> String {
    let c = driver.clsid;
    format!(
        "asio:{:08x}-{:04x}-{:04x}-{}",
        c.data1,
        c.data2,
        c.data3,
        c.data4
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

fn buffer_choices(s: maolan_asio::BufferSizes) -> Vec<u32> {
    if s.min < 1 || s.max < s.min {
        return Vec::new();
    }
    if s.granularity == 0 {
        return vec![s.preferred.clamp(s.min, s.max) as u32];
    }
    if s.granularity == -1 {
        let mut n = s.min as u32;
        let mut choices = Vec::new();
        while n <= s.max as u32 {
            choices.push(n);
            let Some(next) = n.checked_mul(2) else {
                break;
            };
            n = next;
        }
        return choices;
    }
    let mut choices = vec![s.min, s.preferred, s.max];
    choices.extend([
        32, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024, 2048, 4096,
    ]);
    choices.retain(|n| {
        *n >= s.min && *n <= s.max && s.granularity > 0 && (*n - s.min) % s.granularity == 0
    });
    choices.sort_unstable();
    choices.dedup();
    choices.into_iter().map(|n| n as u32).collect()
}

pub(crate) fn control_panel(id: String) -> Result<(), String> {
    thread::spawn(move || {
        let guard = maolan_asio::com::initialize().map_err(|e| format!("ASIO COM: {e:?}"))?;
        let mut driver = Driver::open(&descriptor(&id)?, &guard).map_err(|e| e.to_string())?;
        driver.control_panel().map_err(|e| e.to_string())
    })
    .join()
    .map_err(|_| "ASIO control panel thread failed".to_owned())?
}

pub(crate) struct AsioOutputStream {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    pub info: WasapiOutputInfo,
    pub driver_info: AsioDeviceInfo,
}

impl AsioOutputStream {
    pub fn open(
        id: String,
        rate: Option<u32>,
        frames: Option<u32>,
        failed: Arc<AtomicBool>,
        overload_windows: Arc<AtomicU64>,
        callback_collisions: Arc<AtomicU64>,
        mmcss_trace: Arc<AsioMmcssTrace>,
        callback_timing: Arc<CallbackTimingTrace>,
        make_renderer: impl FnOnce(u32, u16) -> Box<dyn FnMut(&mut [f32]) + Send> + Send + 'static,
    ) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let overload_windows = Arc::clone(&overload_windows);
        let callback_collisions = Arc::clone(&callback_collisions);
        let mmcss_trace = Arc::clone(&mmcss_trace);
        let callback_timing = Arc::clone(&callback_timing);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("opentrack-asio-owner".into())
            .spawn(move || {
                let run = || -> Result<(), String> {
                    let guard =
                        maolan_asio::com::initialize().map_err(|e| format!("ASIO COM: {e:?}"))?;
                    let desc = descriptor(&id)?;
                    let mut driver = Driver::open(&desc, &guard).map_err(|e| e.to_string())?;
                    let (inputs, outputs) = driver.channels().map_err(|e| e.to_string())?;
                    if inputs < 1 || outputs < 1 {
                        return Err("The driver must expose input and output channels".into());
                    }
                    let actual_rate = rate
                        .map(f64::from)
                        .unwrap_or(driver.sample_rate().map_err(|e| e.to_string())?);
                    if driver.sample_rate().map_err(|e| e.to_string())? != actual_rate {
                        driver
                            .set_sample_rate(actual_rate)
                            .map_err(|e| e.to_string())?;
                    }
                    let sizes = driver.buffer_sizes().map_err(|e| e.to_string())?;
                    if sizes.max > 1_048_576 || sizes.max < 1 {
                        return Err("Invalid ASIO buffer limits".into());
                    }
                    let (input_latency, output_latency) =
                        driver.latencies().map_err(|e| e.to_string())?;
                    let port_names = |input, count| {
                        (0..count)
                            .map(|n| {
                                driver
                                    .channel_info(n, input)
                                    .map(|c| c.name)
                                    .map_err(|e| e.to_string())
                            })
                            .collect::<Result<Vec<_>, _>>()
                    };
                    let mut info = AsioDeviceInfo {
                        name: desc.name.clone(),
                        input_ports: port_names(true, inputs.min(2))?,
                        output_ports: port_names(false, outputs.min(2))?,
                        sample_rates: [22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000]
                            .into_iter()
                            .filter(|r| driver.can_sample_rate(f64::from(*r)))
                            .collect(),
                        buffer_choices: buffer_choices(sizes),
                        input_latency_frames: input_latency.max(0) as u32,
                        output_latency_frames: output_latency.max(0) as u32,
                    };
                    let config = StreamConfig {
                        sample_rate: actual_rate,
                        buffer_size: frames.unwrap_or(sizes.preferred.max(1) as u32) as usize,
                        num_inputs: inputs.min(2) as usize,
                        num_outputs: outputs.min(2) as usize,
                    };
                    let capture = Arc::new(AsioCapture::new(
                        desc.name.clone(),
                        actual_rate as u32,
                        config.num_inputs as u16,
                        Arc::clone(&failed),
                    ));
                    let callback_capture = Arc::clone(&capture);
                    let callback_failed = Arc::clone(&failed);
                    let callback_mmcss_trace = Arc::clone(&mmcss_trace);
                    let mut render = make_renderer(actual_rate as u32, config.num_outputs as u16);
                    let mut mmcss_task = None;
                    let mut mmcss_setup_attempted = false;
                    let mut input = vec![0.0_f32; sizes.max.max(1) as usize * config.num_inputs];
                    let mut output = vec![0.0_f32; sizes.max.max(1) as usize * config.num_outputs];
                    let mut stream = driver
                        .stream_with_timing(
                            &config,
                            move |mut ctx| {
                                service_mmcss_setup(
                                    &callback_mmcss_trace,
                                    &mut mmcss_task,
                                    &mut mmcss_setup_attempted,
                                );
                                let frames = ctx.frames();
                                let ni = ctx.num_inputs();
                                let no = ctx.num_outputs();
                                if frames * ni > input.len() || frames * no > output.len() {
                                    callback_failed.store(true, Ordering::Release);
                                    service_mmcss_teardown(&callback_mmcss_trace, &mut mmcss_task);
                                    return;
                                }
                                for ch in 0..ni {
                                    for (n, sample) in ctx.input(ch).iter().enumerate() {
                                        input[n * ni + ch] = *sample;
                                    }
                                }
                                callback_capture.push(&mut input[..frames * ni]);
                                render(&mut output[..frames * no]);
                                for ch in 0..no {
                                    for (n, sample) in ctx.output(ch).iter_mut().enumerate() {
                                        *sample = output[n * no + ch];
                                    }
                                }
                                service_mmcss_teardown(&callback_mmcss_trace, &mut mmcss_task);
                            },
                            callback_timing.enabled(),
                        )
                        .map_err(|e| e.to_string())?;
                    let actual_frames = stream.buffer_size as u32;
                    capture
                        .period_frames
                        .store(actual_frames, Ordering::Release);
                    *CAPTURE
                        .get_or_init(|| Mutex::new(None))
                        .lock()
                        .map_err(|_| "ASIO session state unavailable")? =
                        Some((id.clone(), Arc::downgrade(&capture)));
                    stream.start().map_err(|e| e.to_string())?;
                    let (input_latency, output_latency) =
                        stream.latencies().map_err(|e| e.to_string())?;
                    info.input_latency_frames = input_latency.max(0) as u32;
                    info.output_latency_frames = output_latency.max(0) as u32;
                    let output_info = WasapiOutputInfo {
                        device_name: desc.name,
                        sample_rate: actual_rate as u32,
                        channels: config.num_outputs as u16,
                        buffer_frames: actual_frames,
                        period_frames: actual_frames,
                        period_choices: info.buffer_choices.clone(),
                        mode: AudioDeviceMode::Exclusive,
                        detail: "ASIO full duplex".into(),
                    };
                    let _ = ready_tx.send(Ok((output_info, info)));
                    while !worker_stop.load(Ordering::Acquire) {
                        let events = stream.take_events();
                        let collisions = stream.take_callback_collisions();
                        if collisions > 0 {
                            callback_collisions.fetch_add(collisions, Ordering::Relaxed);
                        }
                        if callback_timing.enabled() {
                            let timing_batch = stream.take_callback_timing_samples();
                            callback_timing.report_overwritten(timing_batch.overwritten);
                            for sample in timing_batch.samples {
                                callback_timing.record_ns(sample.interval_ns, sample.duration_ns);
                            }
                        }
                        if events & maolan_asio::stream::event::OVERLOAD != 0 {
                            overload_windows.fetch_add(1, Ordering::Relaxed);
                        }
                        if events
                            & (maolan_asio::stream::event::RESET_REQUESTED
                                | maolan_asio::stream::event::BUFFER_SIZE_CHANGED
                                | maolan_asio::stream::event::SAMPLE_RATE_CHANGED
                                | maolan_asio::stream::event::RESYNC_REQUESTED
                                | maolan_asio::stream::event::LATENCIES_CHANGED)
                            != 0
                        {
                            failed.store(true, Ordering::Release);
                            break;
                        }
                        // The COM owner also pumps driver-created Windows messages.
                        unsafe {
                            use windows_sys::Win32::UI::WindowsAndMessaging::*;
                            let mut message: MSG = std::mem::zeroed();
                            while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE)
                                != 0
                            {
                                TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    mmcss_trace.request_teardown();
                    if mmcss_trace.enabled {
                        let deadline = std::time::Instant::now() + Duration::from_secs(1);
                        while !mmcss_trace.teardown_complete.load(Ordering::Acquire)
                            && std::time::Instant::now() < deadline
                        {
                            thread::sleep(Duration::from_millis(1));
                        }
                        if !mmcss_trace.teardown_complete.load(Ordering::Acquire) {
                            mmcss_trace
                                .state
                                .store(AsioMmcssTrace::TEARDOWN_TIMEOUT, Ordering::Release);
                        }
                    }
                    let _ = stream.stop();
                    let collisions = stream.take_callback_collisions();
                    if collisions > 0 {
                        callback_collisions.fetch_add(collisions, Ordering::Relaxed);
                    }
                    if callback_timing.enabled() {
                        let timing_batch = stream.take_callback_timing_samples();
                        callback_timing.report_overwritten(timing_batch.overwritten);
                        for sample in timing_batch.samples {
                            callback_timing.record_ns(sample.interval_ns, sample.duration_ns);
                        }
                    }
                    drop(stream);
                    capture.failed.store(true, Ordering::Release);
                    Ok(())
                };
                if let Err(error) = run() {
                    failed.store(true, Ordering::Release);
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|e| e.to_string())?;
        match ready_rx.recv() {
            Ok(Ok((info, driver_info))) => Ok(Self {
                stop,
                worker: Some(worker),
                info,
                driver_info,
            }),
            Ok(Err(error)) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(error.to_string())
            }
        }
    }
}

impl Drop for AsioOutputStream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn driver_buffer_constraints_produce_only_valid_choices() {
        let sizes = maolan_asio::BufferSizes {
            min: 8,
            max: 2048,
            preferred: 128,
            granularity: -1,
        };
        assert_eq!(
            buffer_choices(sizes),
            [8, 16, 32, 64, 128, 256, 512, 1024, 2048]
        );
        assert_eq!(
            buffer_choices(maolan_asio::BufferSizes {
                granularity: 0,
                ..sizes
            }),
            [128]
        );
        let stepped = maolan_asio::BufferSizes {
            min: 8,
            max: 104,
            preferred: 56,
            granularity: 16,
        };
        assert!(
            buffer_choices(stepped)
                .into_iter()
                .all(|n| (n - 8) % 16 == 0)
        );
        assert!(buffer_choices(stepped).contains(&56));
    }
}
