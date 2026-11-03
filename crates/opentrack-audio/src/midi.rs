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

/// One MIDI output port.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiOutputDevice {
    pub id: String,
    pub name: String,
    platform_id: Option<u32>,
}

/// An opened MIDI output, used for MMC and other short messages.
pub struct MidiOutputStream {
    connection: platform::OutputConnection,
    pub device_name: String,
}

impl MidiOutputStream {
    /// Lists the MIDI output ports the platform exposes.
    pub fn available_devices() -> Result<Vec<MidiOutputDevice>, String> {
        platform::available_output_devices()
    }

    /// Opens one output port.
    pub fn open(device_id: &str) -> Result<Self, String> {
        let device = Self::available_devices()?
            .into_iter()
            .find(|device| device.id == device_id)
            .ok_or_else(|| "the selected MIDI output is no longer available".to_owned())?;
        let device_name = device.name.clone();
        let connection = platform::OutputConnection::open(device)?;
        Ok(Self {
            connection,
            device_name,
        })
    }

    /// Sends one message: a one-to-three byte channel message or a system-exclusive message.
    pub fn send(&self, bytes: &[u8]) -> Result<(), String> {
        self.connection.send(bytes)
    }
}

/// MIDI Machine Control transport commands OpenTrack sends.
pub const MMC_STOP: u8 = 0x01;
pub const MMC_PLAY: u8 = 0x02;
pub const MMC_DEFERRED_PLAY: u8 = 0x03;
pub const MMC_RECORD_STROBE: u8 = 0x06;
pub const MMC_RECORD_EXIT: u8 = 0x07;

/// One MMC command for `device_id`, as a system-exclusive message.
pub fn mmc_message(device_id: u8, command: u8) -> Vec<u8> {
    vec![0xF0, 0x7F, device_id & 0x7F, 0x06, command & 0x7F, 0xF7]
}

