//! Streaming: buffer allocation, host callback trampolines, and the
//! start/stop lifecycle.
//!
//! # One stream per process
//!
//! The ABI's `ASIOCallbacks` table has no user-data pointer, so the
//! trampoline functions recover the stream state through a process-global
//! pointer (the same approach as JUCE and cpal). Creating a second
//! simultaneous stream fails with [`StreamError::Busy`].
//!
//! # Real-time rules
//!
//! The user callback runs on the driver's buffer-switch thread. It must
//! not allocate, block, or hold locks for a significant time.

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::fmt;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering};

use crate::abi::{
    self, ASE_OK, AsioBool, AsioBufferInfo, AsioCallbacks, AsioTime, AsioTimeInfo, Iasio,
    K_ASIO_BUFFER_SIZE_CHANGE, K_ASIO_ENGINE_VERSION, K_ASIO_LATENCIES_CHANGED, K_ASIO_OVERLOAD,
    K_ASIO_RESET_REQUEST, K_ASIO_RESYNC_REQUEST, K_ASIO_SELECTOR_SUPPORTED,
    K_ASIO_SUPPORTS_INPUT_MONITOR, K_ASIO_SUPPORTS_TIME_CODE, K_ASIO_SUPPORTS_TIME_INFO,
};
use crate::convert;
use crate::driver::{BufferSizes, Driver, SampleType};
use crate::error::{AsioError, StreamError, check};

/// Process-global pointer to the active stream's state; see the module
/// docs for why this exists.
static ACTIVE: AtomicPtr<StreamInner> = AtomicPtr::new(ptr::null_mut());

/// Driver-to-host notification bits accumulated between
/// [`Stream::take_events`] calls.
pub mod event {
    /// The driver asked for a full reset/renegotiation
    /// (`kAsioResetRequest`).
    pub const RESET_REQUESTED: u32 = 1 << 0;
    /// The driver changed the buffer size (`kAsioBufferSizeChange`).
    pub const BUFFER_SIZE_CHANGED: u32 = 1 << 1;
    /// The driver lost sync (`kAsioResyncRequest`).
    pub const RESYNC_REQUESTED: u32 = 1 << 2;
    /// The driver reports changed latencies (`kAsioLatenciesChanged`).
    pub const LATENCIES_CHANGED: u32 = 1 << 3;
    /// An overload (dropout) occurred (`kAsioOverload`).
    pub const OVERLOAD: u32 = 1 << 4;
    /// The sample rate changed mid-stream (`sampleRateDidChange`).
    pub const SAMPLE_RATE_CHANGED: u32 = 1 << 5;
}

/// Desired stream parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamConfig {
    /// Sample rate in Hz; the driver is switched to it if needed.
    pub sample_rate: f64,
    /// Requested buffer size in frames; snapped to the driver's
    /// constraints at creation.
    pub buffer_size: usize,
    /// Number of input channels to open (indices `0..num_inputs`).
    pub num_inputs: usize,
    /// Number of output channels to open (indices `0..num_outputs`).
    pub num_outputs: usize,
}

impl StreamConfig {
    /// Full-duplex stream on all channels of the device at `sample_rate`.
    pub fn for_device(
        driver: &Driver,
        sample_rate: f64,
        buffer_size: usize,
    ) -> Result<Self, AsioError> {
        let (inputs, outputs) = driver.channels()?;
        Ok(Self {
            sample_rate,
            buffer_size,
            num_inputs: inputs as usize,
            num_outputs: outputs as usize,
        })
    }
}

/// Timestamp information for a buffer switch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StreamTime {
    /// Stream speed relative to nominal (1.0 = nominal).
    pub speed: f64,
    /// System time in nanoseconds.
    pub system_time_ns: i64,
    /// Stream position in samples.
    pub sample_position: i64,
    /// Raw `ASIOTimeInfo` flags.
    pub flags: u32,
}

