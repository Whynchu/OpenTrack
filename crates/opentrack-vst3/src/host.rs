// SPDX-License-Identifier: Apache-2.0

//! Minimal VST3 host context.
//!
//! This is a control-thread skeleton. It implements `IHostApplication`,
//! `IMessage`, and `IAttributeList` so a component can be initialized later, but
//! it does not yet own a component or process audio. Attribute storage uses a
//! mutex and must never be called from the audio callback.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_void};
#[cfg(target_os = "linux")]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use vst3::Steinberg::Vst::{
    IAttributeList, IAttributeListTrait, IHostApplication, IHostApplicationTrait, IMessage,
    IMessageTrait, String128, TChar,
};
use vst3::Steinberg::{
    FIDString, FUnknown, TUID, int64, kInternalError, kInvalidArgument, kResultFalse, kResultOk,
    tresult, uint32,
};
use vst3::{Class, ComPtr, ComRef, ComWrapper, Interface};

#[cfg(target_os = "linux")]
use vst3::Steinberg::Linux::{
    IEventHandler, IEventHandlerTrait, IRunLoop, IRunLoopTrait, ITimerHandler, ITimerHandlerTrait,
};

const HOST_NAME: &str = "OpenTrack";

fn write_string128(destination: *mut String128, value: &str) {
    if destination.is_null() {
        return;
    }
    let destination = unsafe { &mut *destination };
    let mut length = 0usize;
    for unit in value
        .encode_utf16()
        .take(destination.len().saturating_sub(1))
    {
        destination[length] = unit as TChar;
        length += 1;
    }
    destination[length] = 0;
}

fn attribute_key(id: *const std::ffi::c_char) -> Option<Vec<u8>> {
    if id.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(id) }.to_bytes().to_vec())
}

unsafe fn read_utf16(value: *const TChar) -> Option<Vec<u16>> {
    if value.is_null() {
        return None;
    }
    let mut length = 0usize;
    while *value.add(length) != 0 {
        length += 1;
        if length > 65_536 {
            return None;
        }
    }
    Some(unsafe { std::slice::from_raw_parts(value, length) }.to_vec())
}

#[derive(Clone, Debug)]
enum AttributeValue {
    Int(int64),
    Float(f64),
    String(Vec<u16>),
    Binary(Vec<u8>),
}

#[derive(Default)]
struct AttributeList {
    values: Mutex<HashMap<Vec<u8>, AttributeValue>>,
}

impl Class for AttributeList {
    type Interfaces = (IAttributeList,);
}

impl IAttributeListTrait for AttributeList {
    unsafe fn setInt(&self, id: *const std::ffi::c_char, value: int64) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        self.values
            .lock()
            .expect("host attribute lock")
            .insert(key, AttributeValue::Int(value));
        kResultOk
    }

    unsafe fn getInt(&self, id: *const std::ffi::c_char, value: *mut int64) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        if value.is_null() {
            return kInvalidArgument;
        }
        let values = self.values.lock().expect("host attribute lock");
        match values.get(&key) {
            Some(AttributeValue::Int(stored)) => {
                *value = *stored;
                kResultOk
            }
            _ => kResultFalse,
        }
    }

    unsafe fn setFloat(&self, id: *const std::ffi::c_char, value: f64) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        self.values
            .lock()
            .expect("host attribute lock")
            .insert(key, AttributeValue::Float(value));
        kResultOk
    }

    unsafe fn getFloat(&self, id: *const std::ffi::c_char, value: *mut f64) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        if value.is_null() {
            return kInvalidArgument;
        }
        let values = self.values.lock().expect("host attribute lock");
        match values.get(&key) {
            Some(AttributeValue::Float(stored)) => {
                *value = *stored;
                kResultOk
            }
            _ => kResultFalse,
        }
    }

    unsafe fn setString(&self, id: *const std::ffi::c_char, string: *const TChar) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        let Some(value) = (unsafe { read_utf16(string) }) else {
            return kInvalidArgument;
        };
        self.values
            .lock()
            .expect("host attribute lock")
            .insert(key, AttributeValue::String(value));
        kResultOk
    }

    unsafe fn getString(
        &self,
        id: *const std::ffi::c_char,
        string: *mut TChar,
        size_in_bytes: uint32,
    ) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        if string.is_null() || size_in_bytes < 2 {
            return kInvalidArgument;
        }
        let capacity = (size_in_bytes as usize / 2).saturating_sub(1);
        let values = self.values.lock().expect("host attribute lock");
        let Some(AttributeValue::String(stored)) = values.get(&key) else {
            return kResultFalse;
        };
        if stored.len() > capacity {
            return kResultFalse;
        }
        for (index, unit) in stored.iter().copied().enumerate() {
            *string.add(index) = unit as TChar;
        }
        *string.add(stored.len()) = 0;
        kResultOk
    }

    unsafe fn setBinary(
        &self,
        id: *const std::ffi::c_char,
        data: *const c_void,
        size_in_bytes: uint32,
    ) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        if data.is_null() && size_in_bytes != 0 {
            return kInvalidArgument;
        }
        let bytes = if size_in_bytes == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size_in_bytes as usize) }
                .to_vec()
        };
        self.values
            .lock()
            .expect("host attribute lock")
            .insert(key, AttributeValue::Binary(bytes));
        kResultOk
    }

    unsafe fn getBinary(
        &self,
        id: *const std::ffi::c_char,
        data: *mut *const c_void,
        size_in_bytes: *mut uint32,
    ) -> tresult {
        let Some(key) = attribute_key(id) else {
            return kInvalidArgument;
        };
        if data.is_null() || size_in_bytes.is_null() {
            return kInvalidArgument;
        }
        let values = self.values.lock().expect("host attribute lock");
        match values.get(&key) {
            Some(AttributeValue::Binary(stored)) => {
                *data = stored.as_ptr().cast();
                *size_in_bytes = stored.len() as uint32;
                kResultOk
            }
            _ => kResultFalse,
        }
    }
}

