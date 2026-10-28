// SPDX-License-Identifier: Apache-2.0

//! Audio-thread parameter plumbing.
//!
//! Two pieces live here:
//!
//! - [`PendingParameters`], a fixed-size lock-free multi-producer/single-consumer
//!   queue. Control/UI threads push normalized edits; the audio callback pops
//!   them once per block. It never allocates or takes a lock.
//! - [`ParameterChanges`], the host `IParameterChanges`/`IParamValueQueue`
//!   implementation handed to `IAudioProcessor::process`. Its storage is
//!   preallocated and only the audio thread mutates it while the plug-in reads.
//!
//! Both types rely on the VST3 contract that `process` is serialized on one
//! audio thread. No controller or editor method is called from here.

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

use vst3::Steinberg::Vst::{
    IParamValueQueue, IParamValueQueueTrait, IParameterChanges, IParameterChangesTrait, ParamID,
    ParamValue,
};
use vst3::Steinberg::{int32, kInvalidArgument, kResultFalse, kResultOk, tresult};
use vst3::{Class, ComWrapper};

const PENDING_CAPACITY: usize = 256;
const PENDING_MASK: usize = PENDING_CAPACITY - 1;

const MAX_PARAMETER_QUEUES: usize = 8;
const MAX_POINTS_PER_QUEUE: usize = 32;

#[derive(Clone, Copy)]
pub(crate) struct ParamChange {
    pub(crate) id: ParamID,
    pub(crate) value: ParamValue,
}

struct QueueCell {
    sequence: AtomicUsize,
    value: UnsafeCell<MaybeUninit<ParamChange>>,
}

// SAFETY: the Vyukov-style sequence protocol below synchronizes publication and
// consumption of `value`. At most one consumer (the audio thread) and any
// number of producers participate. `MaybeUninit` means the cell contents are
// only read after the producer published a matching sequence.
unsafe impl Sync for QueueCell {}

/// Bounded lock-free queue of normalized parameter edits.
///
/// `push` never blocks; when the queue is full the edit is dropped. That is
/// preferable to stalling the UI thread and keeps the audio contract intact.
pub(crate) struct PendingParameters {
    cells: [QueueCell; PENDING_CAPACITY],
    enqueue: AtomicUsize,
    dequeue: AtomicUsize,
}

impl Default for PendingParameters {
    fn default() -> Self {
        Self {
            cells: std::array::from_fn(|index| QueueCell {
                sequence: AtomicUsize::new(index),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            }),
            enqueue: AtomicUsize::new(0),
            dequeue: AtomicUsize::new(0),
        }
    }
}

impl PendingParameters {
    /// Enqueues one edit. Returns false when the queue is full.
    pub(crate) fn push(&self, id: ParamID, value: ParamValue) -> bool {
        let change = ParamChange { id, value };
        let mut position = self.enqueue.load(Ordering::Relaxed);
        loop {
            let cell = &self.cells[position & PENDING_MASK];
            let sequence = cell.sequence.load(Ordering::Acquire);
            // Vyukov MPMC algorithm: a matching sequence means the slot is free.
            let difference = sequence.wrapping_sub(position) as isize;
            if difference == 0 {
                match self.enqueue.compare_exchange_weak(
                    position,
                    position.wrapping_add(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // SAFETY: this producer owns the slot until the release
                        // store below. `MaybeUninit` accepts any bit pattern.
                        unsafe {
                            (*cell.value.get()).write(change);
                        }
                        cell.sequence
                            .store(position.wrapping_add(1), Ordering::Release);
                        return true;
                    }
                    Err(actual) => position = actual,
                }
            } else if difference < 0 {
                return false;
            } else {
                position = self.enqueue.load(Ordering::Relaxed);
            }
        }
    }

