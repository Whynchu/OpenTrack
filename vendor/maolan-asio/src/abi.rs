//! Raw ASIO ABI types: interface vtable, structures, constants, and
//! callback signatures.
//!
//! Everything in this module is a transcription of the publicly documented
//! ASIO ABI into `#[repr(C)]` Rust for the purpose of interoperability. See
//! the crate-level documentation for the provenance statement.

use core::ffi::c_void;
use core::fmt;

/// Boolean type used across the ABI (`ASIOBool`, a C `int`).
pub type AsioBool = i32;

/// `ASIOFalse`: negative boolean result.
pub const ASIO_FALSE: AsioBool = 0;

/// `ASIOTrue`: positive boolean result.
pub const ASIO_TRUE: AsioBool = 1;

/// Sample rate in Hz (`ASIOSampleRate`, a C `double`).
pub type SampleRate = f64;

/// COM-style GUID with the exact memory layout of `GUID` on Windows.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AbiGuid {
    /// First 4 bytes as a little-endian `u32`.
    pub data1: u32,
    /// Next 2 bytes as a little-endian `u16`.
    pub data2: u16,
    /// Next 2 bytes as a little-endian `u16`.
    pub data3: u16,
    /// Final 8 bytes, in order.
    pub data4: [u8; 8],
}

/// Interface identifier of the `IASIO` COM interface
/// (`{232685C6-6548-49D8-846D-4141A3EF7560}`).
///
/// Note: this is the fixed IID from the public ABI — the same GUID that
/// several drivers (e.g. ASIO4ALL) reuse as their coclass CLSID. Hosts
/// must also be prepared for drivers whose `QueryInterface` accepts only
/// their own CLSID; see `com::create_driver`.
pub const IID_IASIO: AbiGuid = AbiGuid {
    data1: 0x2326_85c6,
    data2: 0x6548,
    data3: 0x49d8,
    data4: [0x84, 0x6d, 0x41, 0x41, 0xa3, 0xef, 0x75, 0x60],
};

impl fmt::Display for AbiGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7],
        )
    }
}

impl fmt::Debug for AbiGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Success, not unique (`ASE_OK`, 0).
pub const ASE_OK: i32 = 0;

/// Success, unique (`ASE_SUCCESS`) — returned by some calls (e.g.
/// `setSampleRate`) to indicate the driver handled the request itself.
pub const ASE_SUCCESS: i32 = 0x3f48_47a0_u32 as i32;

/// Hardware input or output is not present or available (`ASE_NotPresent`).
pub const ASE_NOT_PRESENT: i32 = 1000;

/// Hardware malfunctioning (`ASE_HWMalfunction`).
pub const ASE_HW_MALFUNCTION: i32 = 1001;

/// Invalid input parameter (`ASE_InvalidParameter`).
pub const ASE_INVALID_PARAMETER: i32 = 1002;

/// Hardware is in a bad mode or used in a wrong mode (`ASE_InvalidMode`).
pub const ASE_INVALID_MODE: i32 = 1003;

/// Sample position not advancing (`ASE_SPNotAdvancing`).
pub const ASE_SP_NOT_ADVANCING: i32 = 1004;

/// Sample clock or rate cannot be determined (`ASE_NoClock`).
pub const ASE_NO_CLOCK: i32 = 1005;

/// Not enough memory for the requested operation (`ASE_NoMemory`).
pub const ASE_NO_MEMORY: i32 = 1006;

/// 16-bit integer, big-endian (`ASIOSTInt16MSB`).
pub const ASIOST_INT16_MSB: i32 = 0;

/// 24-bit integer, big-endian (`ASIOSTInt24MSB`).
pub const ASIOST_INT24_MSB: i32 = 1;

/// 32-bit integer, big-endian (`ASIOSTInt32MSB`).
pub const ASIOST_INT32_MSB: i32 = 2;

/// 32-bit float, big-endian (`ASIOSTFloat32MSB`).
pub const ASIOST_FLOAT32_MSB: i32 = 3;

/// 64-bit float, big-endian (`ASIOSTFloat64MSB`).
pub const ASIOST_FLOAT64_MSB: i32 = 4;

