//! In-memory ASIO driver test double.
//!
//! Implements the [`IasioVtbl`] ABI in plain Rust so the host state
//! machine can be exercised without hardware, a COM apartment, or a real
//! driver DLL. The fake is intentionally configurable: tests set channel
//! counts, sample types, supported rates, buffer constraints, and can
//! observe which calls the host made.
//!
//! # Layout
//!
//! [`FakeDriver`] begins with a vtable pointer so that a `*mut Iasio`
//! obtained from [`FakeDriver::into_raw`] can be cast back to
//! `*mut FakeDriver`; this mirrors the COM object header layout.

#![cfg(test)]

use core::cell::{Cell, RefCell};
use core::ffi::c_void;

use crate::abi::{
    self, ASE_INVALID_MODE, ASE_INVALID_PARAMETER, ASE_NOT_PRESENT, ASE_OK, AsioBufferInfo,
    AsioCallbacks, AsioChannelInfo, AsioClockSource, AsioSamples, AsioTime, AsioTimeCode,
    AsioTimeInfo, AsioTimeStamp, IID_IASIO, Iasio, IasioVtbl,
};

/// `IID_IUnknown` (`{00000000-0000-0000-C000-000000000046}`).
const IID_IUNKNOWN: abi::AbiGuid = abi::AbiGuid {
    data1: 0x0000_0000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const S_OK: i32 = 0;
const E_NOINTERFACE: i32 = -0x7fffbffe;

/// One channel of the fake device.
#[derive(Clone, Debug)]
pub struct FakeChannel {
    /// Whether this is an input channel.
    pub is_input: bool,
    /// Sample type tag, one of the `ASIOST_*` constants.
    pub sample_type: i32,
    /// Channel name.
    pub name: String,
}

/// Scriptable ASIO driver for tests.
///
/// `repr(C)` is load-bearing: the vtable pointer must be the first field
/// so that the object can be viewed as an [`Iasio`] COM header.
#[repr(C)]
pub struct FakeDriver {
    vtbl: *const IasioVtbl,
    /// Driver name reported by `getDriverName`.
    pub name: String,
    /// Version reported by `getDriverVersion`.
    pub version: i32,
    /// Message reported by `getErrorMessage`.
    pub error_message: String,
    /// Number of input channels.
    pub num_inputs: i32,
    /// Number of output channels.
    pub num_outputs: i32,
    /// Input latency in samples.
    pub input_latency: i32,
    /// Output latency in samples.
    pub output_latency: i32,
    /// Buffer size constraints reported by `getBufferSize`.
    pub min_size: i32,
    /// Maximum buffer size.
    pub max_size: i32,
    /// Preferred buffer size.
    pub preferred_size: i32,
    /// Buffer granularity (-1 = doubling sizes, 0 = fixed, positive = step).
    pub granularity: i32,
    /// Sample rates the driver accepts.
    pub supported_rates: Vec<f64>,
    /// Channel descriptors, both inputs and outputs.
    pub channels: Vec<FakeChannel>,
    /// Whether `init` has been called.
    pub init_called: Cell<bool>,
    /// The window handle passed to `init`.
    pub init_handle: Cell<*mut c_void>,
    /// Whether the driver offers time info (selectable per test).
    pub supports_time_info: bool,
    /// Whether the driver accepts `outputReady()` (selectable per test).
    pub supports_output_ready: bool,
    /// Whether the engine is running (`start`/`stop`).
    pub started: Cell<bool>,
    /// How often the host called `outputReady()`.
    pub output_ready_count: Cell<u32>,
    /// How often the host called `disposeBuffers()`.
    pub dispose_count: Cell<u32>,
    /// What the host answered when probed for time-info support.
    pub host_supports_time_info: Cell<bool>,
    /// What the host answered when probed for `outputReady` support.
    pub host_supports_output_ready: Cell<bool>,
    /// Last buffer size passed to `createBuffers`.
    pub last_buffer_size: Cell<i32>,
    callbacks: RefCell<Option<AsioCallbacks>>,
    buffers: RefCell<Vec<Box<[u8]>>>,
    sample_position: Cell<i64>,
    current_rate: Cell<f64>,
    refcount: Cell<u32>,
}

impl FakeDriver {
    /// Create a stock 2-in/2-out 48 kHz device with 16-bit LSB channels.
    pub fn new() -> Box<Self> {
        let channels = (0..2)
            .map(|_| FakeChannel {
                is_input: true,
                sample_type: abi::ASIOST_INT16_LSB,
                name: "in".to_owned(),
            })
            .chain((0..2).map(|_| FakeChannel {
                is_input: false,
                sample_type: abi::ASIOST_INT16_LSB,
                name: "out".to_owned(),
            }))
            .collect();
        Box::new(Self {
            vtbl: &FAKE_VTBL,
            name: "FakeASIO".to_owned(),
            version: 0x0102,
            error_message: String::new(),
            num_inputs: 2,
            num_outputs: 2,
            input_latency: 96,
            output_latency: 96,
            min_size: 16,
            max_size: 2048,
            preferred_size: 512,
            granularity: -1,
            supported_rates: vec![44_100.0, 48_000.0, 96_000.0],
            channels,
            init_called: Cell::new(false),
            init_handle: Cell::new(core::ptr::null_mut()),
            supports_time_info: true,
            supports_output_ready: true,
            started: Cell::new(false),
            output_ready_count: Cell::new(0),
            dispose_count: Cell::new(0),
            host_supports_time_info: Cell::new(false),
            host_supports_output_ready: Cell::new(false),
            last_buffer_size: Cell::new(0),
            callbacks: RefCell::new(None),
            buffers: RefCell::new(Vec::new()),
            sample_position: Cell::new(0),
            current_rate: Cell::new(48_000.0),
            refcount: Cell::new(1),
        })
    }

    /// Current sample rate as the driver reports it.
    pub fn current_rate(&self) -> f64 {
        self.current_rate.get()
    }

    /// Act as the driver's engine for one buffer switch: invoke the host
    /// callback installed by `createBuffers`, advancing the sample
    /// position. Panics if buffers were not created.
    pub fn pump(&self, double_buffer_index: i32) {
        let cbs = *self
            .callbacks
            .borrow()
            .as_ref()
            .expect("pump: createBuffers was not called");
        let half = double_buffer_index as usize;
        debug_assert!(half < 2);
        if self.supports_time_info && self.host_supports_time_info.get() {
            let f = cbs
                .buffer_switch_time_info
                .expect("host must install bufferSwitchTimeInfo");
            let position = self.sample_position.get();
            let mut time = AsioTime {
                reserved: [0; 4],
                time_info: AsioTimeInfo {
                    speed: 1.0,
                    system_time: AsioSamples::default(),
                    sample_position: AsioSamples::from_i64(position),
                    flags: abi::K_SYSTEM_TIME_VALID | abi::K_SAMPLE_POSITION_VALID,
                    reserved: [0; 12],
                },
                time_code: AsioTimeCode {
                    speed: 1.0,
                    time_code_samples: AsioSamples::default(),
                    flags: 0,
                    reserved: [0; 64],
                },
            };
            // SAFETY: `time` is a valid AsioTime for the call; the
            // trampoline returns it (possibly modified).
            let returned = unsafe { f(&mut time, double_buffer_index, abi::ASIO_TRUE) };
            assert!(
                !returned.is_null(),
                "host returned null from bufferSwitchTimeInfo"
            );
            self.sample_position
                .set(position + self.last_buffer_size.get() as i64);
        } else {
            let f = cbs.buffer_switch.expect("host must install bufferSwitch");
            // SAFETY: plain ABI callback with a valid index.
            unsafe { f(double_buffer_index, abi::ASIO_TRUE) };
            self.sample_position
                .set(self.sample_position.get() + self.last_buffer_size.get() as i64);
        }
    }

    /// Raw pointer to an allocated half-buffer, in `createBuffers` order
    /// (inputs first, then outputs). For injecting input data and
    /// verifying output data in tests.
    ///
    /// # Panics
    ///
    /// Panics if buffers were not created or `ordinal`/`half` are out of
    /// range.
    pub fn raw_buffer(&self, ordinal: usize, half: usize) -> *mut u8 {
        let buffers = self.buffers.borrow();
        assert!(
            !buffers.is_empty(),
            "raw_buffer: createBuffers was not called"
        );
        buffers[ordinal * 2 + half].as_ptr() as *mut u8
    }

    /// Leak the box into a raw `*mut Iasio`. The pointer stays valid for
    /// the program duration (tests only); prefer keeping the owning box
    /// around instead.
    pub fn into_raw(self: Box<Self>) -> *mut Iasio {
        Box::into_raw(self).cast()
    }

    /// Borrow the fake back from a raw pointer obtained via
    /// [`FakeDriver::into_raw`].
    ///
    /// # Safety
    ///
    /// `raw` must have come from [`FakeDriver::into_raw`] and the owning
    /// box must still be alive.
    pub unsafe fn from_raw<'a>(raw: *mut Iasio) -> &'a Self {
        // SAFETY: guaranteed by the caller per the documented invariant;
        // the vtable pointer is the first field, so the layouts match.
        let fake = unsafe { &*raw.cast::<Self>() };
        debug_assert!(
            core::ptr::eq(fake.vtbl, &FAKE_VTBL),
            "IASIO pointer does not belong to a FakeDriver",
        );
        fake
    }
}

/// Copy `s` into `dst` as a NUL-terminated byte string with capacity
/// `cap` (including the NUL).
unsafe fn write_cstr(dst: *mut u8, s: &str, cap: usize) {
    let bytes = s.as_bytes();
    let n = bytes.len().min(cap.saturating_sub(1));
    // SAFETY: the caller guarantees `dst` is valid for `cap` writable
    // bytes; we write at most `cap - 1` bytes plus a NUL.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, n);
        *dst.add(n) = 0;
    }
}

