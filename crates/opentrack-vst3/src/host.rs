// SPDX-License-Identifier: Apache-2.0

//! Minimal VST3 host context.
//!
//! This is a control-thread skeleton. It implements `IHostApplication`,
//! `IMessage`, and `IAttributeList` so a component can be initialized later, but
//! it does not yet own a component or process audio. Attribute storage uses a
//! mutex and must never be called from the audio callback.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_void};
use std::sync::Mutex;

use vst3::Steinberg::Vst::{
    IAttributeList, IAttributeListTrait, IHostApplication, IHostApplicationTrait, IMessage,
    IMessageTrait, String128, TChar,
};
use vst3::Steinberg::{
    FIDString, FUnknown, TUID, int64, kInvalidArgument, kNotImplemented, kResultFalse, kResultOk,
    tresult, uint32,
};
use vst3::{Class, ComPtr, ComRef, ComWrapper};

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
}

impl Default for HostContext {
    fn default() -> Self {
        Self::new()
    }
}

impl HostContext {
    pub fn new() -> Self {
        Self {
            application: ComWrapper::new(HostApplication),
        }
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

struct HostApplication;

impl Class for HostApplication {
    type Interfaces = (IHostApplication,);
}

impl IHostApplicationTrait for HostApplication {
    unsafe fn getName(&self, name: *mut String128) -> tresult {
        write_string128(name, HOST_NAME);
        kResultOk
    }

    unsafe fn createInstance(
        &self,
        _cid: *mut TUID,
        _iid: *mut TUID,
        _obj: *mut *mut c_void,
    ) -> tresult {
        // VST3 defines this for host-supplied classes. OpenTrack does not expose
        // any yet; components must not be initialized with an unsupported
        // expectation.
        kNotImplemented
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
}
