//! Opened ASIO driver: identification, channel inventory, and
//! stream-parameter negotiation.
//!
//! This is where the host-side state machine lives (see the crate-level
//! docs). The driver is consumed by value and is neither `Send` nor
//! `Sync`: ASIO objects are bound to the COM apartment (and usually the
//! thread) that created them.

use core::ffi::c_void;
use core::marker::PhantomData;

use crate::abi::{self, ASIO_FALSE, ASIO_TRUE, AsioBool, AsioChannelInfo, Iasio, IasioVtbl};
use crate::error::{AsioError, check};

/// Sample encoding of a channel (`ASIOSampleType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleType {
    /// 16-bit integer, big-endian.
    Int16Msb,
    /// 24-bit integer, big-endian.
    Int24Msb,
    /// 32-bit integer, big-endian.
    Int32Msb,
    /// 32-bit float, big-endian.
    Float32Msb,
    /// 64-bit float, big-endian.
    Float64Msb,
    /// 32-bit container holding 16-bit data, big-endian.
    Int32Msb16,
    /// 32-bit container holding 18-bit data, big-endian.
    Int32Msb18,
    /// 32-bit container holding 20-bit data, big-endian.
    Int32Msb20,
    /// 32-bit container holding 24-bit data, big-endian.
    Int32Msb24,
    /// 16-bit integer, little-endian.
    Int16Lsb,
    /// 24-bit integer, little-endian (3 bytes per sample, packed).
    Int24Lsb,
    /// 32-bit integer, little-endian.
    Int32Lsb,
    /// 32-bit float, little-endian.
    Float32Lsb,
    /// 64-bit float, little-endian.
    Float64Lsb,
    /// 32-bit container holding 16-bit data, little-endian.
    Int32Lsb16,
    /// 32-bit container holding 18-bit data, little-endian.
    Int32Lsb18,
    /// 32-bit container holding 20-bit data, little-endian.
    Int32Lsb20,
    /// 32-bit container holding 24-bit data, little-endian.
    Int32Lsb24,
    /// A tag not defined by the public ABI (forward compatibility).
    Unknown(i32),
}

impl SampleType {
    /// Raw `ASIOST_*` tag.
    pub fn raw(self) -> i32 {
        match self {
            Self::Int16Msb => abi::ASIOST_INT16_MSB,
            Self::Int24Msb => abi::ASIOST_INT24_MSB,
            Self::Int32Msb => abi::ASIOST_INT32_MSB,
            Self::Float32Msb => abi::ASIOST_FLOAT32_MSB,
            Self::Float64Msb => abi::ASIOST_FLOAT64_MSB,
            Self::Int32Msb16 => abi::ASIOST_INT32_MSB16,
            Self::Int32Msb18 => abi::ASIOST_INT32_MSB18,
            Self::Int32Msb20 => abi::ASIOST_INT32_MSB20,
            Self::Int32Msb24 => abi::ASIOST_INT32_MSB24,
            Self::Int16Lsb => abi::ASIOST_INT16_LSB,
            Self::Int24Lsb => abi::ASIOST_INT24_LSB,
            Self::Int32Lsb => abi::ASIOST_INT32_LSB,
            Self::Float32Lsb => abi::ASIOST_FLOAT32_LSB,
            Self::Float64Lsb => abi::ASIOST_FLOAT64_LSB,
            Self::Int32Lsb16 => abi::ASIOST_INT32_LSB16,
            Self::Int32Lsb18 => abi::ASIOST_INT32_LSB18,
            Self::Int32Lsb20 => abi::ASIOST_INT32_LSB20,
            Self::Int32Lsb24 => abi::ASIOST_INT32_LSB24,
            Self::Unknown(raw) => raw,
        }
    }

    /// Bytes per sample in the driver's buffers; `None` for container
    /// formats whose payload width is ambiguous without the tag.
    pub fn size_bytes(self) -> Option<usize> {
        Some(match self {
            Self::Int16Msb | Self::Int16Lsb => 2,
            Self::Int24Msb | Self::Int24Lsb => 3,
            Self::Int32Msb
            | Self::Float32Msb
            | Self::Int32Msb16
            | Self::Int32Msb18
            | Self::Int32Msb20
            | Self::Int32Msb24
            | Self::Int32Lsb
            | Self::Float32Lsb
            | Self::Int32Lsb16
            | Self::Int32Lsb18
            | Self::Int32Lsb20
            | Self::Int32Lsb24 => 4,
            Self::Float64Msb | Self::Float64Lsb => 8,
            Self::Unknown(_) => return None,
        })
    }
}

