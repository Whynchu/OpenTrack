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

    /// Reported processing latency in samples at the prepared sample rate.
    fn latency_samples(&self) -> u32;

    /// Reported tail length in samples at the prepared sample rate.
    fn tail_samples(&self) -> u32;
}