impl StreamTime {
    fn from_raw(info: AsioTimeInfo) -> Self {
        Self {
            speed: info.speed,
            system_time_ns: info.system_time.to_i64(),
            sample_position: info.sample_position.to_i64(),
            flags: info.flags,
        }
    }
}

/// Access to the stream's channel buffers inside the user callback.
pub struct CallbackContext<'a> {
    channels: &'a mut [Channel],
    num_inputs: usize,
    frames: usize,
    /// Which half-buffer (0 or 1) is being processed.
    pub buffer_index: usize,
    time: Option<StreamTime>,
}

impl CallbackContext<'_> {
    /// Frames per buffer.
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Timestamp for this switch, when the driver supports time info.
    pub fn time(&self) -> Option<StreamTime> {
        self.time
    }

    /// Number of open input channels.
    pub fn num_inputs(&self) -> usize {
        self.num_inputs
    }

    /// Number of open output channels.
    pub fn num_outputs(&self) -> usize {
        self.channels.len() - self.num_inputs
    }

    /// Converted input channel `ch` for this switch. Panics if `ch` is
    /// out of range.
    pub fn input(&self, ch: usize) -> &[f32] {
        let channel = &self.channels[ch];
        debug_assert!(channel.is_input);
        &channel.scratch[..self.frames]
    }

    /// Writable output channel `ch`; the contents are copied to the
    /// driver after the callback returns. Panics if `ch` is out of range.
    pub fn output(&mut self, ch: usize) -> &mut [f32] {
        let channel = &mut self.channels[self.num_inputs + ch];
        debug_assert!(!channel.is_input);
        &mut channel.scratch[..self.frames]
    }
}

/// One open channel's buffers and conversion scratch.
struct Channel {
    is_input: bool,
    sample_type: SampleType,
    raw: [*mut u8; 2],
    scratch: Vec<f32>,
}