/// 32-bit container, 16-bit data in MSB (`ASIOSTInt32MSB16`).
pub const ASIOST_INT32_MSB16: i32 = 5;

/// 32-bit container, 18-bit data in MSB (`ASIOSTInt32MSB18`).
pub const ASIOST_INT32_MSB18: i32 = 6;

/// 32-bit container, 20-bit data in MSB (`ASIOSTInt32MSB20`).
pub const ASIOST_INT32_MSB20: i32 = 7;

/// 32-bit container, 24-bit data in MSB (`ASIOSTInt32MSB24`).
pub const ASIOST_INT32_MSB24: i32 = 8;

/// 16-bit integer, little-endian (`ASIOSTInt16LSB`).
pub const ASIOST_INT16_LSB: i32 = 16;

/// 24-bit integer, little-endian (`ASIOSTInt24LSB`).
pub const ASIOST_INT24_LSB: i32 = 17;

/// 32-bit integer, little-endian (`ASIOSTInt32LSB`).
pub const ASIOST_INT32_LSB: i32 = 18;

/// 32-bit float, little-endian (`ASIOSTFloat32LSB`).
pub const ASIOST_FLOAT32_LSB: i32 = 19;

/// 64-bit float, little-endian (`ASIOSTFloat64LSB`).
pub const ASIOST_FLOAT64_LSB: i32 = 20;

/// 32-bit container, 16-bit data in LSB (`ASIOSTInt32LSB16`).
pub const ASIOST_INT32_LSB16: i32 = 21;

/// 32-bit container, 18-bit data in LSB (`ASIOSTInt32LSB18`).
pub const ASIOST_INT32_LSB18: i32 = 22;

/// 32-bit container, 20-bit data in LSB (`ASIOSTInt32LSB20`).
pub const ASIOST_INT32_LSB20: i32 = 23;

/// 32-bit container, 24-bit data in LSB (`ASIOSTInt32LSB24`).
pub const ASIOST_INT32_LSB24: i32 = 24;

/// `asioMessage` selector: query whether a selector is supported
/// (`kAsioSelectorSupported`).
pub const K_ASIO_SELECTOR_SUPPORTED: i32 = 1;

/// `asioMessage` selector: request the driver's engine version
/// (`kAsioEngineVersion`); expected reply is [`K_ASIO_ENGINE_VERSION`].
pub const K_ASIO_ENGINE_VERSION: i32 = 2;

/// `asioMessage` selector: driver requests a host reset (`kAsioResetRequest`).
pub const K_ASIO_RESET_REQUEST: i32 = 3;

/// `asioMessage` selector: driver changed the buffer size
/// (`kAsioBufferSizeChange`).
pub const K_ASIO_BUFFER_SIZE_CHANGE: i32 = 4;

/// `asioMessage` selector: driver lost sync and wants a resync
/// (`kAsioResyncRequest`).
pub const K_ASIO_RESYNC_REQUEST: i32 = 5;

/// `asioMessage` selector: driver reports latencies changed
/// (`kAsioLatenciesChanged`).
pub const K_ASIO_LATENCIES_CHANGED: i32 = 6;

/// `asioMessage` selector: query time-info support (`kAsioSupportsTimeInfo`).
pub const K_ASIO_SUPPORTS_TIME_INFO: i32 = 7;

/// `asioMessage` selector: query time-code support (`kAsioSupportsTimeCode`).
pub const K_ASIO_SUPPORTS_TIME_CODE: i32 = 8;

/// `asioMessage` selector: MMC command from the driver (`kAsioMMCCommand`).
pub const K_ASIO_MMC_COMMAND: i32 = 9;

/// `asioMessage` selector: query input-monitor support
/// (`kAsioSupportsInputMonitor`).
pub const K_ASIO_SUPPORTS_INPUT_MONITOR: i32 = 10;

/// `asioMessage` selector: query input-gain support
/// (`kAsioSupportsInputGain`).
pub const K_ASIO_SUPPORTS_INPUT_GAIN: i32 = 11;

/// `asioMessage` selector: query input-meter support
/// (`kAsioSupportsInputMeter`).
pub const K_ASIO_SUPPORTS_INPUT_METER: i32 = 12;