impl From<i32> for SampleType {
    fn from(raw: i32) -> Self {
        match raw {
            abi::ASIOST_INT16_MSB => Self::Int16Msb,
            abi::ASIOST_INT24_MSB => Self::Int24Msb,
            abi::ASIOST_INT32_MSB => Self::Int32Msb,
            abi::ASIOST_FLOAT32_MSB => Self::Float32Msb,
            abi::ASIOST_FLOAT64_MSB => Self::Float64Msb,
            abi::ASIOST_INT32_MSB16 => Self::Int32Msb16,
            abi::ASIOST_INT32_MSB18 => Self::Int32Msb18,
            abi::ASIOST_INT32_MSB20 => Self::Int32Msb20,
            abi::ASIOST_INT32_MSB24 => Self::Int32Msb24,
            abi::ASIOST_INT16_LSB => Self::Int16Lsb,
            abi::ASIOST_INT24_LSB => Self::Int24Lsb,
            abi::ASIOST_INT32_LSB => Self::Int32Lsb,
            abi::ASIOST_FLOAT32_LSB => Self::Float32Lsb,
            abi::ASIOST_FLOAT64_LSB => Self::Float64Lsb,
            abi::ASIOST_INT32_LSB16 => Self::Int32Lsb16,
            abi::ASIOST_INT32_LSB18 => Self::Int32Lsb18,
            abi::ASIOST_INT32_LSB20 => Self::Int32Lsb20,
            abi::ASIOST_INT32_LSB24 => Self::Int32Lsb24,
            other => Self::Unknown(other),
        }
    }
}

/// Buffer-size constraints reported by the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferSizes {
    /// Minimum buffer size in samples.
    pub min: i32,
    /// Maximum buffer size in samples.
    pub max: i32,
    /// Driver-preferred buffer size in samples.
    pub preferred: i32,
    /// Buffer granularity: -1 means doubling sizes from min, 0 selects
    /// the fixed preferred size, and a positive value is the step from min.
    pub granularity: i32,
}

impl BufferSizes {
    /// Whether any size between `min` and `max` is valid.
    pub fn is_continuous(self) -> bool {
        self.granularity == 1
    }

    /// Whether only power-of-two sizes are valid.
    pub fn is_power_of_two_only(self) -> bool {
        self.granularity == -1
    }
}

/// Description of one channel, filled in by the driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelInfo {
    /// Channel index within its direction.
    pub index: i32,
    /// Input or output.
    pub is_input: bool,
    /// Whether the channel is active (always true before buffers exist,
    /// per the ABI; drivers may deactivate unsupported channels during
    /// `createBuffers`).
    pub is_active: bool,
    /// Channel group, or -1 when the driver does not group channels.
    pub group: i32,
    /// Sample encoding of the channel's buffers.
    pub sample_type: SampleType,
    /// Channel name.
    pub name: String,
}

/// Failure to instantiate a driver.
#[cfg(windows)]
#[derive(Debug)]
pub enum OpenError {
    /// COM instantiation or interface query failed.
    Com(windows::core::Error),
    /// The driver loaded but rejected initialization.
    Init(AsioError),
}

#[cfg(windows)]
impl core::fmt::Display for OpenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Com(e) => write!(f, "COM error: {e}"),
            Self::Init(e) => write!(f, "driver init failed: {e}"),
        }
    }
}

#[cfg(windows)]
impl std::error::Error for OpenError {}

/// An opened ASIO driver.
///
/// Obtained with `Driver::open` (from a registry descriptor discovered
/// via [`crate::drivers`]) or, in tests, from a raw pointer.
/// Not `Send`/`Sync`: ASIO objects are bound to the COM apartment (and
/// usually the thread) that created them.
pub struct Driver {
    this: *mut Iasio,
    /// Owned COM reference; `None` for borrowed (test-only) handles, where
    /// the caller keeps the object alive.
    #[cfg(windows)]
    _owned: Option<crate::com::DriverPtr>,
    _not_send_sync: PhantomData<*mut c_void>,
}