/// MMC locate to `seconds`, expressed as `44 06 01 hh mn ss ff fr`.
///
/// Returns `None` for a negative or non-finite position and for hours past 23.
pub fn mmc_locate(device_id: u8, seconds: f64, frames_per_second: u8) -> Option<Vec<u8>> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let fps = f64::from(frames_per_second.max(1));
    let total = seconds;
    let hours = (total / 3600.0).floor();
    if hours > 23.0 {
        return None;
    }
    let minutes = (total / 60.0).floor() % 60.0;
    let whole_seconds = total.floor() % 60.0;
    let frames = ((total - total.floor()) * fps).floor() % fps;
    Some(vec![
        0xF0,
        0x7F,
        device_id & 0x7F,
        0x06,
        0x44,
        0x06,
        0x01,
        hours as u8,
        minutes as u8,
        whole_seconds as u8,
        frames as u8,
        0,
        0xF7,
    ])
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
        thread,
        time::{Duration, Instant},
    };

    use windows_sys::Win32::Media::Audio::{
        CALLBACK_FUNCTION, CALLBACK_NULL, HMIDIIN, HMIDIOUT, MHDR_DONE, MIDIHDR, MIDIINCAPSW,
        MIDIOUTCAPSW, midiInClose, midiInGetDevCapsW, midiInGetNumDevs, midiInOpen, midiInReset,
        midiInStart, midiInStop, midiOutClose, midiOutGetDevCapsW, midiOutGetNumDevs,
        midiOutLongMsg, midiOutOpen, midiOutPrepareHeader, midiOutReset, midiOutShortMsg,
        midiOutUnprepareHeader,
    };

    use super::{
        MIDI_INPUT_QUEUE_CAPACITY, MidiInputDevice, MidiInputEvent, MidiInputMessage,
        MidiOutputDevice,
    };

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

    /// One opened MIDI output. WinMM will not call back because the stream is
    /// opened with `CALLBACK_NULL`.
    pub(super) struct OutputConnection {
        handle: HMIDIOUT,
    }

    impl OutputConnection {
        pub(super) fn open(device: MidiOutputDevice) -> Result<Self, String> {
            let device_index = device
                .platform_id
                .ok_or_else(|| "the MIDI output does not have a Windows device ID".to_owned())?;
            let mut handle = ptr::null_mut();
            // SAFETY: CALLBACK_NULL with no callback means WinMM never calls
            // back; a successful open returns an owned handle.
            let result = unsafe { midiOutOpen(&mut handle, device_index, 0, 0, CALLBACK_NULL) };
            if result != 0 {
                return Err(format!(
                    "WinMM could not open MIDI output '{}' (error {result})",
                    device.name
                ));
            }
            Ok(Self { handle })
        }

        pub(super) fn send(&self, bytes: &[u8]) -> Result<(), String> {
            match bytes {
                [] => Ok(()),
                [status, data1, data2] if *status < 0xF0 => {
                    let packed =
                        u32::from(*status) | (u32::from(*data1) << 8) | (u32::from(*data2) << 16);
                    // SAFETY: the handle is open for the lifetime of self.
                    let result = unsafe { midiOutShortMsg(self.handle, packed) };
                    if result == 0 {
                        Ok(())
                    } else {
                        Err(format!(
                            "WinMM could not send a MIDI message (error {result})"
                        ))
                    }
                }
                _ => self.send_long(bytes),
            }
        }

        /// System-exclusive messages need a prepared header. The wait is
        /// bounded, and the buffer stays alive until it has been unprepared.
        fn send_long(&self, bytes: &[u8]) -> Result<(), String> {
            let mut buffer = bytes.to_vec();
            let mut header = MIDIHDR {
                lpData: buffer.as_mut_ptr(),
                dwBufferLength: buffer.len() as u32,
                ..MIDIHDR::default()
            };
            let size = size_of::<MIDIHDR>() as u32;
            // SAFETY: the header and the buffer it points at are valid for the
            // whole call, and the handle is open.
            let prepared = unsafe { midiOutPrepareHeader(self.handle, &mut header, size) };
            if prepared != 0 {
                return Err(format!(
                    "WinMM could not prepare a MIDI message (error {prepared})"
                ));
            }
            // SAFETY: the prepared header and its buffer are valid.
            let sent = unsafe { midiOutLongMsg(self.handle, &header, size) };
            let deadline = Instant::now() + Duration::from_millis(500);
            while header.dwFlags & MHDR_DONE == 0 && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(1));
            }
            // SAFETY: the header was prepared above and the handle is open.
            let unprepared = unsafe { midiOutUnprepareHeader(self.handle, &mut header, size) };
            drop(buffer);
            if sent != 0 {
                return Err(format!(
                    "WinMM could not send a MIDI message (error {sent})"
                ));
            }
            if unprepared != 0 {
                return Err(format!(
                    "WinMM could not release a MIDI message (error {unprepared})"
                ));
            }
            Ok(())
        }
    }

    impl Drop for OutputConnection {
        fn drop(&mut self) {
            if self.handle.is_null() {
                return;
            }
            // SAFETY: this object owns the open handle.
            unsafe {
                midiOutReset(self.handle);
                midiOutClose(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }

    pub(super) fn available_output_devices() -> Result<Vec<MidiOutputDevice>, String> {
        // SAFETY: midiOutGetNumDevs has no preconditions.
        let count = unsafe { midiOutGetNumDevs() };
        let mut devices = Vec::with_capacity(count as usize);
        for index in 0..count {
            let mut caps = MIDIOUTCAPSW::default();
            // SAFETY: caps points to a correctly sized MIDIOUTCAPSW value.
            let result = unsafe {
                midiOutGetDevCapsW(index as usize, &mut caps, size_of::<MIDIOUTCAPSW>() as u32)
            };
            if result != 0 {
                continue;
            }
            // MIDIOUTCAPSW is packed by the WinMM ABI; read packed fields
            // without creating an aligned reference to them.
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
            devices.push(MidiOutputDevice {
                id: format!("winmm-out:{index}:{manufacturer:04X}:{product:04X}:{name}"),
                name: if name.is_empty() {
                    format!("MIDI Output {index}")
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

    use super::{MidiInputDevice, MidiInputEvent, MidiOutputDevice};

    pub(super) struct OutputConnection;

    impl OutputConnection {
        pub(super) fn open(device: MidiOutputDevice) -> Result<Self, String> {
            let _ = device;
            Err("MIDI output is only implemented on Windows in this build".to_owned())
        }

        pub(super) fn send(&self, _bytes: &[u8]) -> Result<(), String> {
            Err("MIDI output is only implemented on Windows in this build".to_owned())
        }
    }

    pub(super) fn available_output_devices() -> Result<Vec<MidiOutputDevice>, String> {
        Ok(Vec::new())
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmc_transport_messages_are_well_formed() {
        assert_eq!(
            mmc_message(0x7F, MMC_PLAY),
            vec![0xF0, 0x7F, 0x7F, 0x06, 0x02, 0xF7]
        );
        assert_eq!(
            mmc_message(0x01, MMC_STOP),
            vec![0xF0, 0x7F, 0x01, 0x06, 0x01, 0xF7]
        );
        // Device ids are 7-bit; wider values are masked rather than emitted raw.
        assert_eq!(mmc_message(0xFF, MMC_DEFERRED_PLAY)[2], 0x7F);
        assert_eq!(mmc_message(0x01, 0x86)[4], 0x06);
    }

    #[test]
    fn mmc_locate_converts_seconds_to_timecode() {
        assert_eq!(
            mmc_locate(0x01, 3661.5, 25).expect("locate"),
            vec![
                0xF0, 0x7F, 0x01, 0x06, 0x44, 0x06, 0x01, 0x01, 0x01, 0x01, 12, 0, 0xF7
            ]
        );
        assert_eq!(
            mmc_locate(0x7F, 0.0, 30).expect("locate"),
            vec![
                0xF0, 0x7F, 0x7F, 0x06, 0x44, 0x06, 0x01, 0, 0, 0, 0, 0, 0xF7
            ]
        );
        assert!(mmc_locate(1, -1.0, 25).is_none());
        assert!(mmc_locate(1, f64::NAN, 25).is_none());
        assert!(mmc_locate(1, 24.0 * 3600.0, 25).is_none(), "past 23 hours");
    }

    #[cfg(not(windows))]
    #[test]
    fn midi_output_is_reported_unavailable_off_windows() {
        assert!(
            MidiOutputStream::available_devices()
                .expect("list")
                .is_empty()
        );
        let error = match MidiOutputStream::open("missing") {
            Ok(_) => panic!("no MIDI output should open"),
            Err(error) => error,
        };
        assert!(error.contains("no longer available"), "{error}");
    }
}