// --- vtable entries -------------------------------------------------

unsafe extern "system" fn query_interface(
    this: *mut Iasio,
    iid: *const abi::AbiGuid,
    out: *mut *mut c_void,
) -> i32 {
    // SAFETY: `this` is a live fake object; `iid`/`out` follow COM rules.
    let matches = unsafe { *iid == IID_IASIO || *iid == IID_IUNKNOWN };
    if matches {
        // SAFETY: same object; handing out another reference.
        let fake = unsafe { FakeDriver::from_raw(this) };
        fake.refcount.set(fake.refcount.get() + 1);
        // SAFETY: `out` is valid for writes per COM rules.
        unsafe {
            *out = this.cast();
        }
        S_OK
    } else {
        // SAFETY: `out` must be set to null on failure.
        unsafe {
            *out = core::ptr::null_mut();
        }
        E_NOINTERFACE
    }
}

unsafe extern "system" fn add_ref(this: *mut Iasio) -> u32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    let n = fake.refcount.get() + 1;
    fake.refcount.set(n);
    n
}

unsafe extern "system" fn release(this: *mut Iasio) -> u32 {
    // SAFETY: `this` is a live fake object. We never free: the owning
    // test box keeps the memory alive for the program duration.
    let fake = unsafe { FakeDriver::from_raw(this) };
    let n = fake.refcount.get().saturating_sub(1);
    fake.refcount.set(n);
    n
}