/// Stream state reachable from the callback trampolines.
struct StreamInner {
    driver: *mut Iasio,
    frames: usize,
    num_inputs: usize,
    channels: UnsafeCell<Vec<Channel>>,
    callback: UnsafeCell<Box<dyn FnMut(CallbackContext<'_>) + Send + 'static>>,
    callback_active: AtomicBool,
    host_callbacks: AsioCallbacks,
    current_rate: AtomicU64,
    output_ready: bool,
    events: AtomicU32,
}

impl StreamInner {
    fn run_callback(&self, index: usize, time: Option<StreamTime>) {
        if index > 1
            || self
                .callback_active
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
        {
            return;
        }
        // SAFETY: this gate permits only one audio callback to access scratch or the closure.
        let channels = unsafe { &mut *self.channels.get() };
        let callback = unsafe { &mut *self.callback.get() };
        let frames = self.frames;
        for channel in channels.iter_mut().filter(|c| c.is_input) {
            // SAFETY: `createBuffers` returned valid half-buffers of
            // `frames * size` bytes; the driver passes `index` in 0..2.
            unsafe {
                convert::to_f32(
                    channel.sample_type,
                    channel.raw[index],
                    frames,
                    &mut channel.scratch,
                );
            }
        }
        callback(CallbackContext {
            channels,
            num_inputs: self.num_inputs,
            frames,
            buffer_index: index,
            time,
        });
        for channel in channels.iter_mut().filter(|c| !c.is_input) {
            // SAFETY: same guarantees as the input conversion.
            unsafe { convert::from_f32(channel.sample_type, &channel.scratch, channel.raw[index]) };
        }
        if self.output_ready {
            // SAFETY: `driver` outlives the stream (mutably borrowed via
            // `Stream<'a>`), and `outputReady` is a plain ABI call.
            let driver = self.driver;
            unsafe {
                let vtbl = (*(driver)).lp_vtbl.as_ref().expect("null vtable");
                let _ = (vtbl.output_ready)(driver);
            };
        }
        self.callback_active.store(false, Ordering::Release);
    }
}

// --- callback trampolines (ABI: plain C function pointers) ------------

fn with_inner(f: impl FnOnce(&StreamInner)) {
    let p = ACTIVE.load(Ordering::Acquire);
    if !p.is_null() {
        // SAFETY: set at stream creation, cleared before destruction; the
        // driver only invokes callbacks between createBuffers and
        // disposeBuffers.
        f(unsafe { &*p });
    }
}

unsafe extern "C" fn buffer_switch_tr(index: i32, _direct: AsioBool) {
    with_inner(|inner| inner.run_callback(index as usize, None));
}

unsafe extern "C" fn buffer_switch_time_info_tr(
    params: *mut AsioTime,
    index: i32,
    _direct: AsioBool,
) -> *mut AsioTime {
    with_inner(|inner| {
        // SAFETY: the driver passes a valid AsioTime for this switch.
        if params.is_null() {
            inner.run_callback(index as usize, None);
            return;
        }
        let time = unsafe { (*params).time_info };
        inner.run_callback(index as usize, Some(StreamTime::from_raw(time)));
    });
    params
}

unsafe extern "C" fn sample_rate_did_change_tr(rate: f64) {
    with_inner(|inner| {
        inner.current_rate.store(rate.to_bits(), Ordering::Relaxed);
        inner
            .events
            .fetch_or(event::SAMPLE_RATE_CHANGED, Ordering::Relaxed);
    });
}

unsafe extern "C" fn asio_message_tr(
    selector: i32,
    value: i32,
    _message: *mut c_void,
    _opt: *mut f64,
) -> i32 {
    let bit = match selector {
        K_ASIO_RESET_REQUEST => Some(event::RESET_REQUESTED),
        K_ASIO_BUFFER_SIZE_CHANGE => Some(event::BUFFER_SIZE_CHANGED),
        K_ASIO_RESYNC_REQUEST => Some(event::RESYNC_REQUESTED),
        K_ASIO_LATENCIES_CHANGED => Some(event::LATENCIES_CHANGED),
        K_ASIO_OVERLOAD => Some(event::OVERLOAD),
        _ => None,
    };
    if let Some(bit) = bit {
        let p = ACTIVE.load(Ordering::Relaxed);
        if !p.is_null() {
            // SAFETY: pointer valid while a stream is active.
            unsafe { (*p).events.fetch_or(bit, Ordering::Relaxed) };
        }
    }
    match selector {
        K_ASIO_SELECTOR_SUPPORTED => match value {
            K_ASIO_RESET_REQUEST
            | K_ASIO_ENGINE_VERSION
            | K_ASIO_RESYNC_REQUEST
            | K_ASIO_LATENCIES_CHANGED
            | K_ASIO_SUPPORTS_INPUT_MONITOR
            | K_ASIO_OVERLOAD
            | K_ASIO_SUPPORTS_TIME_INFO
            | abi::K_ASIO_SUPPORTS_OUTPUT_READY => 1,
            _ => 0,
        },
        K_ASIO_RESET_REQUEST | K_ASIO_RESYNC_REQUEST | K_ASIO_LATENCIES_CHANGED => 1,
        K_ASIO_ENGINE_VERSION => abi::ASIO_ENGINE_VERSION,
        K_ASIO_SUPPORTS_TIME_INFO => 1,
        K_ASIO_SUPPORTS_TIME_CODE => 0,
        K_ASIO_OVERLOAD => 1,
        _ => 0,
    }
}

/// Pick a buffer size that satisfies the driver's constraints.
fn negotiate_buffer_size(requested: usize, sizes: &BufferSizes) -> i32 {
    let min = i64::from(sizes.min).max(1);
    let max = i64::from(sizes.max).max(min);
    let mut v = (requested as i64).clamp(min, max);
    match sizes.granularity {
        0 => {
            v = i64::from(sizes.preferred).clamp(min, max);
        }
        -1 => {
            let mut p = min;
            while p.saturating_mul(2) <= v {
                p *= 2;
            }
            v = p;
        }
        g if g > 0 => {
            let g = i64::from(g);
            v = min + ((v - min + g / 2) / g).max(0) * g;
            if v > max {
                v = min + ((max - min) / g) * g;
            }
        }
        _ => {
            v = i64::from(sizes.preferred).clamp(min, max);
        }
    }
    v as i32
}

/// A running or ready stream. Borrows the driver mutably: while the
/// stream exists, nothing else can touch the driver. Dropping stops the
/// stream and disposes the buffers.
pub struct Stream<'a> {
    driver: &'a mut Driver,
    inner: Box<StreamInner>,
    started: bool,
    /// Negotiated buffer size in frames (may differ from the request).
    pub buffer_size: usize,
}