/// Host application object passed to `IPluginBase::initialize`.
pub struct HostContext {
    application: ComWrapper<HostApplication>,
    #[cfg(target_os = "linux")]
    run_loop: Arc<LinuxRunLoop>,
}

impl Default for HostContext {
    fn default() -> Self {
        Self::new()
    }
}

impl HostContext {
    pub fn new() -> Self {
        #[cfg(target_os = "linux")]
        let run_loop = Arc::new(LinuxRunLoop::default());
        Self {
            application: ComWrapper::new(HostApplication {
                #[cfg(target_os = "linux")]
                run_loop: Arc::clone(&run_loop),
            }),
            #[cfg(target_os = "linux")]
            run_loop,
        }
    }

    /// Services registered Linux VST3 file descriptors and timers. Called on
    /// the native editor thread; never called by the audio callback.
    #[cfg(target_os = "linux")]
    pub fn pump_run_loop(&self) {
        self.run_loop.pump();
    }

    /// Shares the same UI-thread run loop with the host `IPlugFrame` object.
    #[cfg(target_os = "linux")]
    pub(crate) fn run_loop(&self) -> Arc<LinuxRunLoop> {
        Arc::clone(&self.run_loop)
    }

    /// Returns the `FUnknown` interface pointer for the lifetime of this host.
    pub fn as_funknown(&self) -> *mut FUnknown {
        self.application
            .as_com_ref::<IHostApplication>()
            .expect("host application interface")
            .upcast::<FUnknown>()
            .as_ptr()
    }
}

struct HostApplication {
    #[cfg(target_os = "linux")]
    run_loop: Arc<LinuxRunLoop>,
}

#[cfg(target_os = "linux")]
type HostApplicationInterfaces = (IHostApplication, IRunLoop);
#[cfg(not(target_os = "linux"))]
type HostApplicationInterfaces = (IHostApplication,);

impl Class for HostApplication {
    type Interfaces = HostApplicationInterfaces;
}

#[cfg(target_os = "linux")]
#[derive(Default)]
pub(crate) struct LinuxRunLoop {
    event_handlers: Mutex<HashMap<(i32, usize), ComPtr<IEventHandler>>>,
    timers: Mutex<HashMap<usize, LinuxTimer>>,
}

#[cfg(target_os = "linux")]
struct LinuxTimer {
    handler: ComPtr<ITimerHandler>,
    interval: Duration,
    next: Instant,
}