unsafe extern "system" fn init(this: *mut Iasio, sys_handle: *mut c_void) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    fake.init_called.set(true);
    fake.init_handle.set(sys_handle);
    abi::ASIO_TRUE
}

unsafe extern "system" fn get_driver_name(this: *mut Iasio, name: *mut u8) {
    // SAFETY: `this` is a live fake object; the host passes a 32-byte
    // buffer per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    unsafe {
        write_cstr(name, &fake.name, 32);
    }
}

unsafe extern "system" fn get_driver_version(this: *mut Iasio) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    fake.version
}

unsafe extern "system" fn get_error_message(this: *mut Iasio, message: *mut u8) {
    // SAFETY: `this` is a live fake object; the host passes a 128-byte
    // buffer per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    unsafe {
        write_cstr(message, &fake.error_message, 128);
    }
}

unsafe extern "system" fn start(this: *mut Iasio) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    fake.started.set(true);
    ASE_OK
}

unsafe extern "system" fn stop(this: *mut Iasio) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    fake.started.set(false);
    ASE_OK
}

unsafe extern "system" fn get_channels(
    this: *mut Iasio,
    num_inputs: *mut i32,
    num_outputs: *mut i32,
) -> i32 {
    // SAFETY: `this` is live; output pointers are writable per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    unsafe {
        *num_inputs = fake.num_inputs;
        *num_outputs = fake.num_outputs;
    }
    ASE_OK
}