impl Stream<'_> {
    /// Start the driver's engine; buffer switches begin arriving on the
    /// callback.
    pub fn start(&mut self) -> Result<(), AsioError> {
        if self.started {
            return Ok(());
        }
        self.driver.start()?;
        self.started = true;
        Ok(())
    }

    /// Stop the engine.
    pub fn stop(&mut self) -> Result<(), AsioError> {
        if !self.started {
            return Ok(());
        }
        self.driver.stop()?;
        self.started = false;
        Ok(())
    }

    /// Sample rate as last reported by the driver (may have changed
    /// mid-stream; see [`Stream::take_events`]).
    pub fn current_sample_rate(&self) -> f64 {
        f64::from_bits(self.inner.current_rate.load(Ordering::Relaxed))
    }

    /// Driver-reported input/output latency after buffer negotiation.
    pub fn latencies(&self) -> Result<(i32, i32), AsioError> {
        self.driver.latencies()
    }

    /// Drain pending driver events; returns the accumulated bitmask of
    /// `event::*` bits.
    pub fn take_events(&self) -> u32 {
        self.inner.events.swap(0, Ordering::Relaxed)
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        let _ = self.stop();
        let _ = self.driver.dispose_buffers();
        ACTIVE.store(ptr::null_mut(), Ordering::Release);
    }
}

impl fmt::Debug for Stream<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream")
            .field("frames", &self.inner.frames)
            .field("inputs", &self.inner.num_inputs)
            .field("started", &self.started)
            .finish_non_exhaustive()
    }
}