#[cfg(windows)]
impl Driver {
    fn new(this: *mut Iasio, owned: Option<crate::com::DriverPtr>) -> Self {
        Self {
            this,
            _owned: owned,
            _not_send_sync: PhantomData,
        }
    }
}

#[cfg(not(windows))]
#[cfg(test)]
impl Driver {
    fn new(this: *mut Iasio) -> Self {
        Self {
            this,
            _not_send_sync: PhantomData,
        }
    }
}

impl Driver {
    /// Instantiate and initialize the driver described by `info`.
    ///
    /// `guard` must be a live `ComGuard` (see the `com` module) for the
    /// current thread; the returned driver borrows the thread's apartment.
    ///
    /// Initialization passes a null window handle; some drivers require a
    /// valid HWND (see the design plan) — this is revisited when such a
    /// driver is tested.
    #[cfg(windows)]
    pub fn open(
        info: &crate::registry::DriverInfo,
        guard: &crate::com::ComGuard,
    ) -> Result<Self, OpenError> {
        let _ = guard;
        let owned = crate::com::create_driver(info).map_err(OpenError::Com)?;
        // SAFETY: `owned` is a live COM reference queried for IASIO, so
        // the raw pointer is a valid IASIO object for `owned`'s lifetime,
        // which covers the returned driver.
        let this = unsafe { owned.as_asio() } as *const Iasio as *mut Iasio;
        let mut driver = Self::new(this, Some(owned));
        driver.init().map_err(OpenError::Init)?;
        Ok(driver)
    }

    /// Wrap a raw `IASIO` pointer without taking ownership.
    ///
    /// # Safety
    ///
    /// `this` must be valid for the lifetime of the returned driver, and
    /// must not be concurrently used elsewhere. The object is not
    /// released on drop.
    #[cfg(test)]
    pub(crate) unsafe fn from_raw(this: *mut Iasio) -> Self {
        #[cfg(windows)]
        {
            Self::new(this, None)
        }
        #[cfg(not(windows))]
        {
            Self::new(this)
        }
    }

    pub(crate) fn vtbl(&self) -> &IasioVtbl {
        // SAFETY: `this` is a live interface pointer (owned or borrowed);
        // the vtable pointer is valid for the object's lifetime.
        unsafe { (*self.this).lp_vtbl.as_ref().expect("null IASIO vtable") }
    }

    pub(crate) fn this(&self) -> *mut Iasio {
        self.this
    }

    /// Initialize the driver with a null system handle.
    ///
    /// `init` returns an `ASIOBool`, unlike the other calls: [`ASIO_TRUE`]
    /// means success. On failure, [`Driver::error_message`] usually
    /// carries a human-readable reason.
    pub fn init(&mut self) -> Result<(), AsioError> {
        // SAFETY: `this` is live; a null sysHandle is accepted by the ABI
        // (drivers that need a window are handled separately).
        let ok = unsafe { (self.vtbl().init)(self.this(), core::ptr::null_mut()) };
        if ok == ASIO_TRUE {
            Ok(())
        } else {
            Err(AsioError::InitFailed)
        }
    }

    /// Driver name as reported by the driver.
    pub fn driver_name(&self) -> String {
        let mut buf = [0u8; 32];
        // SAFETY: `this` is live; `buf` is a valid 32-byte output buffer.
        unsafe {
            (self.vtbl().get_driver_name)(self.this(), buf.as_mut_ptr());
        }
        cstr(&buf)
    }

    /// Driver-specific version code.
    pub fn driver_version(&self) -> i32 {
        // SAFETY: `this` is a live interface pointer.
        unsafe { (self.vtbl().get_driver_version)(self.this()) }
    }

    /// Human-readable error string for the most recent failure.
    pub fn error_message(&self) -> String {
        let mut buf = [0u8; 128];
        // SAFETY: `this` is live; `buf` is a valid 128-byte output buffer.
        unsafe {
            (self.vtbl().get_error_message)(self.this(), buf.as_mut_ptr());
        }
        cstr(&buf)
    }