#[cfg(target_os = "linux")]
impl LinuxRunLoop {
    pub(crate) unsafe fn register_event_handler(
        &self,
        handler: *mut IEventHandler,
        fd: i32,
    ) -> tresult {
        if handler.is_null() || fd < 0 {
            return kInvalidArgument;
        }
        // SAFETY: the VST3 callback supplies a live borrowed interface. Retain
        // it for the registered lifetime, then transfer that owned ref to ComPtr.
        unsafe { ((*(*handler).vtbl).base.addRef)(handler.cast()) };
        let Some(retained) = (unsafe { ComPtr::<IEventHandler>::from_raw(handler) }) else {
            return kInvalidArgument;
        };
        let key = (fd, retained.as_ptr() as usize);
        self.event_handlers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(key, retained);
        kResultOk
    }

    pub(crate) unsafe fn unregister_event_handler(&self, handler: *mut IEventHandler) -> tresult {
        if handler.is_null() {
            return kInvalidArgument;
        }
        let address = handler as usize;
        let mut handlers = self
            .event_handlers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let keys: Vec<_> = handlers
            .keys()
            .filter(|(_, registered)| *registered == address)
            .copied()
            .collect();
        if keys.is_empty() {
            return kResultFalse;
        }
        for key in keys {
            handlers.remove(&key);
        }
        kResultOk
    }