impl Driver {
    /// Open a stream: negotiate the sample rate and buffer size, query
    /// channel formats, allocate buffers, and install the callback table.
    ///
    /// The callback runs on the driver's buffer-switch thread; see the
    /// module docs for the real-time rules.
    pub fn stream(
        &mut self,
        config: &StreamConfig,
        callback: impl FnMut(CallbackContext<'_>) + Send + 'static,
    ) -> Result<Stream<'_>, StreamError> {
        if !config.sample_rate.is_finite() || config.sample_rate <= 0.0 {
            return Err(AsioError::InvalidParameter.into());
        }
        if !self.can_sample_rate(config.sample_rate) {
            return Err(AsioError::InvalidMode.into());
        }
        if self.sample_rate()? != config.sample_rate {
            self.set_sample_rate(config.sample_rate)?;
        }

        // Inventory channels and validate convertible formats up front.
        let mut channels: Vec<Channel> = Vec::new();
        for input in [true, false] {
            let count = if input {
                config.num_inputs
            } else {
                config.num_outputs
            };
            for index in 0..count {
                let info = self.channel_info(index as i32, input)?;
                if !convert::supported(info.sample_type) {
                    return Err(StreamError::UnsupportedSampleType(info.sample_type.raw()));
                }
                channels.push(Channel {
                    is_input: input,
                    sample_type: info.sample_type,
                    raw: [ptr::null_mut(); 2],
                    scratch: Vec::new(),
                });
            }
        }

        let sizes = self.buffer_sizes()?;
        let frames = negotiate_buffer_size(config.buffer_size, &sizes) as usize;
        for channel in &mut channels {
            channel.scratch.resize(frames, 0.0);
        }

        let inner = Box::new(StreamInner {
            driver: self.raw(),
            frames,
            num_inputs: config.num_inputs,
            channels: UnsafeCell::new(channels),
            callback: UnsafeCell::new(Box::new(callback)),
            callback_active: AtomicBool::new(false),
            host_callbacks: AsioCallbacks {
                buffer_switch: Some(buffer_switch_tr),
                sample_rate_did_change: Some(sample_rate_did_change_tr),
                asio_message: Some(asio_message_tr),
                buffer_switch_time_info: Some(buffer_switch_time_info_tr),
            },
            current_rate: AtomicU64::new(config.sample_rate.to_bits()),
            output_ready: false,
            events: AtomicU32::new(0),
        });
        let ptr = Box::into_raw(inner);
        if ACTIVE
            .compare_exchange(ptr::null_mut(), ptr, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            // SAFETY: freshly allocated above, never shared.
            drop(unsafe { Box::from_raw(ptr) });
            return Err(StreamError::Busy);
        }

        match self.setup_buffers(unsafe { &mut *ptr }, &sizes) {
            Ok(()) => {
                // SAFETY: ACTIVE holds the box; we take ownership back.
                let inner = unsafe { Box::from_raw(ptr) };
                let frames = inner.frames;
                Ok(Stream {
                    driver: self,
                    inner,
                    started: false,
                    buffer_size: frames,
                })
            }
            Err(e) => {
                let _ = self.dispose_buffers();
                ACTIVE.store(ptr::null_mut(), Ordering::Release);
                // SAFETY: owned here; no driver callbacks will fire after
                // disposeBuffers.
                drop(unsafe { Box::from_raw(ptr) });
                Err(e)
            }
        }
    }

    /// Allocate driver buffers, install the callback table, and probe
    /// `outputReady` support (the empirical approach: call it once and
    /// check for `ASE_OK`, as JUCE does).
    fn setup_buffers(
        &mut self,
        inner: &mut StreamInner,
        sizes: &BufferSizes,
    ) -> Result<(), StreamError> {
        fn create(driver: &Driver, inner: &mut StreamInner, frames: i32) -> Result<(), AsioError> {
            let mut infos: Vec<AsioBufferInfo> = Vec::with_capacity(inner.channels.get_mut().len());
            let (mut in_idx, mut out_idx) = (0i32, 0i32);
            for channel in inner.channels.get_mut().iter() {
                let index = if channel.is_input {
                    let i = in_idx;
                    in_idx += 1;
                    i
                } else {
                    let i = out_idx;
                    out_idx += 1;
                    i
                };
                infos.push(AsioBufferInfo {
                    is_input: if channel.is_input {
                        crate::abi::ASIO_TRUE
                    } else {
                        crate::abi::ASIO_FALSE
                    },
                    channel: index,
                    buffers: [ptr::null_mut(); 2],
                });
            }
            let num_channels = infos.len() as i32;
            // SAFETY: `infos` is valid for `num_channels` entries;
            // the callback table is pinned inside the stream for its full lifetime. The driver fills
            // `infos[i].buffers`.
            check(unsafe {
                (driver.vtbl().create_buffers)(
                    driver.this(),
                    infos.as_mut_ptr(),
                    num_channels,
                    frames,
                    &mut inner.host_callbacks,
                )
            })?;
            for (channel, info) in inner.channels.get_mut().iter_mut().zip(&infos) {
                channel.raw = info.buffers.map(|p| p.cast::<u8>());
            }
            Ok(())
        }

        let mut result = create(self, inner, inner.frames as i32);
        if result.is_err() && inner.frames as i32 != sizes.preferred {
            // Retry once with the driver's preferred size (some drivers
            // reject sizes outside a narrower internal range).
            let _ = self.dispose_buffers();
            let preferred = negotiate_buffer_size(sizes.preferred as usize, sizes) as usize;
            inner.frames = preferred;
            for channel in inner.channels.get_mut().iter_mut() {
                channel.scratch.resize(preferred, 0.0);
            }
            result = create(self, inner, preferred as i32);
        }
        result?;

        // Empirical outputReady probe: supported iff it returns ASE_OK.
        // SAFETY: plain ABI call on a live driver.
        let code = unsafe { (self.vtbl().output_ready)(self.this()) };
        inner.output_ready = code == ASE_OK;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeDriver;

    /// Serialize streaming tests: the ACTIVE pointer is process-global.
    static STREAM_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn negotiates_buffer_size() {
        let sizes = BufferSizes {
            min: 16,
            max: 2048,
            preferred: 512,
            granularity: -1,
        };
        assert_eq!(negotiate_buffer_size(512, &sizes), 512);
        assert_eq!(negotiate_buffer_size(500, &sizes), 256);
        assert_eq!(negotiate_buffer_size(4096, &sizes), 2048);
        assert_eq!(negotiate_buffer_size(1, &sizes), 16);

        let continuous = BufferSizes {
            min: 32,
            max: 480,
            preferred: 192,
            granularity: 1,
        };
        assert_eq!(negotiate_buffer_size(100, &continuous), 100);

        let stepped = BufferSizes {
            min: 64,
            max: 1024,
            preferred: 512,
            granularity: 48,
        };
        assert_eq!(negotiate_buffer_size(100, &stepped), 64 + 48);
    }

    #[test]
    fn rejects_unsupported_sample_rate() {
        let _lock = STREAM_LOCK.lock().unwrap();
        let mut fake = FakeDriver::new();
        let raw = (&mut *fake) as *mut FakeDriver as *mut Iasio;
        let mut driver = unsafe { Driver::from_raw(raw) };
        let err = driver
            .stream(
                &StreamConfig {
                    sample_rate: 123.0,
                    buffer_size: 64,
                    num_inputs: 1,
                    num_outputs: 1,
                },
                |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, StreamError::Asio(AsioError::InvalidMode)));
    }