    /// Number of input and output channels.
    pub fn channels(&self) -> Result<(i32, i32), AsioError> {
        let mut inputs = 0;
        let mut outputs = 0;
        // SAFETY: `this` is live; both pointers are writable.
        check(unsafe { (self.vtbl().get_channels)(self.this(), &mut inputs, &mut outputs) })?;
        Ok((inputs, outputs))
    }

    /// Input and output latency in samples.
    pub fn latencies(&self) -> Result<(i32, i32), AsioError> {
        let mut input = 0;
        let mut output = 0;
        // SAFETY: `this` is live; both pointers are writable.
        check(unsafe { (self.vtbl().get_latencies)(self.this(), &mut input, &mut output) })?;
        Ok((input, output))
    }

    /// Buffer-size constraints.
    pub fn buffer_sizes(&self) -> Result<BufferSizes, AsioError> {
        let mut min = 0;
        let mut max = 0;
        let mut preferred = 0;
        let mut granularity = 0;
        // SAFETY: `this` is live; all four pointers are writable.
        check(unsafe {
            (self.vtbl().get_buffer_size)(
                self.this(),
                &mut min,
                &mut max,
                &mut preferred,
                &mut granularity,
            )
        })?;
        Ok(BufferSizes {
            min,
            max,
            preferred,
            granularity,
        })
    }

    /// Whether the driver supports `rate` Hz.
    pub fn can_sample_rate(&self, rate: f64) -> bool {
        // SAFETY: `this` is a live interface pointer.
        let code = unsafe { (self.vtbl().can_sample_rate)(self.this(), rate) };
        code == abi::ASE_OK
    }

    /// Current sample rate in Hz.
    pub fn sample_rate(&self) -> Result<f64, AsioError> {
        let mut rate = 0.0;
        // SAFETY: `this` is live; `rate` is writable.
        check(unsafe { (self.vtbl().get_sample_rate)(self.this(), &mut rate) })?;
        Ok(rate)
    }

    /// Request a new sample rate.
    pub fn set_sample_rate(&mut self, rate: f64) -> Result<(), AsioError> {
        // SAFETY: `this` is a live interface pointer.
        check(unsafe { (self.vtbl().set_sample_rate)(self.this(), rate) })
    }

    /// Query the driver for one channel's description.
    pub fn channel_info(&self, index: i32, is_input: bool) -> Result<ChannelInfo, AsioError> {
        let mut info = AsioChannelInfo {
            channel: index,
            is_input: bool_asio(is_input),
            is_active: ASIO_FALSE,
            channel_group: 0,
            sample_type: 0,
            name: [0; 32],
        };
        // SAFETY: `this` is live; `info` is readable and writable for the
        // call duration.
        check(unsafe { (self.vtbl().get_channel_info)(self.this(), &mut info) })?;
        Ok(ChannelInfo {
            index: info.channel,
            is_input: info.is_input == ASIO_TRUE,
            is_active: info.is_active == ASIO_TRUE,
            group: info.channel_group,
            sample_type: SampleType::from(info.sample_type),
            name: cstr(&info.name),
        })
    }

    /// Start the streaming engine (see [`crate::stream::Stream`]).
    pub fn start(&mut self) -> Result<(), AsioError> {
        // SAFETY: `this` is a live interface pointer.
        check(unsafe { (self.vtbl().start)(self.this()) })
    }

    /// Stop the streaming engine.
    pub fn stop(&mut self) -> Result<(), AsioError> {
        // SAFETY: `this` is a live interface pointer.
        check(unsafe { (self.vtbl().stop)(self.this()) })
    }

    /// Release all buffers allocated by `createBuffers`.
    pub fn dispose_buffers(&mut self) -> Result<(), AsioError> {
        // SAFETY: `this` is a live interface pointer.
        check(unsafe { (self.vtbl().dispose_buffers)(self.this()) })
    }