/// `asioMessage` selector: query output-gain support
/// (`kAsioSupportsOutputGain`).
pub const K_ASIO_SUPPORTS_OUTPUT_GAIN: i32 = 13;

/// `asioMessage` selector: query output-meter support
/// (`kAsioSupportsOutputMeter`).
pub const K_ASIO_SUPPORTS_OUTPUT_METER: i32 = 14;

/// `asioMessage` selector: driver reports an overload (`kAsioOverload`).
pub const K_ASIO_OVERLOAD: i32 = 15;

/// One past the last valid `asioMessage` selector (`kAsioNumMessageSelectors`).
pub const K_ASIO_NUM_MESSAGE_SELECTORS: i32 = 16;

/// Pseudo-selector used with [`K_ASIO_SELECTOR_SUPPORTED`] to probe
/// `outputReady()` support (value `kAsioNumMessageSelectors`; there is no
/// dedicated `kAsioSupportsOutputReady` constant in the public ABI).
pub const K_ASIO_SUPPORTS_OUTPUT_READY: i32 = K_ASIO_NUM_MESSAGE_SELECTORS;

/// Engine version a host is expected to handle when queried with
/// [`K_ASIO_ENGINE_VERSION`].
pub const ASIO_ENGINE_VERSION: i32 = 2;

/// `ASIOTimeInfo` flag: `systemTime` is valid (`kSystemTimeValid`).
pub const K_SYSTEM_TIME_VALID: u32 = 1;

/// `ASIOTimeInfo` flag: `samplePosition` is valid (`kSamplePositionValid`).
pub const K_SAMPLE_POSITION_VALID: u32 = 1 << 1;

/// `ASIOTimeInfo` flag: `sampleType` field present (documented as
/// `kSampleTypeValid`; the field itself is reserved in the public ABI).
pub const K_SAMPLE_TYPE_VALID: u32 = 1 << 2;

/// `ASIOTimeCode` flag: time code is valid (`kTcValid`).
pub const K_TC_VALID: u32 = 1;

/// `ASIOTimeCode` flag: time code is running (`kTcRunning`).
pub const K_TC_RUNNING: u32 = 1 << 1;

/// `ASIOTimeCode` flag: time code is running backwards (`kTcReverse`).
pub const K_TC_REVERSE: u32 = 1 << 2;

/// `ASIOTimeCode` flag: time code is on speed (`kTcOnspeed`).
pub const K_TC_ONSPEED: u32 = 1 << 3;

/// `ASIOTimeCode` flag: time code is still (`kTcStill`).
pub const K_TC_STILL: u32 = 1 << 4;

/// `ASIOTimeCode` mask for the time-code source field (`kTcSource`).
pub const K_TC_SOURCE: u32 = 0x0f << 5;

/// 64-bit sample count (`ASIOSamples`): `hi` and `lo` combine to a 64-bit
/// value regardless of host endianness.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AsioSamples {
    /// High 32 bits.
    pub hi: i32,
    /// Low 32 bits.
    pub lo: u32,
}

impl AsioSamples {
    /// Combine into a host-order `i64`.
    pub fn to_i64(self) -> i64 {
        ((self.hi as i64) << 32) | (self.lo as i64)
    }

    /// Split a host-order `i64` into the ABI representation.
    pub fn from_i64(v: i64) -> Self {
        Self {
            hi: (v >> 32) as i32,
            lo: v as u32,
        }
    }
}

/// 64-bit nanosecond timestamp (`ASIOTimeStamp`), same split as
/// [`AsioSamples`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AsioTimeStamp {
    /// High 32 bits.
    pub hi: i32,
    /// Low 32 bits.
    pub lo: u32,
}

/// SMPTE time code (`ASIOTimeCode`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AsioTimeCode {
    /// Speed of the time-code source relative to nominal.
    pub speed: f64,
    /// Time in samples.
    pub time_code_samples: AsioSamples,
    /// Flags, see `K_TC_*` constants.
    pub flags: u32,
    /// Reserved, must be zero.
    pub reserved: [u8; 64],
}