unsafe extern "system" fn get_latencies(
    this: *mut Iasio,
    input: *mut i32,
    output: *mut i32,
) -> i32 {
    // SAFETY: `this` is live; output pointers are writable per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    unsafe {
        *input = fake.input_latency;
        *output = fake.output_latency;
    }
    ASE_OK
}

unsafe extern "system" fn get_buffer_size(
    this: *mut Iasio,
    min_size: *mut i32,
    max_size: *mut i32,
    preferred_size: *mut i32,
    granularity: *mut i32,
) -> i32 {
    // SAFETY: `this` is live; output pointers are writable per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    unsafe {
        *min_size = fake.min_size;
        *max_size = fake.max_size;
        *preferred_size = fake.preferred_size;
        *granularity = fake.granularity;
    }
    ASE_OK
}

unsafe extern "system" fn can_sample_rate(this: *mut Iasio, rate: f64) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    if fake.supported_rates.contains(&rate) {
        ASE_OK
    } else {
        ASE_INVALID_MODE
    }
}

unsafe extern "system" fn get_sample_rate(this: *mut Iasio, rate: *mut f64) -> i32 {
    // SAFETY: `this` is live; `rate` is writable per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    unsafe {
        *rate = fake.current_rate.get();
    }
    ASE_OK
}

unsafe extern "system" fn set_sample_rate(this: *mut Iasio, rate: f64) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    if fake.supported_rates.contains(&rate) {
        fake.current_rate.set(rate);
        ASE_OK
    } else {
        ASE_INVALID_MODE
    }
}

unsafe extern "system" fn get_clock_sources(
    _this: *mut Iasio,
    _clocks: *mut AsioClockSource,
    _num_sources: *mut i32,
) -> i32 {
    ASE_NOT_PRESENT
}

unsafe extern "system" fn set_clock_source(_this: *mut Iasio, _reference: i32) -> i32 {
    ASE_OK
}

unsafe extern "system" fn get_sample_position(
    _this: *mut Iasio,
    position: *mut AsioSamples,
    timestamp: *mut AsioTimeStamp,
) -> i32 {
    // SAFETY: output pointers are writable per the ABI.
    unsafe {
        *position = AsioSamples::default();
        *timestamp = AsioTimeStamp::default();
    }
    ASE_OK
}

unsafe extern "system" fn get_channel_info(this: *mut Iasio, info: *mut AsioChannelInfo) -> i32 {
    // SAFETY: `this` is live; `info` is readable/writable per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    let info = unsafe { &mut *info };
    match fake
        .channels
        .iter()
        .filter(|c| c.is_input == (info.is_input == abi::ASIO_TRUE))
        .nth(info.channel as usize)
    {
        Some(ch) => {
            info.is_active = abi::ASIO_TRUE;
            info.channel_group = 0;
            info.sample_type = ch.sample_type;
            let name_bytes = ch.name.as_bytes();
            let n = name_bytes.len().min(info.name.len() - 1);
            info.name[..n].copy_from_slice(&name_bytes[..n]);
            info.name[n] = 0;
            ASE_OK
        }
        None => ASE_INVALID_PARAMETER,
    }
}

