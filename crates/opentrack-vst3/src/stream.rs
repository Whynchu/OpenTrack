// SPDX-License-Identifier: Apache-2.0

//! Host-side `IBStream` over an in-memory byte buffer.
//!
//! VST3 uses `IBStream` for component/controller state and presets. The
//! implementation is deliberately small: it supports the read, write, seek and
//! tell contract used by `getState`/`setState` and bounds all plugin writes so a
//! broken plug-in cannot make the host allocate without limit.
//!
//! The stream is used only on the control thread and locks a mutex per call.
//! It must never be reached from the audio callback.

use std::ffi::c_void;
use std::sync::Mutex;

use vst3::Steinberg::{
    IBStream, IBStreamTrait, int32, int64, kInvalidArgument, kResultFalse, kResultOk, tresult,
};

// See the note in `effect.rs`: `IStreamSeekMode` is `c_uint` off Windows while
// `IBStream::seek` takes `int32`, so compare against the ABI-width constants.
const kIBSeekSet: int32 = vst3::Steinberg::IBStream_::IStreamSeekMode_::kIBSeekSet as int32;
const kIBSeekCur: int32 = vst3::Steinberg::IBStream_::IStreamSeekMode_::kIBSeekCur as int32;
const kIBSeekEnd: int32 = vst3::Steinberg::IBStream_::IStreamSeekMode_::kIBSeekEnd as int32;
use vst3::{Class, ComWrapper};

/// Upper bound for one component or controller state chunk.
///
/// Real plug-ins normally write a few kilobytes to a few megabytes. 256 MiB is
/// generous while still bounding untrusted stream writes.
pub const MAX_STATE_BYTES: usize = 256 * 1024 * 1024;

struct StreamData {
    bytes: Vec<u8>,
    position: usize,
}

/// `IBStream` implementation backed by a `Vec<u8>`.
pub(crate) struct MemoryStream {
    data: Mutex<StreamData>,
}

impl MemoryStream {
    /// Creates an empty writable stream.
    pub(crate) fn empty() -> ComWrapper<Self> {
        ComWrapper::new(Self {
            data: Mutex::new(StreamData {
                bytes: Vec::new(),
                position: 0,
            }),
        })
    }

    /// Creates a readable stream over a copy of `bytes`.
    pub(crate) fn from_slice(bytes: &[u8]) -> ComWrapper<Self> {
        ComWrapper::new(Self {
            data: Mutex::new(StreamData {
                bytes: bytes.to_vec(),
                position: 0,
            }),
        })
    }

    /// Returns a copy of the current stream contents.
    pub(crate) fn contents(&self) -> Vec<u8> {
        match self.data.lock() {
            Ok(data) => data.bytes.clone(),
            Err(poisoned) => poisoned.into_inner().bytes.clone(),
        }
    }
}

/// Returns a borrowed `IBStream` interface for the lifetime of `stream`.
#[cfg(test)]
fn stream_ref(stream: &ComWrapper<MemoryStream>) -> Option<vst3::ComPtr<IBStream>> {
    stream.to_com_ptr::<IBStream>()
}

impl Class for MemoryStream {
    type Interfaces = (IBStream,);
}

