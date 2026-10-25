//! COM initialization and ASIO driver instantiation.
//!
//! ASIO drivers are in-process COM servers registered by CLSID under
//! `HKLM\SOFTWARE\ASIO` (see [`crate::registry`]). This module owns the
//! apartment lifetime and the raw interface pointer.

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::core::{HRESULT, IUnknown, Interface};

use crate::abi::{IID_IASIO, Iasio};
use crate::registry::DriverInfo;
use crate::registry::imp::to_windows_guid;

/// RAII guard for the thread's COM apartment.
///
/// Obtained via [`initialize`]; calls `CoUninitialize` on drop. Each
/// thread that talks to COM needs its own guard; the guard is not `Send`.
pub struct ComGuard {
    _not_send: core::marker::PhantomData<*mut ()>,
}

/// Initialize COM as a single-threaded apartment for the current thread.
///
/// Returns an error if the thread already belongs to a different apartment
/// type (e.g. a multi-threaded apartment created by the embedding
/// application); in that case the embedder is expected to manage COM
/// itself.
pub fn initialize() -> Result<ComGuard, HRESULT> {
    // SAFETY: `CoInitializeEx` with a null reserved pointer is always
    // valid to call.
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    // S_OK: initialized. S_FALSE: already initialized on this thread; the
    // guard still balances it with CoUninitialize on drop.
    if hr.is_ok() || hr == HRESULT(1) {
        Ok(ComGuard {
            _not_send: core::marker::PhantomData,
        })
    } else {
        Err(hr)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: balances the successful CoInitializeEx in `initialize`.
        unsafe {
            CoUninitialize();
        }
    }
}

/// A COM reference to an instantiated ASIO driver.
///
/// The underlying interface pointer has been queried for [`IID_IASIO`],
/// so its vtable is an [`crate::abi::IasioVtbl`]. The reference is released
/// on drop (via the `windows` crate `IUnknown` wrapper).
pub struct DriverPtr {
    ptr: IUnknown,
}

impl DriverPtr {
    /// Raw view of the interface as an [`Iasio`] for vtable calls.
    ///
    /// # Safety
    ///
    /// The returned reference borrows `self`; the pointer is valid until
    /// `self` is dropped. Vtable methods must be called following the COM
    /// threading rules of the apartment that created the object.
    pub unsafe fn as_asio(&self) -> &Iasio {
        let raw = <IUnknown as Interface>::as_raw(&self.ptr);
        // SAFETY: `raw` is a live interface pointer whose vtable was
        // verified to be IASIO at construction; the header is a single
        // vtable pointer.
        unsafe { &*(raw as *const Iasio) }
    }
}

/// Instantiate an ASIO driver described by `info`.
///
/// The driver DLL is loaded in-process (`CLSCTX_INPROC_SERVER`), created
/// as `IUnknown`, and then queried for the `IASIO` interface. Most
/// drivers answer `QueryInterface` with the fixed [`IID_IASIO`]; some
/// drivers only accept their own coclass CLSID as the interface IID (the
/// approach JUCE uses), so that is tried as a fallback.
///
/// The returned [`DriverPtr`] must be dropped on the same thread that
/// created it (STA rules).
pub fn create_driver(info: &DriverInfo) -> windows::core::Result<DriverPtr> {
    let clsid = to_windows_guid(&info.clsid);
    // SAFETY: `clsid` is a valid class ID for the duration of the call;
    // no aggregation (null outer).
    let unknown = unsafe {
        CoCreateInstance::<Option<&IUnknown>, IUnknown>(&clsid, None, CLSCTX_INPROC_SERVER)?
    };
    // SAFETY: `unknown` is a live interface reference, the IID values are
    // valid for the calls, and `ptr` is valid for writes of an interface
    // pointer. QueryInterface follows COM rules.
    let iid = to_windows_guid(&IID_IASIO);
    let mut ptr: *mut core::ffi::c_void = core::ptr::null_mut();
    let hr = unsafe { (unknown.vtable().QueryInterface)(unknown.as_raw(), &iid, &mut ptr) };
    if hr.is_err() {
        // Fallback for drivers that key their IASIO interface on their
        // own CLSID instead of the ABI-wide IID.
        ptr = core::ptr::null_mut();
        unsafe { (unknown.vtable().QueryInterface)(unknown.as_raw(), &clsid, &mut ptr) }.ok()?;
    }
    debug_assert!(!ptr.is_null());
    // SAFETY: on success QueryInterface returned an owned reference to
    // the IASIO interface which we take over; IUnknown's Drop releases it.
    let iasio = unsafe { IUnknown::from_raw(ptr) };
    Ok(DriverPtr { ptr: iasio })
}