unsafe extern "system" fn create_buffers(
    this: *mut Iasio,
    buffer_infos: *mut AsioBufferInfo,
    num_channels: i32,
    buffer_size: i32,
    callbacks: *mut AsioCallbacks,
) -> i32 {
    // SAFETY: `this` is live; the pointer arguments are valid for the
    // documented sizes per the ABI.
    let fake = unsafe { FakeDriver::from_raw(this) };
    let infos = unsafe { core::slice::from_raw_parts_mut(buffer_infos, num_channels as usize) };
    let mut allocated = Vec::with_capacity(infos.len() * 2);
    for info in infos.iter_mut() {
        let sample_type = fake
            .channels
            .iter()
            .filter(|c| c.is_input == (info.is_input == abi::ASIO_TRUE))
            .nth(info.channel as usize)
            .map(|c| c.sample_type)
            .unwrap_or(abi::ASIOST_INT16_LSB);
        let bytes = sample_size(sample_type) * buffer_size.max(0) as usize;
        let mut first = vec![0u8; bytes].into_boxed_slice();
        let mut second = vec![0u8; bytes].into_boxed_slice();
        info.buffers[0] = first.as_mut_ptr().cast();
        info.buffers[1] = second.as_mut_ptr().cast();
        allocated.push(first);
        allocated.push(second);
    }
    *fake.callbacks.borrow_mut() = Some(unsafe { *callbacks });
    *fake.buffers.borrow_mut() = allocated;
    fake.last_buffer_size.set(buffer_size);

    // Probe the host like a real driver does during createBuffers.
    let cbs = *fake.callbacks.borrow().as_ref().unwrap();
    if let Some(msg) = cbs.asio_message {
        // SAFETY: null message/opt pointers are valid for these probes.
        fake.host_supports_time_info.set(
            unsafe {
                msg(
                    abi::K_ASIO_SELECTOR_SUPPORTED,
                    abi::K_ASIO_SUPPORTS_TIME_INFO,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                )
            } == abi::ASIO_TRUE,
        );
        fake.host_supports_output_ready.set(
            unsafe {
                msg(
                    abi::K_ASIO_SELECTOR_SUPPORTED,
                    abi::K_ASIO_SUPPORTS_OUTPUT_READY,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                )
            } == abi::ASIO_TRUE,
        );
    }
    ASE_OK
}

unsafe extern "system" fn dispose_buffers(this: *mut Iasio) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    fake.buffers.borrow_mut().clear();
    *fake.callbacks.borrow_mut() = None;
    fake.dispose_count.set(fake.dispose_count.get() + 1);
    ASE_OK
}

unsafe extern "system" fn control_panel(_this: *mut Iasio) -> i32 {
    ASE_OK
}

unsafe extern "system" fn future(_this: *mut Iasio, _selector: i32, _opt: *mut c_void) -> i32 {
    0
}

unsafe extern "system" fn output_ready(this: *mut Iasio) -> i32 {
    // SAFETY: `this` is a live fake object.
    let fake = unsafe { FakeDriver::from_raw(this) };
    if fake.supports_output_ready {
        fake.output_ready_count
            .set(fake.output_ready_count.get() + 1);
        ASE_OK
    } else {
        ASE_INVALID_MODE
    }
}

/// Bytes per sample for the formats the fake allocates.
fn sample_size(sample_type: i32) -> usize {
    match sample_type {
        abi::ASIOST_FLOAT32_MSB | abi::ASIOST_FLOAT32_LSB => 4,
        abi::ASIOST_FLOAT64_MSB | abi::ASIOST_FLOAT64_LSB => 8,
        abi::ASIOST_INT16_MSB | abi::ASIOST_INT16_LSB => 2,
        abi::ASIOST_INT24_MSB | abi::ASIOST_INT24_LSB => 3,
        abi::ASIOST_INT32_MSB..=abi::ASIOST_INT32_MSB24
        | abi::ASIOST_INT32_LSB..=abi::ASIOST_INT32_LSB24 => 4,
        _ => 2,
    }
}

static FAKE_VTBL: IasioVtbl = IasioVtbl {
    query_interface,
    add_ref,
    release,
    init,
    get_driver_name,
    get_driver_version,
    get_error_message,
    start,
    stop,
    get_channels,
    get_latencies,
    get_buffer_size,
    can_sample_rate,
    get_sample_rate,
    set_sample_rate,
    get_clock_sources,
    set_clock_source,
    get_sample_position,
    get_channel_info,
    create_buffers,
    dispose_buffers,
    control_panel,
    future,
    output_ready,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_reports_identity() {
        let fake = FakeDriver::new();
        let raw = fake.into_raw();
        let d = unsafe { FakeDriver::from_raw(raw) };
        assert_eq!(d.num_inputs, 2);
        assert_eq!(d.supported_rates, vec![44_100.0, 48_000.0, 96_000.0]);
    }
}