impl IBStreamTrait for MemoryStream {
    unsafe fn read(
        &self,
        buffer: *mut c_void,
        num_bytes: int32,
        num_bytes_read: *mut int32,
    ) -> tresult {
        if num_bytes < 0 {
            return kInvalidArgument;
        }
        let requested = num_bytes as usize;
        if buffer.is_null() && requested != 0 {
            return kInvalidArgument;
        }
        let mut data = match self.data.lock() {
            Ok(data) => data,
            Err(poisoned) => poisoned.into_inner(),
        };
        let available = data.bytes.len().saturating_sub(data.position);
        let count = requested.min(available);
        if count > 0 {
            // SAFETY: `buffer` was validated non-null above (a non-zero request
            // requires one) and the caller promises room for `num_bytes` bytes;
            // `count <= requested`.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.bytes.as_ptr().add(data.position),
                    buffer.cast::<u8>(),
                    count,
                );
            }
            data.position += count;
        }
        // VST3 lets callers pass a null byte-count out pointer when they do not
        // need it.
        if !num_bytes_read.is_null() {
            // SAFETY: validated non-null above.
            unsafe { *num_bytes_read = count as int32 };
        }
        kResultOk
    }

    unsafe fn write(
        &self,
        buffer: *mut c_void,
        num_bytes: int32,
        num_bytes_written: *mut int32,
    ) -> tresult {
        if num_bytes < 0 {
            return kInvalidArgument;
        }
        if !num_bytes_written.is_null() {
            // SAFETY: validated non-null above.
            unsafe { *num_bytes_written = 0 };
        }
        let requested = num_bytes as usize;
        if requested == 0 {
            return kResultOk;
        }
        if buffer.is_null() {
            return kInvalidArgument;
        }
        let mut data = match self.data.lock() {
            Ok(data) => data,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(end) = data.position.checked_add(requested) else {
            return kResultFalse;
        };
        if end > MAX_STATE_BYTES {
            return kResultFalse;
        }
        if data.bytes.len() < end {
            data.bytes.resize(end, 0);
        }
        // SAFETY: `buffer` was validated non-null and the caller promises room
        // for `num_bytes` bytes; `end <= MAX_STATE_BYTES`.
        unsafe {
            std::ptr::copy_nonoverlapping(
                buffer.cast::<u8>(),
                data.bytes.as_mut_ptr().add(data.position),
                requested,
            );
        }
        if !num_bytes_written.is_null() {
            // SAFETY: validated non-null above.
            unsafe { *num_bytes_written = requested as int32 };
        }
        data.position = end;
        kResultOk
    }

    #[allow(non_upper_case_globals)]
    unsafe fn seek(&self, pos: int64, mode: int32, result: *mut int64) -> tresult {
        let mut data = match self.data.lock() {
            Ok(data) => data,
            Err(poisoned) => poisoned.into_inner(),
        };
        let base = match mode {
            kIBSeekSet => 0i64,
            kIBSeekCur => data.position as i64,
            kIBSeekEnd => data.bytes.len() as i64,
            _ => return kInvalidArgument,
        };
        let Some(next) = base.checked_add(pos) else {
            return kInvalidArgument;
        };
        if next < 0 || next as u64 > MAX_STATE_BYTES as u64 {
            return kInvalidArgument;
        }
        data.position = next as usize;
        if !result.is_null() {
            // SAFETY: validated non-null above.
            unsafe { *result = next };
        }
        kResultOk
    }

    unsafe fn tell(&self, pos: *mut int64) -> tresult {
        if pos.is_null() {
            return kInvalidArgument;
        }
        let data = match self.data.lock() {
            Ok(data) => data,
            Err(poisoned) => poisoned.into_inner(),
        };
        // SAFETY: validated non-null above.
        unsafe { *pos = data.position as int64 };
        kResultOk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_stream_round_trips_read_write_seek_and_tell() {
        let stream = MemoryStream::empty();
        let ibstream = stream_ref(&stream).expect("stream interface");
        let payload = b"OpenTrack state chunk";
        let mut written = 0;
        // SAFETY: the payload pointer is valid for its length.
        let write_result = unsafe {
            ibstream.write(
                payload.as_ptr().cast_mut().cast(),
                payload.len() as int32,
                &mut written,
            )
        };
        assert_eq!(write_result, kResultOk);
        assert_eq!(written as usize, payload.len());

        let mut position = -1;
        // SAFETY: the out pointer is valid.
        assert_eq!(unsafe { ibstream.tell(&mut position) }, kResultOk);
        assert_eq!(position as usize, payload.len());

        let mut seek_result = -1;
        // SAFETY: the out pointer is valid.
        assert_eq!(
            unsafe { ibstream.seek(0, kIBSeekSet, &mut seek_result) },
            kResultOk
        );
        assert_eq!(seek_result, 0);

        let mut buffer = [0u8; 32];
        let mut read = 0;
        // SAFETY: the buffer is valid for 32 bytes.
        let read_result =
            unsafe { ibstream.read(buffer.as_mut_ptr().cast(), buffer.len() as int32, &mut read) };
        assert_eq!(read_result, kResultOk);
        assert_eq!(&buffer[..read as usize], payload);

        let mut end = 0;
        // SAFETY: the out pointer is valid.
        assert_eq!(unsafe { ibstream.seek(0, kIBSeekEnd, &mut end) }, kResultOk);
        assert_eq!(end as usize, payload.len());
        assert_eq!(stream.contents(), payload);
    }

    #[test]
    fn memory_stream_rejects_negative_lengths_and_bad_seeks() {
        let stream = MemoryStream::from_slice(b"abc");
        let ibstream = stream_ref(&stream).expect("stream interface");
        let mut count = 0;
        // SAFETY: null buffer is explicitly rejected by the implementation.
        assert_eq!(
            unsafe { ibstream.read(std::ptr::null_mut(), 1, &mut count) },
            kInvalidArgument
        );
        let mut position = 0;
        // SAFETY: the out pointers are valid.
        assert_eq!(
            unsafe { ibstream.seek(-1, kIBSeekSet, &mut position) },
            kInvalidArgument
        );
        let mut buffer = [0u8; 4];
        // SAFETY: the out pointers are valid.
        assert_eq!(
            unsafe { ibstream.read(buffer.as_mut_ptr().cast(), -1, &mut count) },
            kInvalidArgument
        );
    }

    #[test]
    fn memory_stream_allows_optional_byte_count_pointers() {
        let stream = MemoryStream::empty();
        let ibstream = stream_ref(&stream).expect("stream interface");
        let payload = b"state";
        // SAFETY: the buffer is valid for its length; VST3 permits a null
        // numBytesWritten pointer.
        assert_eq!(
            unsafe {
                ibstream.write(
                    payload.as_ptr().cast_mut().cast(),
                    payload.len() as i32,
                    std::ptr::null_mut(),
                )
            },
            kResultOk
        );
        let mut seek_result = 0;
        // SAFETY: out pointer is valid.
        assert_eq!(
            unsafe { ibstream.seek(0, kIBSeekSet, &mut seek_result) },
            kResultOk
        );
        let mut buffer = [0u8; 8];
        let mut read = 0;
        // SAFETY: the buffer is valid for 8 bytes; VST3 permits a null
        // numBytesRead pointer.
        assert_eq!(
            unsafe { ibstream.read(buffer.as_mut_ptr().cast(), 5, std::ptr::null_mut()) },
            kResultOk
        );
        assert_eq!(&buffer[..5], payload);
        // The count pointer variant still reports the count.
        // SAFETY: both out pointers are valid.
        assert_eq!(
            unsafe { ibstream.read(buffer.as_mut_ptr().cast(), 5, &mut read) },
            kResultOk
        );
        assert_eq!(read, 0, "stream is already at the end");
    }

    #[test]
    fn memory_stream_interface_is_reference_counted() {
        let stream = MemoryStream::empty();
        let first = stream_ref(&stream).expect("first interface");
        let second = first.clone();
        drop(first);
        drop(second);
        assert!(stream.contents().is_empty());
    }
}
