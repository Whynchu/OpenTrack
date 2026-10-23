//! Windows WinMM MIDI input. The driver callback only copies packed short
//! messages into a bounded queue; project edits stay on the UI thread.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    time::Instant,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MidiInputDevice {
    pub(crate) id: u32,
    pub(crate) name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MidiInputEvent {
    pub(crate) status: u8,
    pub(crate) data1: u8,
    pub(crate) data2: u8,
    pub(crate) timestamp_ms: u32,
}

const INPUT_QUEUE_CAPACITY: usize = 8_192;

pub(crate) fn available_midi_input_devices() -> Result<Vec<MidiInputDevice>, String> {
    #[cfg(windows)]
    {
        windows::available_devices()
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

pub(crate) struct MidiInputCapture {
    receiver: Receiver<MidiInputEvent>,
    dropped_events: Arc<AtomicU64>,
    started_at: Instant,
    #[cfg(windows)]
    native: windows::NativeInput,
}

impl MidiInputCapture {
    pub(crate) fn start(device_id: u32) -> Result<Self, String> {
        #[cfg(windows)]
        {
            let (sender, receiver) = mpsc::sync_channel(INPUT_QUEUE_CAPACITY);
            let dropped_events = Arc::new(AtomicU64::new(0));
            let (native, started_at) =
                windows::NativeInput::start(device_id, sender, Arc::clone(&dropped_events))?;
            Ok(Self {
                receiver,
                dropped_events,
                started_at,
                native,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = device_id;
            Err("live MIDI input is currently supported on Windows only".to_owned())
        }
    }

    pub(crate) fn try_recv(&self) -> Option<MidiInputEvent> {
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    pub(crate) fn dropped_events(&self) -> u64 {
        self.dropped_events.load(Ordering::Relaxed)
    }

    /// Milliseconds since the native input stream began.
    pub(crate) fn elapsed_millis(&self) -> u32 {
        self.started_at.elapsed().as_millis().min(u128::from(u32::MAX)) as u32
    }

    pub(crate) fn stop(&mut self) {
        #[cfg(windows)]
        self.native.stop();
    }
}

#[cfg(windows)]
mod windows {
    use std::{
        mem::size_of,
        ptr::{addr_of, null_mut},
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
            mpsc::SyncSender,
        },
        time::Instant,
    };

    use windows_sys::Win32::Media::Audio::{
        CALLBACK_FUNCTION, HMIDIIN, MIDIINCAPSW, midiInClose, midiInGetDevCapsW,
        midiInGetNumDevs, midiInOpen, midiInReset, midiInStart, midiInStop,
    };

    use super::{MidiInputDevice, MidiInputEvent};

    const MMSYSERR_NOERROR: u32 = 0;
    const MIM_DATA_MESSAGE: u32 = 0x03C3;

    struct CallbackContext {
        sender: SyncSender<MidiInputEvent>,
        dropped_events: Arc<AtomicU64>,
    }

    pub(super) struct NativeInput {
        handle: HMIDIIN,
        context: Option<Box<CallbackContext>>,
        stopped: bool,
    }

    impl NativeInput {
        pub(super) fn start(
            device_id: u32,
            sender: SyncSender<MidiInputEvent>,
            dropped_events: Arc<AtomicU64>,
        ) -> Result<(Self, Instant), String> {
            let context = Box::new(CallbackContext {
                sender,
                dropped_events,
            });
            let context = Box::into_raw(context);
            let mut handle = null_mut();
            let open_result = unsafe {
                midiInOpen(
                    &mut handle,
                    device_id,
                    midi_input_callback as usize,
                    context as usize,
                    CALLBACK_FUNCTION,
                )
            };
            if open_result != MMSYSERR_NOERROR {
                unsafe { drop(Box::from_raw(context)) };
                return Err(format!(
                    "could not open MIDI input device {device_id} (WinMM error {open_result})"
                ));
            }

            let started_at = Instant::now();
            let start_result = unsafe { midiInStart(handle) };
            if start_result != MMSYSERR_NOERROR {
                let close_result = unsafe { midiInClose(handle) };
                if close_result == MMSYSERR_NOERROR {
                    unsafe { drop(Box::from_raw(context)) };
                }
                return Err(format!(
                    "could not start MIDI input device {device_id} (WinMM error {start_result})"
                ));
            }

            Ok((
                Self {
                    handle,
                    context: Some(unsafe { Box::from_raw(context) }),
                    stopped: false,
                },
                started_at,
            ))
        }

        pub(super) fn stop(&mut self) {
            if !self.stopped && !self.handle.is_null() {
                unsafe {
                    let _ = midiInStop(self.handle);
                    let _ = midiInReset(self.handle);
                }
                self.stopped = true;
            }
        }
    }

    impl Drop for NativeInput {
        fn drop(&mut self) {
            self.stop();
            if self.handle.is_null() {
                return;
            }
            let close_result = unsafe { midiInClose(self.handle) };
            if close_result == MMSYSERR_NOERROR {
                self.handle = null_mut();
                self.context.take();
            } else {
                // A driver that refuses close may still call back. Keep its
                // context alive rather than leave the callback with a dangling pointer.
                if let Some(context) = self.context.take() {
                    let _ = Box::into_raw(context);
                }
            }
        }
    }

    unsafe extern "system" fn midi_input_callback(
        _handle: HMIDIIN,
        message: u32,
        instance: usize,
        parameter1: usize,
        parameter2: usize,
    ) {
        if message != MIM_DATA_MESSAGE || instance == 0 {
            return;
        }
        let context = unsafe { &*(instance as *const CallbackContext) };
        let packed = parameter1 as u32;
        let event = MidiInputEvent {
            status: packed as u8,
            data1: (packed >> 8) as u8,
            data2: (packed >> 16) as u8,
            timestamp_ms: parameter2 as u32,
        };
        if context.sender.try_send(event).is_err() {
            context.dropped_events.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn available_devices() -> Result<Vec<MidiInputDevice>, String> {
        let count = unsafe { midiInGetNumDevs() };
        let mut devices = Vec::with_capacity(count as usize);
        for id in 0..count {
            let mut caps = MIDIINCAPSW::default();
            let result = unsafe {
                midiInGetDevCapsW(
                    id as usize,
                    &mut caps,
                    size_of::<MIDIINCAPSW>() as u32,
                )
            };
            if result != MMSYSERR_NOERROR {
                return Err(format!(
                    "could not read MIDI input device {id} (WinMM error {result})"
                ));
            }
            let name_units = unsafe { addr_of!(caps.szPname).read_unaligned() };
            let name_end = name_units.iter().position(|unit| *unit == 0).unwrap_or(name_units.len());
            let name = String::from_utf16_lossy(&name_units[..name_end]);
            devices.push(MidiInputDevice { id, name });
        }
        Ok(devices)
    }
}
