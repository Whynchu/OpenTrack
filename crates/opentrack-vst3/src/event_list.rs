// SPDX-License-Identifier: Apache-2.0

//! Fixed-capacity VST3 MIDI event list for realtime processing blocks.

use std::cell::UnsafeCell;

use vst3::ComWrapper;
use vst3::Steinberg::Vst::{
    Event, Event_::EventTypes_, Event__type0, IEventList, IEventListTrait, NoteOffEvent,
    NoteOnEvent,
};
use vst3::Steinberg::{int32, kInvalidArgument, kResultFalse, kResultOk, tresult};
use vst3::{Class, ComPtr};

pub(crate) const MAX_EVENTS_PER_BLOCK: usize = 512;

pub(crate) struct EventList {
    events: UnsafeCell<[Event; MAX_EVENTS_PER_BLOCK]>,
    count: UnsafeCell<usize>,
}

// SAFETY: only the serialized audio callback writes the list before process;
// the plug-in reads it synchronously during IAudioProcessor::process.
unsafe impl Sync for EventList {}

impl EventList {
    pub(crate) fn new() -> (ComWrapper<Self>, ComPtr<IEventList>) {
        let wrapper = ComWrapper::new(Self {
            // VST3 Event is a C ABI POD record. Zero is a valid empty value.
            events: UnsafeCell::new(unsafe { std::mem::zeroed() }),
            count: UnsafeCell::new(0),
        });
        let ptr = wrapper
            .to_com_ptr::<IEventList>()
            .expect("IEventList wrapper exposes its declared interface");
        (wrapper, ptr)
    }

    pub(crate) fn begin_block(&self, events: &[opentrack_plugins::HostedMidiEvent]) {
        let count = events.len().min(MAX_EVENTS_PER_BLOCK);
        // SAFETY: audio-thread-only mutation before the processor reads this list.
        let target = unsafe { &mut *self.events.get() };
        for (slot, source) in target.iter_mut().zip(events.iter()).take(count) {
            let mut event: Event = unsafe { std::mem::zeroed() };
            event.busIndex = 0;
            event.sampleOffset = source.sample_offset.min(i32::MAX as u32) as int32;
            event.ppqPosition = 0.0;
            event.flags = 0;
            if source.note_on {
                event.r#type = EventTypes_::kNoteOnEvent as u16;
                event.__field0 = Event__type0 {
                    noteOn: NoteOnEvent {
                        channel: i16::from(source.channel.min(15)),
                        pitch: i16::from(source.pitch.min(127)),
                        tuning: 0.0,
                        velocity: f32::from(source.velocity.min(127)) / 127.0,
                        length: 0,
                        noteId: source.note_id,
                    },
                };
            } else {
                event.r#type = EventTypes_::kNoteOffEvent as u16;
                event.__field0 = Event__type0 {
                    noteOff: NoteOffEvent {
                        channel: i16::from(source.channel.min(15)),
                        pitch: i16::from(source.pitch.min(127)),
                        velocity: 0.0,
                        noteId: source.note_id,
                        tuning: 0.0,
                    },
                };
            }
            *slot = event;
        }
        // SAFETY: the count is published after all array entries are written.
        unsafe { *self.count.get() = count };
    }
}

impl Class for EventList {
    type Interfaces = (IEventList,);
}

impl IEventListTrait for EventList {
    unsafe fn getEventCount(&self) -> int32 {
        // SAFETY: processor reads synchronously while the audio thread is not mutating.
        unsafe { *self.count.get() as int32 }
    }

    unsafe fn getEvent(&self, index: int32, event: *mut Event) -> tresult {
        if index < 0 || event.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: bounded index and non-null output were checked above; VST3
        // calls getEvent only during process, before the next block is written.
        let Some(value) = (unsafe { (&*self.events.get()).get(index as usize) })
            .filter(|_| (index as usize) < unsafe { *self.count.get() })
        else {
            return kInvalidArgument;
        };
        unsafe { event.write(*value) };
        kResultOk
    }

    unsafe fn addEvent(&self, _event: *mut Event) -> tresult {
        kResultFalse
    }
}