    /// Pops one edit. Called only from the serialized audio callback.
    pub(crate) fn pop(&self) -> Option<ParamChange> {
        let mut position = self.dequeue.load(Ordering::Relaxed);
        loop {
            let cell = &self.cells[position & PENDING_MASK];
            let sequence = cell.sequence.load(Ordering::Acquire);
            let difference = sequence.wrapping_sub(position.wrapping_add(1)) as isize;
            if difference == 0 {
                match self.dequeue.compare_exchange_weak(
                    position,
                    position.wrapping_add(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // SAFETY: the producer published this slot with a
                        // release store and the consumer now owns it.
                        let change = unsafe { (*cell.value.get()).assume_init() };
                        cell.sequence
                            .store(position.wrapping_add(PENDING_CAPACITY), Ordering::Release);
                        return Some(change);
                    }
                    Err(actual) => position = actual,
                }
            } else if difference < 0 {
                return None;
            } else {
                position = self.dequeue.load(Ordering::Relaxed);
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ParamPoint {
    sample_offset: int32,
    value: ParamValue,
}

struct ValueQueueData {
    id: ParamID,
    count: usize,
    points: [ParamPoint; MAX_POINTS_PER_QUEUE],
}

impl Default for ValueQueueData {
    fn default() -> Self {
        Self {
            id: 0,
            count: 0,
            points: [ParamPoint::default(); MAX_POINTS_PER_QUEUE],
        }
    }
}

/// Host `IParamValueQueue` used as input automation for one block.
struct ParameterValueQueue {
    data: UnsafeCell<ValueQueueData>,
    accepts_output: bool,
}

// SAFETY: the effect's single audio thread resets and fills queues before
// calling `process`; the plug-in only reads them during that call. No other
// thread touches the `UnsafeCell`.
unsafe impl Sync for ParameterValueQueue {}

impl Class for ParameterValueQueue {
    type Interfaces = (IParamValueQueue,);
}

impl IParamValueQueueTrait for ParameterValueQueue {
    unsafe fn getParameterId(&self) -> ParamID {
        // SAFETY: audio-thread-only access contract documented above.
        unsafe { (*self.data.get()).id }
    }

    unsafe fn getPointCount(&self) -> int32 {
        // SAFETY: audio-thread-only access contract documented above.
        unsafe { (*self.data.get()).count as int32 }
    }

    unsafe fn getPoint(
        &self,
        index: int32,
        sample_offset: *mut int32,
        value: *mut ParamValue,
    ) -> tresult {
        if sample_offset.is_null() || value.is_null() || index < 0 {
            return kInvalidArgument;
        }
        // SAFETY: audio-thread-only access contract documented above.
        let data = unsafe { &*self.data.get() };
        let Some(point) = data
            .points
            .get(index as usize)
            .filter(|_| (index as usize) < data.count)
        else {
            return kResultFalse;
        };
        // SAFETY: validated non-null above.
        unsafe {
            *sample_offset = point.sample_offset;
            *value = point.value;
        }
        kResultOk
    }

    unsafe fn addPoint(
        &self,
        sample_offset: int32,
        value: ParamValue,
        index: *mut int32,
    ) -> tresult {
        if !self.accepts_output || index.is_null() || !value.is_finite() || sample_offset < 0 {
            return kInvalidArgument;
        }
        // SAFETY: VST3 calls output AddPoint during the serialized process
        // callback, while this queue is exclusively owned by the audio thread.
        let data = unsafe { &mut *self.data.get() };
        let point_index = data.count.min(MAX_POINTS_PER_QUEUE - 1);
        data.points[point_index] = ParamPoint {
            sample_offset,
            value,
        };
        data.count = (data.count + 1).min(MAX_POINTS_PER_QUEUE);
        // SAFETY: validated non-null above; the host returns the point index.
        unsafe { *index = point_index as int32 };
        kResultOk
    }
}

/// Host `IParameterChanges` containing the edits for the current block.
pub(crate) struct ParameterChanges {
    queues: [ComWrapper<ParameterValueQueue>; MAX_PARAMETER_QUEUES],
    active: UnsafeCell<usize>,
    accepts_output: bool,
}

// SAFETY: see `ParameterValueQueue`; additionally the `active` count is only
// changed by the audio thread before `process`.
unsafe impl Sync for ParameterChanges {}

impl ParameterChanges {
    pub(crate) fn new(accepts_output: bool) -> ComWrapper<Self> {
        ComWrapper::new(Self {
            queues: std::array::from_fn(|_| {
                ComWrapper::new(ParameterValueQueue {
                    data: UnsafeCell::new(ValueQueueData::default()),
                    accepts_output,
                })
            }),
            active: UnsafeCell::new(0),
            accepts_output,
        })
    }

    /// Clears all queues for a new block. Audio thread only.
    pub(crate) fn begin_block(&self) {
        // SAFETY: audio-thread-only access contract documented above.
        unsafe {
            *self.active.get() = 0;
        }
        for queue in &self.queues {
            // SAFETY: audio-thread-only access contract documented above.
            unsafe {
                (*queue.data.get()).count = 0;
            }
        }
    }

    /// Adds one point to the queue for `id`, creating a queue if needed.
    /// Audio thread only; allocation-free and bounded.
    pub(crate) fn add(&self, id: ParamID, value: ParamValue) {
        // SAFETY: audio-thread-only access contract documented above.
        let active = unsafe { &mut *self.active.get() };
        let mut queue_index = None;
        for index in 0..*active {
            // SAFETY: indices below `active` were initialized this block.
            if unsafe { (*self.queues[index].data.get()).id } == id {
                queue_index = Some(index);
                break;
            }
        }
        let queue_index = match queue_index {
            Some(index) => index,
            None if *active < MAX_PARAMETER_QUEUES => {
                let index = *active;
                *active += 1;
                // SAFETY: queue slot is owned by the audio thread.
                unsafe {
                    let data = &mut *self.queues[index].data.get();
                    data.id = id;
                    data.count = 0;
                }
                index
            }
            None => return,
        };
        // SAFETY: queue is owned by the audio thread.
        unsafe {
            let data = &mut *self.queues[queue_index].data.get();
            if data.count < MAX_POINTS_PER_QUEUE {
                data.points[data.count] = ParamPoint {
                    sample_offset: 0,
                    value,
                };
                data.count += 1;
            } else if let Some(last) = data.points.last_mut() {
                // Coalesce when a single parameter exceeds the point budget.
                last.value = value;
            }
        }
    }

    /// Copies the last point for each output parameter into the reverse
    /// lock-free queue. Called on the audio thread after `process`.
    pub(crate) fn publish_latest(&self, destination: &PendingParameters) {
        if !self.accepts_output {
            return;
        }
        // SAFETY: output change storage is owned by the serialized audio thread.
        let active = unsafe { *self.active.get() };
        for queue in self.queues.iter().take(active) {
            // SAFETY: only the audio callback accesses this queue until publish.
            let data = unsafe { &*queue.data.get() };
            if let Some(point) = data.points.get(data.count.saturating_sub(1))
                && data.count > 0
                && point.value.is_finite()
            {
                destination.push(data.id, point.value.clamp(0.0, 1.0));
            }
        }
    }
}

impl Class for ParameterChanges {
    type Interfaces = (IParameterChanges,);
}

impl IParameterChangesTrait for ParameterChanges {
    unsafe fn getParameterCount(&self) -> int32 {
        // SAFETY: audio-thread-only access contract documented above.
        unsafe { *self.active.get() as int32 }
    }

    unsafe fn getParameterData(&self, index: int32) -> *mut IParamValueQueue {
        if index < 0 {
            return std::ptr::null_mut();
        }
        // SAFETY: audio-thread-only access contract documented above.
        let active = unsafe { *self.active.get() };
        let Some(queues) = self
            .queues
            .get(index as usize)
            .filter(|_| (index as usize) < active)
        else {
            return std::ptr::null_mut();
        };
        queues
            .as_com_ref::<IParamValueQueue>()
            .map(|reference| reference.as_ptr())
            .unwrap_or(std::ptr::null_mut())
    }

    unsafe fn addParameterData(
        &self,
        id: *const ParamID,
        index: *mut int32,
    ) -> *mut IParamValueQueue {
        if !self.accepts_output || id.is_null() || index.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: the VST3 contract supplies valid ID and queue-index pointers
        // for the duration of this call.
        let id = unsafe { *id };
        // SAFETY: the active count is owned by the serialized audio thread.
        let active = unsafe { &mut *self.active.get() };
        let found = self.queues.iter().take(*active).position(|queue| {
            // SAFETY: active queues are owned by this process callback.
            unsafe { (*queue.data.get()).id == id }
        });
        let queue_index = if let Some(index) = found {
            index
        } else if *active < MAX_PARAMETER_QUEUES {
            let index = *active;
            *active += 1;
            // SAFETY: new queue slot is owned by the audio thread.
            unsafe {
                let data = &mut *self.queues[index].data.get();
                data.id = id;
                data.count = 0;
            }
            index
        } else {
            return std::ptr::null_mut();
        };
        // SAFETY: validated above.
        unsafe { *index = queue_index as int32 };
        self.queues[queue_index]
            .as_com_ref::<IParamValueQueue>()
            .map(|reference| reference.as_ptr())
            .unwrap_or(std::ptr::null_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_parameters_preserve_order_and_drop_when_full() {
        let queue = PendingParameters::default();
        assert!(queue.pop().is_none());
        for index in 0..PENDING_CAPACITY {
            assert!(queue.push(index as u32, index as f64 / 100.0));
        }
        assert!(!queue.push(999, 0.5));
        for index in 0..PENDING_CAPACITY {
            let change = queue.pop().expect("queued change");
            assert_eq!(change.id, index as u32);
            assert!((change.value - index as f64 / 100.0).abs() < f64::EPSILON);
        }
        assert!(queue.pop().is_none());
    }

    #[test]
    fn parameter_changes_are_bounded_and_report_queues() {
        let changes = ParameterChanges::new(false);
        changes.begin_block();
        for parameter in 0..MAX_PARAMETER_QUEUES + 2 {
            changes.add(parameter as u32, 0.5);
        }
        let interface = changes
            .to_com_ptr::<IParameterChanges>()
            .expect("changes interface");
        // SAFETY: reading our own host object on this thread.
        let count = unsafe { interface.getParameterCount() };
        assert_eq!(count as usize, MAX_PARAMETER_QUEUES);
        // SAFETY: reading our own host object on this thread.
        let queue = unsafe { interface.getParameterData(0) };
        assert!(!queue.is_null());
        // SAFETY: the returned queue is a borrowed pointer owned by the live
        // `changes` object; ComRef does not change its reference count.
        let queue = unsafe { vst3::ComRef::<IParamValueQueue>::from_raw(queue) }.expect("queue");
        // SAFETY: local host object.
        assert_eq!(unsafe { queue.getParameterId() }, 0);
        // SAFETY: local host object.
        assert_eq!(unsafe { queue.getPointCount() }, 1);

        changes.begin_block();
        // SAFETY: local host object.
        assert_eq!(unsafe { interface.getParameterCount() }, 0);
    }

    #[test]
    fn output_parameter_changes_are_published_without_audio_thread_controller_calls() {
        let changes = ParameterChanges::new(true);
        let destination = PendingParameters::default();
        changes.begin_block();

        let id = 0xCAFE;
        let mut queue_index = -1;
        // SAFETY: the output change object owns the returned queue for this
        // block and the pointers below remain valid for each call.
        let queue = unsafe { changes.addParameterData(&id, &mut queue_index) };
        assert!(!queue.is_null());
        assert_eq!(queue_index, 0);
        let queue = unsafe { vst3::ComRef::<IParamValueQueue>::from_raw(queue) }
            .expect("output parameter queue");
        let mut point_index = -1;
        assert_eq!(
            unsafe { queue.addPoint(16, 0.75, &mut point_index) },
            kResultOk
        );
        assert_eq!(point_index, 0);

        changes.publish_latest(&destination);
        let published = destination.pop().expect("published output parameter");
        assert_eq!(published.id, id);
        assert!((published.value - 0.75).abs() < f64::EPSILON);
        assert!(destination.pop().is_none());
    }
}
