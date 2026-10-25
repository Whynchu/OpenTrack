//! Windows MIDI input through the WinMM API.
//!
//! The OS callback only copies short note and controller messages into a
//! bounded queue. Project edits, monitoring, and scheduling stay on the app
//! and audio threads.

use std::{
    sync::mpsc::{Receiver, TryRecvError},
    time::Instant,
};

const MIDI_INPUT_QUEUE_CAPACITY: usize = 8_192;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiInputDevice {
    pub id: String,
    pub name: String,
    platform_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MidiInputMessage {
    NoteOn {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct MidiInputEvent {
    /// WinMM's timestamp in milliseconds since the input stream started.
    pub timestamp_ms: u32,
    /// Callback arrival time used to align recording with the OpenTrack audio clock.
    pub received_at: Instant,
    pub message: MidiInputMessage,
}

/// An opened MIDI input whose callback feeds a bounded UI-thread queue.
pub struct MidiInputStream {
    _connection: platform::Connection,
    receiver: Receiver<MidiInputEvent>,
    dropped_messages: platform::DroppedMessageCount,
    pub device_name: String,
}

impl MidiInputStream {
    /// Lists the MIDI input ports exposed by the Windows multimedia API.
    pub fn available_devices() -> Result<Vec<MidiInputDevice>, String> {
        platform::available_devices()
    }

    /// Opens one input port and starts receiving short note and CC messages.
    pub fn open(device_id: &str) -> Result<Self, String> {
        let device = Self::available_devices()?
            .into_iter()
            .find(|device| device.id == device_id)
            .ok_or_else(|| "the selected MIDI input is no longer available".to_owned())?;
        let device_name = device.name.clone();
        let (connection, receiver, dropped_messages) = platform::Connection::open(device)?;
        Ok(Self {
            _connection: connection,
            receiver,
            dropped_messages,
            device_name,
        })
    }

    /// Drains received events into a caller-owned buffer without blocking.
    pub fn drain_into(&self, output: &mut Vec<MidiInputEvent>) {
        loop {
            match self.receiver.try_recv() {
                Ok(event) => output.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    /// Number of messages dropped because the bounded callback queue was full.
    pub fn dropped_messages(&self) -> u64 {
        self.dropped_messages.load()
    }
}

#[cfg(windows)]
mod platform {
    use std::{
        mem::size_of,
        ptr,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
            mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
        },
        time::Instant,
    };

    use windows_sys::Win32::Media::Audio::{
        CALLBACK_FUNCTION, HMIDIIN, MIDIINCAPSW, midiInClose, midiInGetDevCapsW, midiInGetNumDevs,
        midiInOpen, midiInReset, midiInStart, midiInStop,
    };

    use super::{MIDI_INPUT_QUEUE_CAPACITY, MidiInputDevice, MidiInputEvent, MidiInputMessage};

    const MIM_DATA: u32 = 0x03C3;

    pub(super) struct DroppedMessageCount(Arc<AtomicU64>);

    impl DroppedMessageCount {
        pub(super) fn load(&self) -> u64 {
            self.0.load(Ordering::Relaxed)
        }
    }

    struct CallbackContext {
        sender: SyncSender<MidiInputEvent>,
        dropped_messages: Arc<AtomicU64>,
    }

    pub(super) struct Connection {
        handle: HMIDIIN,
        callback_context: Option<Box<CallbackContext>>,
    }

    impl Connection {
        pub(super) fn open(
            device: MidiInputDevice,
        ) -> Result<(Self, Receiver<MidiInputEvent>, DroppedMessageCount), String> {
            let device_index = device
                .platform_id
                .ok_or_else(|| "the MIDI input does not have a Windows device ID".to_owned())?;
            let (sender, receiver) = sync_channel(MIDI_INPUT_QUEUE_CAPACITY);
            let dropped_messages = Arc::new(AtomicU64::new(0));
            let callback_context = Box::new(CallbackContext {
                sender,
                dropped_messages: Arc::clone(&dropped_messages),
            });
            let callback_instance = (&*callback_context as *const CallbackContext) as usize;
            let mut handle = ptr::null_mut();

            // SAFETY: the callback context is heap-stable and remains alive
            // until midiInClose has returned. The callback function follows
            // WinMM's MIDIINPROC calling convention and does not unwind.
            let open_result = unsafe {
                midiInOpen(
                    &mut handle,
                    device_index,
                    midi_input_callback as *const () as usize,
                    callback_instance,
                    CALLBACK_FUNCTION,
                )
            };
            if open_result != 0 {
                return Err(format!(
                    "WinMM could not open MIDI input '{}' (error {open_result})",
                    device.name
                ));
            }

            // SAFETY: a successful midiInOpen returned a valid handle.
            let start_result = unsafe { midiInStart(handle) };
            if start_result != 0 {
                // SAFETY: the handle is valid and the callback context remains
                // alive until the handle has been closed.
                unsafe {
                    midiInStop(handle);
                    midiInReset(handle);
                }
                // SAFETY: the handle is still owned here. If close fails,
                // WinMM may still call back with the instance pointer, so keep
                // the callback allocation alive just as `Connection::close` does.
                let close_result = unsafe { midiInClose(handle) };
                if close_result != 0 {
                    let _ = Box::leak(callback_context);
                }
                return Err(format!(
                    "WinMM could not start MIDI input '{}' (error {start_result})",
                    device.name
                ));
            }

            let connection = Self {
                handle,
                callback_context: Some(callback_context),
            };
            Ok((connection, receiver, DroppedMessageCount(dropped_messages)))
        }

        fn close(&mut self) {
            if self.handle.is_null() {
                return;
            }
            // SAFETY: this object owns the open handle. The callback context
            // remains allocated until midiInClose completes.
            unsafe {
                midiInStop(self.handle);
                midiInReset(self.handle);
            }
            // SAFETY: the handle is still owned by this connection.
            let close_result = unsafe { midiInClose(self.handle) };
            self.handle = ptr::null_mut();
            if close_result == 0 {
                self.callback_context = None;
            } else if let Some(context) = self.callback_context.take() {
                // A failed close may leave WinMM callbacks outstanding. Leak
                // the tiny context rather than expose a dangling callback ptr.
                let _ = Box::leak(context);
            }
        }
    }

    impl Drop for Connection {
        fn drop(&mut self) {
            self.close();
        }
    }

    pub(super) fn available_devices() -> Result<Vec<MidiInputDevice>, String> {
        // SAFETY: midiInGetNumDevs has no preconditions.
        let count = unsafe { midiInGetNumDevs() };
        let mut devices = Vec::with_capacity(count as usize);
        for index in 0..count {
            let mut caps = MIDIINCAPSW::default();
            // SAFETY: caps points to a correctly sized MIDIINCAPSW value.
            let result = unsafe {
                midiInGetDevCapsW(index as usize, &mut caps, size_of::<MIDIINCAPSW>() as u32)
            };
            if result != 0 {
                continue;
            }
            // MIDIINCAPSW is packed by the WinMM ABI. Read each packed field
            // without creating an aligned reference to it.
            let (manufacturer, product, name_units) = unsafe {
                (
                    ptr::addr_of!(caps.wMid).read_unaligned(),
                    ptr::addr_of!(caps.wPid).read_unaligned(),
                    ptr::addr_of!(caps.szPname).read_unaligned(),
                )
            };
            let name_length = name_units
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(name_units.len());
            let name = String::from_utf16_lossy(&name_units[..name_length]);
            devices.push(MidiInputDevice {
                id: format!("winmm:{index}:{manufacturer:04X}:{product:04X}:{name}"),
                name: if name.is_empty() {
                    format!("MIDI Input {index}")
                } else {
                    name
                },
                platform_id: Some(index),
            });
        }
        Ok(devices)
    }

    unsafe extern "system" fn midi_input_callback(
        _handle: HMIDIIN,
        message: u32,
        instance: usize,
        packed_message: usize,
        timestamp_ms: usize,
    ) {
        if message != MIM_DATA || instance == 0 {
            return;
        }
        // SAFETY: midiInOpen receives this pointer as dwInstance, and
        // Connection keeps the allocation alive until the input handle closes.
        let context = unsafe { &*(instance as *const CallbackContext) };
        let packed = packed_message as u32;
        let status = packed as u8;
        let data1 = ((packed >> 8) & 0x7F) as u8;
        let data2 = ((packed >> 16) & 0x7F) as u8;
        let channel = status & 0x0F;
        let message = match status & 0xF0 {
            0x80 => MidiInputMessage::NoteOff {
                channel,
                key: data1,
                velocity: data2,
            },
            0x90 if data2 == 0 => MidiInputMessage::NoteOff {
                channel,
                key: data1,
                velocity: 0,
            },
            0x90 => MidiInputMessage::NoteOn {
                channel,
                key: data1,
                velocity: data2,
            },
            0xB0 => MidiInputMessage::ControlChange {
                channel,
                controller: data1,
                value: data2,
            },
            _ => return,
        };
        let event = MidiInputEvent {
            timestamp_ms: timestamp_ms as u32,
            received_at: Instant::now(),
            message,
        };
        if matches!(context.sender.try_send(event), Err(TrySendError::Full(_))) {
            context.dropped_messages.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::Receiver,
    };

    use super::{MidiInputDevice, MidiInputEvent};

    pub(super) struct DroppedMessageCount(Arc<AtomicU64>);

    impl DroppedMessageCount {
        pub(super) fn load(&self) -> u64 {
            self.0.load(Ordering::Relaxed)
        }
    }

    pub(super) struct Connection;

    impl Connection {
        pub(super) fn open(
            _device: MidiInputDevice,
        ) -> Result<(Self, Receiver<MidiInputEvent>, DroppedMessageCount), String> {
            Err("MIDI input is currently available only on Windows".to_owned())
        }
    }

    pub(super) fn available_devices() -> Result<Vec<MidiInputDevice>, String> {
        Ok(Vec::new())
    }
}