/// Time info for a buffer switch (`ASIOTimeInfo`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AsioTimeInfo {
    /// Speed of the stream relative to nominal.
    pub speed: f64,
    /// System time in nanoseconds.
    pub system_time: AsioSamples,
    /// Stream position in samples.
    pub sample_position: AsioSamples,
    /// Flags, see `K_SYSTEM_TIME_VALID` and related constants.
    pub flags: u32,
    /// Reserved, must be zero.
    pub reserved: [u8; 12],
}

/// Combined time structure passed to and returned by
/// `bufferSwitchTimeInfo` (`ASIOTime`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AsioTime {
    /// Reserved, must be zero.
    pub reserved: [i32; 4],
    /// Time info portion.
    pub time_info: AsioTimeInfo,
    /// Time code portion.
    pub time_code: AsioTimeCode,
}

/// Description of one channel (`ASIOChannelInfo`). The caller fills
/// `channel`, `is_input`, and `channel_group`; the driver fills the rest.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AsioChannelInfo {
    /// Channel index.
    pub channel: i32,
    /// [`ASIO_TRUE`] for input channels, [`ASIO_FALSE`] for output.
    pub is_input: AsioBool,
    /// [`ASIO_TRUE`] if the channel is active after buffer creation.
    pub is_active: AsioBool,
    /// Channel group, or -1 if not grouped.
    pub channel_group: i32,
    /// Sample type, one of the `ASIOST_*` constants.
    pub sample_type: i32,
    /// Channel name, NUL-terminated, at most 31 bytes used.
    pub name: [u8; 32],
}

/// Buffer pair for one channel (`ASIOBufferInfo`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AsioBufferInfo {
    /// [`ASIO_TRUE`] for input, [`ASIO_FALSE`] for output.
    pub is_input: AsioBool,
    /// Channel index.
    pub channel: i32,
    /// The two half-buffers, filled by `createBuffers`.
    pub buffers: [*mut c_void; 2],
}

/// Available clock source (`ASIOClockSource`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AsioClockSource {
    /// Source index.
    pub index: i32,
    /// Associated channel, or -1.
    pub associated_channel: i32,
    /// Associated channel group.
    pub associated_group: f64,
    /// [`ASIO_TRUE`] if this is the current source.
    pub is_current_source: AsioBool,
    /// Source name, NUL-terminated, at most 31 bytes used.
    pub name: [u8; 32],
}

/// Callback invoked by the driver on the buffer-switch thread
/// (`bufferSwitch`).
pub type BufferSwitchFn = unsafe extern "C" fn(double_buffer_index: i32, direct_process: AsioBool);

/// Callback invoked when the sample rate changes (`sampleRateDidChange`).
pub type SampleRateDidChangeFn = unsafe extern "C" fn(sample_rate: SampleRate);

/// Callback for driver-to-host messages and capability queries
/// (`asioMessage`); returns a selector-specific value.
pub type AsioMessageFn =
    unsafe extern "C" fn(selector: i32, value: i32, message: *mut c_void, opt: *mut f64) -> i32;

/// Extended buffer-switch callback carrying time info
/// (`bufferSwitchTimeInfo`); returns the (possibly updated) params pointer.
pub type BufferSwitchTimeInfoFn = unsafe extern "C" fn(
    params: *mut AsioTime,
    double_buffer_index: i32,
    direct_process: AsioBool,
) -> *mut AsioTime;

/// Callback table installed by the host via `createBuffers`
/// (`ASIOCallbacks`). Any entry except `asio_message` may be null if the
/// host does not use it; hosts typically install all four.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AsioCallbacks {
    /// Classic buffer-switch notification.
    pub buffer_switch: Option<BufferSwitchFn>,
    /// Sample-rate change notification.
    pub sample_rate_did_change: Option<SampleRateDidChangeFn>,
    /// Driver-to-host message dispatcher; must be installed.
    pub asio_message: Option<AsioMessageFn>,
    /// Buffer-switch notification with time info.
    pub buffer_switch_time_info: Option<BufferSwitchTimeInfoFn>,
}