    /// Open the driver's control panel.
    ///
    /// Most drivers require the stream to be stopped (or never created);
    /// some additionally require that the host passed a valid window
    /// handle to `init()` (this crate passes null — see `Driver::open`).
    /// The call blocks until the user closes the panel.
    pub fn control_panel(&mut self) -> Result<(), AsioError> {
        // SAFETY: `this` is a live interface pointer.
        check(unsafe { (self.vtbl().control_panel)(self.this()) })
    }

    /// Raw interface pointer for lower-level modules in this crate.
    pub(crate) fn raw(&self) -> *mut Iasio {
        self.this()
    }
}

/// Read a NUL-terminated string from a fixed buffer.
fn cstr(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

fn bool_asio(b: bool) -> AsioBool {
    if b { ASIO_TRUE } else { ASIO_FALSE }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeDriver;

    fn open_fake() -> (Box<FakeDriver>, Driver) {
        let mut fake = FakeDriver::new();
        // The owning box outlives the driver handle in every test.
        let raw = (&mut *fake) as *mut FakeDriver as *mut Iasio;
        // SAFETY: the owning box outlives the driver handle in every test.
        let driver = unsafe { Driver::from_raw(raw) };
        (fake, driver)
    }

    #[test]
    fn identity_queries() {
        let (fake, mut driver) = open_fake();
        driver.init().unwrap();
        assert!(fake.init_called.get());
        assert!(fake.init_handle.get().is_null());
        assert_eq!(driver.driver_name(), "FakeASIO");
        assert_eq!(driver.driver_version(), 0x0102);
        assert_eq!(driver.error_message(), "");
    }

    #[test]
    fn channel_counts_and_latencies() {
        let (_fake, driver) = open_fake();
        assert_eq!(driver.channels().unwrap(), (2, 2));
        assert_eq!(driver.latencies().unwrap(), (96, 96));
    }

    #[test]
    fn buffer_sizes() {
        let (_fake, driver) = open_fake();
        let sizes = driver.buffer_sizes().unwrap();
        assert_eq!(
            sizes,
            BufferSizes {
                min: 16,
                max: 2048,
                preferred: 512,
                granularity: -1,
            }
        );
        assert!(sizes.is_power_of_two_only());
        assert!(!sizes.is_continuous());
    }

    #[test]
    fn sample_rate_negotiation() {
        let (fake, mut driver) = open_fake();
        assert_eq!(driver.sample_rate().unwrap(), 48_000.0);
        assert!(driver.can_sample_rate(44_100.0));
        assert!(!driver.can_sample_rate(123.0));
        driver.set_sample_rate(96_000.0).unwrap();
        assert_eq!(driver.sample_rate().unwrap(), 96_000.0);
        assert_eq!(fake.current_rate(), 96_000.0);
        assert_eq!(driver.set_sample_rate(123.0), Err(AsioError::InvalidMode));
    }

    #[test]
    fn channel_info_describes_every_channel() {
        let (_fake, driver) = open_fake();
        let (inputs, outputs) = driver.channels().unwrap();
        for i in 0..inputs {
            let info = driver.channel_info(i, true).unwrap();
            assert_eq!(info.index, i);
            assert!(info.is_input);
            assert!(info.is_active);
            assert_eq!(info.sample_type, SampleType::Int16Lsb);
        }
        for i in 0..outputs {
            let info = driver.channel_info(i, false).unwrap();
            assert!(!info.is_input);
            assert_eq!(info.sample_type, SampleType::Int16Lsb);
        }
        assert_eq!(
            driver.channel_info(inputs, true),
            Err(AsioError::InvalidParameter)
        );
    }

    #[test]
    fn sample_type_round_trips() {
        for raw in [
            abi::ASIOST_INT16_MSB,
            abi::ASIOST_INT24_LSB,
            abi::ASIOST_FLOAT32_LSB,
            abi::ASIOST_FLOAT64_MSB,
            abi::ASIOST_INT32_LSB20,
        ] {
            assert_eq!(SampleType::from(raw).raw(), raw);
        }
        assert_eq!(SampleType::from(999), SampleType::Unknown(999));
        assert_eq!(SampleType::Int24Lsb.size_bytes(), Some(3));
        assert_eq!(SampleType::Float32Lsb.size_bytes(), Some(4));
        assert_eq!(SampleType::Unknown(1).size_bytes(), None);
    }
}