    #[test]
    fn full_duplex_round_trip_with_time_info() {
        let _lock = STREAM_LOCK.lock().unwrap();
        let mut fake = FakeDriver::new();
        // Use Float32 channels so conversions are exact.
        for ch in &mut fake.channels {
            ch.sample_type = SampleType::Float32Lsb.raw();
            ch.name = "f32".to_owned();
        }
        let raw = (&mut *fake) as *mut FakeDriver as *mut Iasio;
        let mut driver = unsafe { Driver::from_raw(raw) };

        let frames = 64usize;
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_cb = seen.clone();
        let mut stream = driver
            .stream(
                &StreamConfig {
                    sample_rate: 48_000.0,
                    buffer_size: frames,
                    num_inputs: 2,
                    num_outputs: 2,
                },
                move |mut ctx| {
                    assert_eq!(ctx.frames(), frames);
                    assert!(ctx.time().is_some());
                    let mut v = Vec::new();
                    for i in 0..ctx.num_inputs() {
                        v.push(ctx.input(i)[0]);
                    }
                    for i in 0..ctx.num_outputs() {
                        let out = ctx.output(i);
                        for s in out.iter_mut() {
                            *s = 0.25;
                        }
                    }
                    seen_cb.lock().unwrap().push(v);
                },
            )
            .unwrap();
        assert_eq!(stream.buffer_size, frames);

        // Probe results: the fake asked the host about time info and
        // outputReady during createBuffers.
        assert!(fake.host_supports_time_info.get());
        assert!(fake.host_supports_output_ready.get());

        stream.start().unwrap();
        assert!(fake.started.get());
        let output_ready_baseline = fake.output_ready_count.get();

        // Engine writes known input data, then pumps both halves.
        for half in 0..2 {
            let vals = [1.0f32 + half as f32, -0.5];
            for (ch, &val) in vals.iter().enumerate() {
                let raw_ptr = fake.raw_buffer(ch, half);
                unsafe { (raw_ptr as *mut [u8; 4]).write_unaligned(val.to_le_bytes()) };
            }
            fake.pump(half as i32);
        }
        stream.stop().unwrap();
        assert!(!fake.started.get());

        // The callback saw the injected inputs for both halves.
        let seen = seen.lock().unwrap();
        assert_eq!(*seen, vec![vec![1.0, -0.5], vec![2.0, -0.5]]);
        drop(seen);

        // The callback's output landed in the driver's raw buffers, and
        // outputReady was invoked once per switch.
        for half in 0..2 {
            let ptr0 = fake.raw_buffer(2, half);
            let b = unsafe { (ptr0 as *const [u8; 4]).read_unaligned() };
            assert_eq!(f32::from_le_bytes(b), 0.25);
        }
        assert_eq!(fake.output_ready_count.get(), output_ready_baseline + 2);
        drop(stream);

        // Drop disposed the buffers and released the global slot.
        assert_eq!(fake.dispose_count.get(), 1);
    }