/// COM vtable of the `IASIO` interface, `IUnknown` methods first, in
/// declaration order.
///
/// # Safety
///
/// Function pointers in this table are called by the host with `this`
/// pointing to an [`Iasio`] whose `lp_vtbl` references this table. The
/// driver-implemented functions must follow COM ABI rules (preserve
/// registers, return `HRESULT`-style `i32` codes).
#[repr(C)]
pub struct IasioVtbl {
    /// `IUnknown::QueryInterface`.
    pub query_interface: unsafe extern "system" fn(
        this: *mut Iasio,
        iid: *const AbiGuid,
        out: *mut *mut c_void,
    ) -> i32,
    /// `IUnknown::AddRef`.
    pub add_ref: unsafe extern "system" fn(this: *mut Iasio) -> u32,
    /// `IUnknown::Release`.
    pub release: unsafe extern "system" fn(this: *mut Iasio) -> u32,
    /// Initialize the driver with the host's window handle (may be
    /// null). Unlike the other methods, returns [`ASIO_TRUE`] on success
    /// and [`ASIO_FALSE`] on failure (`ASIOBool`, not `ASIOError`).
    pub init: unsafe extern "system" fn(this: *mut Iasio, sys_handle: *mut c_void) -> i32,
    /// Write the driver name (up to 31 bytes plus NUL) into `name`.
    pub get_driver_name: unsafe extern "system" fn(this: *mut Iasio, name: *mut u8),
    /// Driver version, driver-specific encoding.
    pub get_driver_version: unsafe extern "system" fn(this: *mut Iasio) -> i32,
    /// Write a human-readable error string (up to 127 bytes plus NUL).
    pub get_error_message: unsafe extern "system" fn(this: *mut Iasio, message: *mut u8),
    /// Start the streaming engine.
    pub start: unsafe extern "system" fn(this: *mut Iasio) -> i32,
    /// Stop the streaming engine.
    pub stop: unsafe extern "system" fn(this: *mut Iasio) -> i32,
    /// Report channel counts into `num_input_channels` / `num_output_channels`.
    pub get_channels: unsafe extern "system" fn(
        this: *mut Iasio,
        num_input_channels: *mut i32,
        num_output_channels: *mut i32,
    ) -> i32,
    /// Report input and output latency in samples.
    pub get_latencies: unsafe extern "system" fn(
        this: *mut Iasio,
        input_latency: *mut i32,
        output_latency: *mut i32,
    ) -> i32,
    /// Report min/max/preferred buffer size and granularity (0 = power of
    /// two only, -1 = continuous).
    pub get_buffer_size: unsafe extern "system" fn(
        this: *mut Iasio,
        min_size: *mut i32,
        max_size: *mut i32,
        preferred_size: *mut i32,
        granularity: *mut i32,
    ) -> i32,
    /// Probe whether `sample_rate` is supported.
    pub can_sample_rate:
        unsafe extern "system" fn(this: *mut Iasio, sample_rate: SampleRate) -> i32,
    /// Write the current sample rate into `sample_rate`.
    pub get_sample_rate:
        unsafe extern "system" fn(this: *mut Iasio, sample_rate: *mut SampleRate) -> i32,
    /// Set the sample rate.
    pub set_sample_rate:
        unsafe extern "system" fn(this: *mut Iasio, sample_rate: SampleRate) -> i32,
    /// Enumerate clock sources; returns `ASE_NOT_PRESENT` if unsupported.
    pub get_clock_sources: unsafe extern "system" fn(
        this: *mut Iasio,
        clocks: *mut AsioClockSource,
        num_sources: *mut i32,
    ) -> i32,
    /// Select the clock source by index.
    pub set_clock_source: unsafe extern "system" fn(this: *mut Iasio, reference: i32) -> i32,
    /// Write the current sample position and system timestamp.
    pub get_sample_position: unsafe extern "system" fn(
        this: *mut Iasio,
        position: *mut AsioSamples,
        timestamp: *mut AsioTimeStamp,
    ) -> i32,
    /// Fill in channel information for the requested channel.
    pub get_channel_info:
        unsafe extern "system" fn(this: *mut Iasio, info: *mut AsioChannelInfo) -> i32,
    /// Allocate buffers and install the callback table.
    pub create_buffers: unsafe extern "system" fn(
        this: *mut Iasio,
        buffer_infos: *mut AsioBufferInfo,
        num_channels: i32,
        buffer_size: i32,
        callbacks: *mut AsioCallbacks,
    ) -> i32,
    /// Release all buffers.
    pub dispose_buffers: unsafe extern "system" fn(this: *mut Iasio) -> i32,
    /// Open the driver's control panel.
    pub control_panel: unsafe extern "system" fn(this: *mut Iasio) -> i32,
    /// Driver-specific future extension entry point.
    pub future: unsafe extern "system" fn(this: *mut Iasio, selector: i32, opt: *mut c_void) -> i32,
    /// Signal that output buffers are ready; only after the driver claims
    /// support via `kAsioSupportsOutputReady`.
    pub output_ready: unsafe extern "system" fn(this: *mut Iasio) -> i32,
}

