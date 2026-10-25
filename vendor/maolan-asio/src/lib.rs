//! ASIO host support for Windows, in pure Rust.
//!
//! `maolan-asio` enumerates installed ASIO drivers, opens duplex streams
//! against them, and runs the buffer-switch callback loop — without the
//! Steinberg ASIO SDK in the dependency tree and without a C/C++ build
//! step. It is the Windows audio backend building block for the
//! [Maolan](https://maolan.rs) DAW, but is usable standalone.
//!
//! # Protocol overview
//!
//! ASIO drivers are in-process COM servers registered under
//! `HKLM\SOFTWARE\ASIO`. A host session looks like this:
//!
//! 1. **Discover** drivers with [`drivers`] ([`registry`]).
//! 2. **Initialize** COM on the thread (`com::initialize`) and open the
//!    driver with `Driver::open` ([`driver`]). Opening instantiates the
//!    COM object, queries it for the `IASIO` interface, and calls
//!    `init()`.
//! 3. **Negotiate** channel inventory, sample rate, and buffer size via
//!    the query methods on [`Driver`].
//! 4. **Open a stream** with `Driver::stream` ([`stream`]): allocates
//!    driver buffers, installs the callback table, and probes
//!    `outputReady()` support.
//! 5. **Start** the stream; the driver then invokes the callback on its
//!    buffer-switch thread for every half-buffer. Converted `f32`
//!    channel buffers are presented in [`CallbackContext`]; events such
//!    as reset requests accumulate in [`Stream::take_events`].
//! 6. **Drop** the stream: the engine stops and the buffers are
//!    released automatically.
//!
//! # Threading and real-time rules
//!
//! Everything happens on the thread that owns the COM apartment: the
//! [`Driver`] and [`Stream`] are neither `Send` nor `Sync`. The user
//! callback runs on the driver's buffer-switch thread and must not
//! allocate, block, or take locks for a significant time.
//!
//! Because the ABI's `ASIOCallbacks` table has no user-data pointer, only
//! one stream can be active per process (the same limitation as other
//! ASIO hosts); a second [`Driver::stream`] call returns
//! [`StreamError::Busy`].
//!
//! # Quick start
//!
//! ```no_run
//! # #[cfg(windows)]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use maolan_asio::{Driver, StreamConfig, drivers};
//!
//! let info = drivers()?.into_iter().next().expect("no ASIO drivers");
//! let guard = maolan_asio::com::initialize()
//!     .map_err(|e| std::io::Error::from_raw_os_error(e.0))?;
//! let mut driver = Driver::open(&info, &guard)?;
//! let config = StreamConfig::for_device(&driver, 48_000.0, 512)?;
//! let mut stream = driver.stream(&config, |mut ctx| {
//!     for ch in 0..ctx.num_outputs() {
//!         for sample in ctx.output(ch) {
//!             *sample = 0.0;
//!         }
//!     }
//! })?;
//! stream.start()?;
//! // ... run ...
//! stream.stop()?;
//! # Ok(())
//! # }
//! # #[cfg(not(windows))]
//! # fn main() {
//! #     println!("ASIO is only available on Windows");
//! # }
//! ```
//!
//! # Platform support
//!
//! Driver discovery and COM instantiation are Windows-only. The state
//! machine itself is platform-independent (it talks to a raw driver
//! vtable), so the crate compiles everywhere and its test suite runs on
//! any platform.
//!
//! # Testing policy
//!
//! No test in this repository requires audio hardware or a real ASIO
//! driver: tests install an in-memory fake driver behind the raw vtable.
//! See `TESTING.md` for details.
//!
//! # Provenance
//!
//! Interface definitions were derived from the publicly documented ASIO
//! ABI for the purpose of interoperability. This crate contains no
//! Steinberg SDK code. ASIO is a trademark of Steinberg Media
//! Technologies GmbH.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod abi;
#[cfg(windows)]
pub mod com;
mod convert;
pub mod driver;
pub mod error;
pub mod registry;
pub mod stream;

#[cfg(test)]
mod fake;

pub use driver::{BufferSizes, ChannelInfo, Driver, SampleType};
pub use error::{AsioError, StreamError};
pub use registry::{DriverInfo, drivers};
pub use stream::{CallbackContext, Stream, StreamConfig, StreamTime};