    pub(crate) unsafe fn register_timer(
        &self,
        handler: *mut ITimerHandler,
        milliseconds: u64,
    ) -> tresult {
        if handler.is_null() || milliseconds == 0 {
            return kInvalidArgument;
        }
        // SAFETY: retain the plug-in's live borrowed interface before storing
        // it beyond this callback.
        unsafe { ((*(*handler).vtbl).base.addRef)(handler.cast()) };
        let Some(retained) = (unsafe { ComPtr::<ITimerHandler>::from_raw(handler) }) else {
            return kInvalidArgument;
        };
        let interval = Duration::from_millis(milliseconds.max(1));
        let now = Instant::now();
        let Some(next) = now.checked_add(interval) else {
            return kInvalidArgument;
        };
        self.timers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                retained.as_ptr() as usize,
                LinuxTimer {
                    handler: retained,
                    interval,
                    next,
                },
            );
        kResultOk
    }

    pub(crate) unsafe fn unregister_timer(&self, handler: *mut ITimerHandler) -> tresult {
        if handler.is_null() {
            return kInvalidArgument;
        }
        let removed = self
            .timers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&(handler as usize));
        if removed.is_some() {
            kResultOk
        } else {
            kResultFalse
        }
    }

    fn pump(&self) {
        let handlers: Vec<_> = self
            .event_handlers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|(&(fd, _), handler)| (fd, handler.clone()))
            .collect();
        if !handlers.is_empty() {
            let mut pollfds: Vec<_> = handlers
                .iter()
                .map(|(fd, _)| libc::pollfd {
                    fd: *fd,
                    events: libc::POLLIN | libc::POLLPRI | libc::POLLERR | libc::POLLHUP,
                    revents: 0,
                })
                .collect();
            // This is a zero-timeout poll on the editor thread, not the audio
            // callback. A bad plug-in fd cannot stall playback or the app UI.
            let ready = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as _, 0) };
            if ready > 0 {
                for ((fd, handler), pollfd) in handlers.iter().zip(pollfds.iter()) {
                    if pollfd.revents != 0 {
                        unsafe { handler.onFDIsSet(*fd) };
                    }
                }
            }
        }

        let now = Instant::now();
        let due: Vec<_> = {
            let mut timers = self
                .timers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            timers
                .values_mut()
                .filter_map(|timer| {
                    if timer.next <= now {
                        timer.next = now
                            .checked_add(timer.interval)
                            .unwrap_or(now + Duration::from_secs(86_400));
                        Some(timer.handler.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };
        for handler in due {
            unsafe { handler.onTimer() };
        }
    }
}

#[cfg(target_os = "linux")]
impl IRunLoopTrait for HostApplication {
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

impl IHostApplicationTrait for HostApplication {
    unsafe fn getName(&self, name: *mut String128) -> tresult {
        write_string128(name, HOST_NAME);
        kResultOk
    }

    unsafe fn createInstance(
        &self,
        _cid: *mut TUID,
        iid: *mut TUID,
        obj: *mut *mut c_void,
    ) -> tresult {
        // Host services are selected by the requested interface only. This
        // matches the SDK's HostApplication implementation and the behaviour
        // WaveShell children rely on; their class ID does not necessarily equal
        // the host object's interface ID.
        if iid.is_null() || obj.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: validated non-null; the caller passed a 16-byte TUID.
        unsafe { *obj = std::ptr::null_mut() };
        // SAFETY: `iid` points to a 16-byte TUID by the VST3 contract.
        let interface_id = unsafe { std::slice::from_raw_parts(iid.cast::<u8>(), 16) };

        if interface_id == IMessage::IID.as_slice() {
            let message = HostMessage::new();
            let Some(message) = message.to_com_ptr::<IMessage>() else {
                return kInternalError;
            };
            // SAFETY: `into_raw` transfers the reference owned by ComPtr to the
            // caller, which releases it through COM.
            unsafe { *obj = message.into_raw().cast() };
            return kResultOk;
        }
        if interface_id == IAttributeList::IID.as_slice() {
            let attributes = ComWrapper::new(AttributeList::default());
            let Some(attributes) = attributes.to_com_ptr::<IAttributeList>() else {
                return kInternalError;
            };
            // SAFETY: as above; ownership is transferred to the caller.
            unsafe { *obj = attributes.into_raw().cast() };
            return kResultOk;
        }
        kResultFalse
    }
}

/// Host-side message object. A component can obtain it through service
/// discovery; it is never touched by the audio callback.
pub struct HostMessage {
    id: Mutex<CString>,
    attributes: ComWrapper<AttributeList>,
}

impl HostMessage {
    pub fn new() -> ComWrapper<Self> {
        ComWrapper::new(Self {
            id: Mutex::new(CString::new("").expect("empty CString")),
            attributes: ComWrapper::new(AttributeList::default()),
        })
    }
}

impl Class for HostMessage {
    type Interfaces = (IMessage,);
}

impl IMessageTrait for HostMessage {
    unsafe fn getMessageID(&self) -> FIDString {
        self.id.lock().expect("host message lock").as_ptr()
    }

    unsafe fn setMessageID(&self, id: FIDString) {
        if id.is_null() {
            return;
        }
        let value = unsafe { CStr::from_ptr(id) }.to_owned();
        *self.id.lock().expect("host message lock") = value;
    }

    unsafe fn getAttributes(&self) -> *mut IAttributeList {
        self.attributes
            .to_com_ptr::<IAttributeList>()
            .map(ComPtr::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }
}

/// Returns a borrowed host-message interface for tests or future service
/// discovery. The message must outlive the returned reference.
pub fn message_ref(message: &ComWrapper<HostMessage>) -> Option<ComRef<'_, IMessage>> {
    message.as_com_ref::<IMessage>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_context_reports_opentrack_and_messages_round_trip_attributes() {
        let context = HostContext::new();
        let application = context
            .application
            .as_com_ref::<IHostApplication>()
            .expect("application interface");
        let mut name = [0 as TChar; 128];
        unsafe {
            assert_eq!(application.getName(&mut name), kResultOk);
        }
        let end = name
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(name.len());
        assert_eq!(
            String::from_utf16_lossy(&name[..end]),
            "OpenTrack",
            "host name must be visible to components"
        );

        let mut raw_message = std::ptr::null_mut();
        // WaveShell-style plug-ins request host objects by interface ID and may
        // pass an unrelated class ID; a null class ID proves we ignore it.
        let result = unsafe {
            application.createInstance(
                std::ptr::null_mut(),
                IMessage::IID.as_ptr() as *mut TUID,
                &mut raw_message,
            )
        };
        assert_eq!(result, kResultOk);
        let host_message =
            unsafe { ComPtr::<IMessage>::from_raw(raw_message.cast()) }.expect("host message");
        let mut raw_attributes = std::ptr::null_mut();
        let result = unsafe {
            application.createInstance(
                std::ptr::null_mut(),
                IAttributeList::IID.as_ptr() as *mut TUID,
                &mut raw_attributes,
            )
        };
        assert_eq!(result, kResultOk);
        let host_attributes = unsafe { ComPtr::<IAttributeList>::from_raw(raw_attributes.cast()) }
            .expect("host attributes");
        let key = c"host-created".as_ptr();
        unsafe {
            assert_eq!(host_attributes.setInt(key, 7), kResultOk);
            let mut value = 0;
            assert_eq!(host_attributes.getInt(key, &mut value), kResultOk);
            assert_eq!(value, 7);
            assert!(
                CStr::from_ptr(host_message.getMessageID())
                    .to_bytes()
                    .is_empty(),
                "a new host message starts with an empty ID"
            );
        }

        let message = HostMessage::new();
        let reference = message_ref(&message).expect("message interface");
        unsafe {
            reference.setMessageID(c"OpenTrack.test".as_ptr());
            assert_eq!(
                CStr::from_ptr(reference.getMessageID()).to_str().unwrap(),
                "OpenTrack.test"
            );
            let attributes = ComPtr::from_raw(reference.getAttributes()).expect("attributes");
            let key = c"value".as_ptr();
            assert_eq!(attributes.setInt(key, 42), kResultOk);
            let mut value = 0;
            assert_eq!(attributes.getInt(key, &mut value), kResultOk);
            assert_eq!(value, 42);
        }
    }

    #[cfg(target_os = "linux")]
    mod linux_run_loop {
        use super::*;
        use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
        use std::time::Duration;
        use vst3::Steinberg::Linux::{
            FileDescriptor, IEventHandler, IEventHandlerTrait, ITimerHandler, ITimerHandlerTrait,
        };

        struct TestEventHandler {
            calls: AtomicUsize,
            last_fd: AtomicI32,
        }

        impl Class for TestEventHandler {
            type Interfaces = (IEventHandler,);
        }

        impl IEventHandlerTrait for TestEventHandler {
            unsafe fn onFDIsSet(&self, fd: FileDescriptor) {
                self.last_fd.store(fd, Ordering::Release);
                self.calls.fetch_add(1, Ordering::AcqRel);
            }
        }

        struct TestTimerHandler {
            calls: AtomicUsize,
        }

        impl Class for TestTimerHandler {
            type Interfaces = (ITimerHandler,);
        }

        impl ITimerHandlerTrait for TestTimerHandler {
            unsafe fn onTimer(&self) {
                self.calls.fetch_add(1, Ordering::AcqRel);
            }
        }

        #[test]
        fn dispatches_and_unregisters_readable_fds() {
            let run_loop = LinuxRunLoop::default();
            let handler = ComWrapper::new(TestEventHandler {
                calls: AtomicUsize::new(0),
                last_fd: AtomicI32::new(-1),
            });
            let interface = handler.as_com_ref::<IEventHandler>().unwrap();
            let mut pipe_fds = [-1; 2];
            assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0);
            assert_eq!(
                unsafe { run_loop.register_event_handler(interface.as_ptr(), pipe_fds[0]) },
                kResultOk
            );

            let byte = [1u8];
            assert_eq!(
                unsafe { libc::write(pipe_fds[1], byte.as_ptr().cast(), 1) },
                1
            );
            run_loop.pump();
            assert_eq!(handler.calls.load(Ordering::Acquire), 1);
            assert_eq!(handler.last_fd.load(Ordering::Acquire), pipe_fds[0]);

            assert_eq!(
                unsafe { run_loop.unregister_event_handler(interface.as_ptr()) },
                kResultOk
            );
            run_loop.pump();
            assert_eq!(handler.calls.load(Ordering::Acquire), 1);
            assert_eq!(
                unsafe { run_loop.unregister_event_handler(interface.as_ptr()) },
                kResultFalse
            );
            unsafe {
                libc::close(pipe_fds[0]);
                libc::close(pipe_fds[1]);
            }
        }

        #[test]
        fn dispatches_and_unregisters_timers() {
            let run_loop = LinuxRunLoop::default();
            let handler = ComWrapper::new(TestTimerHandler {
                calls: AtomicUsize::new(0),
            });
            let interface = handler.as_com_ref::<ITimerHandler>().unwrap();
            assert_eq!(
                unsafe { run_loop.register_timer(interface.as_ptr(), 1) },
                kResultOk
            );
            std::thread::sleep(Duration::from_millis(5));
            run_loop.pump();
            assert_eq!(handler.calls.load(Ordering::Acquire), 1);
            assert_eq!(
                unsafe { run_loop.unregister_timer(interface.as_ptr()) },
                kResultOk
            );
            std::thread::sleep(Duration::from_millis(2));
            run_loop.pump();
            assert_eq!(handler.calls.load(Ordering::Acquire), 1);
            assert_eq!(
                unsafe { run_loop.unregister_timer(interface.as_ptr()) },
                kResultFalse
            );
        }
    }
}
