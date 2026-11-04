// SPDX-License-Identifier: Apache-2.0

//! Format-neutral block processing contract.
//!
//! `opentrack-audio` calls these methods from the serialized audio callback.
//! Implementations must not allocate, lock, perform I/O, or call controller or
//! editor methods. Control-thread work belongs to the format backend.

/// One prepared hosted processor used by the block graph.
///
/// The object is shared by playback plans and may be called while a controller
/// or editor is used on the UI thread. Implementations must keep those paths
/// separate, as required by the format ABI.
pub trait HostedProcessor: Send + Sync {
    /// Processes one stereo block in place. `left` and `right` have the same
    /// length and never exceed the prepared maximum block size.
    fn process(&self, left: &mut [f32], right: &mut [f32]) -> Result<(), String>;

    /// Processes a block with normalized parameter points at sample offsets.
    /// The slice is bounded by the host and ordered by offset. Implementations
    /// that do not support automation may ignore it and use process().
    fn process_automated(
        &self,
        left: &mut [f32],
        right: &mut [f32],
        _events: &[HostedParameterEvent],
    ) -> Result<(), String> {
        self.process(left, right)
    }

    /// Processes a block with bounded MIDI note events and parameter points.
    /// Effect processors can ignore MIDI; instrument processors should consume
    /// it without allocating or calling controller/editor methods.
    fn process_with_events(
        &self,
        left: &mut [f32],
        right: &mut [f32],
        parameters: &[HostedParameterEvent],
        _midi: &[HostedMidiEvent],
    ) -> Result<(), String> {
        self.process_automated(left, right, parameters)
    }

    /// Reported processing latency in samples at the prepared sample rate.
    fn latency_samples(&self) -> u32;

    /// Reported tail length in samples at the prepared sample rate.
    fn tail_samples(&self) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostedParameterEvent {
    pub parameter_id: u32,
    pub sample_offset: u32,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HostedMidiEvent {
    pub sample_offset: u32,
    pub note_id: i32,
    pub channel: u8,
    pub pitch: u8,
    pub velocity: u8,
    pub note_on: bool,
}
