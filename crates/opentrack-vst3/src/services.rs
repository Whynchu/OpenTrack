// SPDX-License-Identifier: Apache-2.0

//! Host services handed to a plug-in: edit gestures, dirty state, restart
//! notifications and editor resize requests.
//!
//! All methods may run on the control/UI thread. `performEdit` only publishes a
//! value into the lock-free parameter queue, so a plug-in editor can report
//! gestures while audio continues.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};

use vst3::Steinberg::Vst::{
    IComponentHandler, IComponentHandler2, IComponentHandler2Trait, IComponentHandlerTrait,
    ParamID, ParamValue,
};
use vst3::Steinberg::{
    FIDString, IPlugFrame, IPlugFrameTrait, IPlugView, IPlugViewTrait, TBool, ViewRect,
    kInvalidArgument, kResultFalse, kResultOk, tresult,
};
use vst3::{Class, ComWrapper};

#[cfg(target_os = "linux")]
use crate::host::LinuxRunLoop;
#[cfg(target_os = "linux")]
use vst3::Steinberg::Linux::{IEventHandler, IRunLoop, IRunLoopTrait, ITimerHandler};

use crate::parameter::PendingParameters;

/// State shared between the effect and the host service objects.
#[derive(Default)]
pub(crate) struct HostShared {
    pub(crate) parameters: PendingParameters,
    pub(crate) output_parameters: PendingParameters,
    pub(crate) dirty: AtomicBool,
    pub(crate) restart_flags: AtomicI32,
    pub(crate) editor_hwnd: AtomicIsize,
    /// The main editor view attached to `editor_hwnd`; sub-views and popup
    /// windows must not resize the host window.
    pub(crate) editor_view: AtomicIsize,
}

/// `IComponentHandler`, `IComponentHandler2` and `IPlugFrame` implementation.
pub(crate) struct HostServices {
    pub(crate) shared: Arc<HostShared>,
    #[cfg(target_os = "linux")]
    run_loop: Arc<LinuxRunLoop>,
}

impl HostServices {
    #[cfg(target_os = "linux")]
    pub(crate) fn new(shared: Arc<HostShared>, run_loop: Arc<LinuxRunLoop>) -> ComWrapper<Self> {
        ComWrapper::new(Self { shared, run_loop })
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn new(shared: Arc<HostShared>) -> ComWrapper<Self> {
        ComWrapper::new(Self { shared })
    }
}

#[cfg(target_os = "linux")]
type HostServiceInterfaces = (IComponentHandler, IComponentHandler2, IPlugFrame, IRunLoop);
#[cfg(not(target_os = "linux"))]
type HostServiceInterfaces = (IComponentHandler, IComponentHandler2, IPlugFrame);

impl Class for HostServices {
    type Interfaces = HostServiceInterfaces;
}

#[cfg(target_os = "linux")]
impl IRunLoopTrait for HostServices {
    unsafe fn registerEventHandler(&self, handler: *mut IEventHandler, fd: i32) -> tresult {
        unsafe { self.run_loop.register_event_handler(handler, fd) }
    }

    unsafe fn unregisterEventHandler(&self, handler: *mut IEventHandler) -> tresult {
        unsafe { self.run_loop.unregister_event_handler(handler) }
    }

    unsafe fn registerTimer(&self, handler: *mut ITimerHandler, milliseconds: u64) -> tresult {
        unsafe { self.run_loop.register_timer(handler, milliseconds) }
    }

    unsafe fn unregisterTimer(&self, handler: *mut ITimerHandler) -> tresult {
        unsafe { self.run_loop.unregister_timer(handler) }
    }
}

impl IComponentHandlerTrait for HostServices {
    unsafe fn beginEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    unsafe fn performEdit(&self, id: ParamID, value_normalized: ParamValue) -> tresult {
        if !value_normalized.is_finite() {
            return kInvalidArgument;
        }
        self.shared
            .parameters
            .push(id, value_normalized.clamp(0.0, 1.0));
        self.shared.dirty.store(true, Ordering::Release);
        kResultOk
    }

    unsafe fn endEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    unsafe fn restartComponent(&self, flags: i32) -> tresult {
        self.shared.restart_flags.fetch_or(flags, Ordering::AcqRel);
        kResultOk
    }
}

impl IComponentHandler2Trait for HostServices {
    unsafe fn setDirty(&self, state: TBool) -> tresult {
        self.shared.dirty.store(state != 0, Ordering::Release);
        kResultOk
    }

    unsafe fn requestOpenEditor(&self, _name: FIDString) -> tresult {
        // The application decides when to create the editor window; accepting
        // the request keeps plug-ins that call this from failing.
        kResultOk
    }

    unsafe fn startGroupEdit(&self) -> tresult {
        kResultOk
    }

    unsafe fn finishGroupEdit(&self) -> tresult {
        kResultOk
    }
}

impl IPlugFrameTrait for HostServices {
    unsafe fn resizeView(&self, view: *mut IPlugView, new_size: *mut ViewRect) -> tresult {
        if view.is_null() || new_size.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: VST3 passes a borrowed view pointer for the duration of this
        // call; we only borrow it as well.
        let Some(view) = (unsafe { vst3::ComRef::<IPlugView>::from_raw(view) }) else {
            return kInvalidArgument;
        };
        // SAFETY: the size pointer was validated non-null.
        let rect = unsafe { *new_size };
        let width = rect.right.wrapping_sub(rect.left);
        let height = rect.bottom.wrapping_sub(rect.top);
        if width <= 0 || height <= 0 || width > 16_384 || height > 16_384 {
            return kInvalidArgument;
        }
        let hwnd = self.shared.editor_hwnd.load(Ordering::Acquire);
        let main_view = self.shared.editor_view.load(Ordering::Acquire);
        // Popup/sub-views inside the editor may request a size for themselves;
        // only the main view that owns the host window may resize it.
        let owns_window = main_view == 0 || main_view == view.as_ptr() as isize;
        if owns_window
            && hwnd != 0
            && crate::editor::resize_native_window(hwnd, width as u32, height as u32).is_err()
        {
            return kResultFalse;
        }
        // SAFETY: `new_size` is valid for the duration of the call.
        unsafe { view.onSize(new_size) }
    }
}

impl HostShared {
    pub(crate) fn take_restart_flags(&self) -> i32 {
        self.restart_flags.swap(0, Ordering::AcqRel)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn plug_frame_exposes_the_linux_run_loop() {
        let services = HostServices::new(
            Arc::new(HostShared::default()),
            Arc::new(LinuxRunLoop::default()),
        );
        assert!(services.as_com_ref::<IPlugFrame>().is_some());
        assert!(services.as_com_ref::<IRunLoop>().is_some());
    }
}