    #[test]
    fn classic_buffer_switch_fallback() {
        let _lock = STREAM_LOCK.lock().unwrap();
        let mut fake = FakeDriver::new();
        fake.supports_time_info = false;
        let raw = (&mut *fake) as *mut FakeDriver as *mut Iasio;
        let mut driver = unsafe { Driver::from_raw(raw) };
        let switches = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let switches_cb = switches.clone();
        let mut stream = driver
            .stream(
                &StreamConfig {
                    sample_rate: 48_000.0,
                    buffer_size: 32,
                    num_inputs: 0,
                    num_outputs: 1,
                },
                move |ctx| {
                    assert!(ctx.time().is_none());
                    *switches_cb.lock().unwrap() += 1;
                },
            )
            .unwrap();
        // The driver does not offer time info, so the classic
        // bufferSwitch path is used (asserted inside the callback).
        stream.start().unwrap();
        fake.pump(0);
        fake.pump(1);
        assert_eq!(*switches.lock().unwrap(), 2);
    }

    #[test]
    fn rejects_unsupported_sample_type() {
        let _lock = STREAM_LOCK.lock().unwrap();
        let mut fake = FakeDriver::new();
        fake.channels[0].sample_type = SampleType::Float64Lsb.raw();
        let raw = (&mut *fake) as *mut FakeDriver as *mut Iasio;
        let mut driver = unsafe { Driver::from_raw(raw) };
        let err = driver
            .stream(
                &StreamConfig {
                    sample_rate: 48_000.0,
                    buffer_size: 64,
                    num_inputs: 1,
                    num_outputs: 0,
                },
                |_| {},
            )
            .unwrap_err();
        assert!(matches!(err, StreamError::UnsupportedSampleType(_)));
    }

    #[test]
    fn busy_when_two_streams_at_once() {
        let _lock = STREAM_LOCK.lock().unwrap();
        let mut fake_a = FakeDriver::new();
        let raw_a = (&mut *fake_a) as *mut FakeDriver as *mut Iasio;
        let mut driver_a = unsafe { Driver::from_raw(raw_a) };
        let mut fake_b = FakeDriver::new();
        let raw_b = (&mut *fake_b) as *mut FakeDriver as *mut Iasio;
        let mut driver_b = unsafe { Driver::from_raw(raw_b) };

        let cfg = StreamConfig {
            sample_rate: 48_000.0,
            buffer_size: 64,
            num_inputs: 0,
            num_outputs: 1,
        };
        let first = driver_a.stream(&cfg, |_| {}).unwrap();
        let err = driver_b.stream(&cfg, |_| {}).unwrap_err();
        assert!(matches!(err, StreamError::Busy));
        drop(first);
        // After the first stream is dropped, the slot is free again.
        let second = driver_b.stream(&cfg, |_| {}).unwrap();
        drop(second);
    }
}