/// COM object header: a pointer to the [`IasioVtbl`] vtable.
#[repr(C)]
pub struct Iasio {
    /// Pointer to the vtable; the first three entries are `IUnknown`.
    pub lp_vtbl: *const IasioVtbl,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn guid_layout() {
        assert_eq!(size_of::<AbiGuid>(), 16);
        assert_eq!(
            IID_IASIO.to_string(),
            "232685C6-6548-49D8-846D-4141A3EF7560"
        );
    }

    #[test]
    fn time_layout() {
        assert_eq!(size_of::<AsioSamples>(), 8);
        assert_eq!(size_of::<AsioTimeStamp>(), 8);
        assert_eq!(size_of::<AsioTimeInfo>(), 40);
        assert_eq!(size_of::<AsioTimeCode>(), 88);
        assert_eq!(size_of::<AsioTime>(), 144);
        assert_eq!(offset_of!(AsioTime, time_info), 16);
        assert_eq!(offset_of!(AsioTime, time_code), 56);
    }

    #[test]
    fn channel_info_layout() {
        assert_eq!(size_of::<AsioChannelInfo>(), 52);
        assert_eq!(offset_of!(AsioChannelInfo, is_input), 4);
        assert_eq!(offset_of!(AsioChannelInfo, is_active), 8);
        assert_eq!(offset_of!(AsioChannelInfo, channel_group), 12);
        assert_eq!(offset_of!(AsioChannelInfo, sample_type), 16);
        assert_eq!(offset_of!(AsioChannelInfo, name), 20);
    }

    #[test]
    fn clock_source_layout() {
        assert_eq!(size_of::<AsioClockSource>(), 56);
        assert_eq!(offset_of!(AsioClockSource, associated_channel), 4);
        assert_eq!(offset_of!(AsioClockSource, associated_group), 8);
        assert_eq!(offset_of!(AsioClockSource, is_current_source), 16);
        assert_eq!(offset_of!(AsioClockSource, name), 20);
    }

    #[test]
    fn buffer_info_layout() {
        assert_eq!(offset_of!(AsioBufferInfo, is_input), 0);
        assert_eq!(offset_of!(AsioBufferInfo, channel), 4);
        assert_eq!(offset_of!(AsioBufferInfo, buffers), 8);
        assert_eq!(size_of::<AsioBufferInfo>() % align_of::<*mut c_void>(), 0);
    }

    #[test]
    fn callbacks_layout() {
        assert_eq!(size_of::<AsioCallbacks>(), 4 * size_of::<usize>());
        assert_eq!(
            offset_of!(AsioCallbacks, sample_rate_did_change),
            size_of::<usize>()
        );
        assert_eq!(
            offset_of!(AsioCallbacks, asio_message),
            2 * size_of::<usize>()
        );
        assert_eq!(
            offset_of!(AsioCallbacks, buffer_switch_time_info),
            3 * size_of::<usize>()
        );
    }

    #[test]
    fn samples_join() {
        assert_eq!(AsioSamples { hi: 0, lo: 7 }.to_i64(), 7);
        assert_eq!(AsioSamples { hi: 1, lo: 0 }.to_i64(), 1 << 32);
        assert_eq!(
            AsioSamples {
                hi: -1,
                lo: 0xffff_ffff
            }
            .to_i64(),
            -1
        );
    }

    #[test]
    fn error_codes() {
        assert_eq!(ASE_OK, 0);
        assert_eq!(ASE_SUCCESS, 0x3f4847a0);
        // Error codes are positive; success codes are not.
        const {
            assert!(ASE_NOT_PRESENT > 0 && ASE_NO_MEMORY > ASE_NOT_PRESENT);
        }
    }
}
