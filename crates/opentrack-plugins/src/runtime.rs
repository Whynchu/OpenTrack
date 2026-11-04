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

    /// Control-thread description of the main audio outputs exposed by this
    /// processor. The default keeps existing single-output processors source
    /// compatible. Implementations must not query a plug-in ABI from the audio
    /// callback to produce this description.
    fn output_buses(&self) -> Vec<HostedOutputBus> {
        vec![HostedOutputBus {
            index: 0,
            identity_key: String::from("main-output"),
            name: String::from("Output"),
            channels: 2,
        }]
    }

    /// Processes one block and writes main output buses into bus-major left and
    /// right planes. Each plane contains `block_frames * output_bus_count`
    /// samples. The default adapts legacy processors to one stereo bus and
    /// clears extras. Buffers are prepared before callback entry.
    fn process_routed_with_events(
        &self,
        input_left: &mut [f32],
        input_right: &mut [f32],
        output_left: &mut [f32],
        output_right: &mut [f32],
        output_bus_count: usize,
        parameters: &[HostedParameterEvent],
        midi: &[HostedMidiEvent],
    ) -> Result<(), String> {
        let expected_len = input_left.len().saturating_mul(output_bus_count);
        if input_left.len() != input_right.len()
            || output_left.len() != expected_len
            || output_right.len() != expected_len
        {
            return Err(String::from(
                "hosted routed buffers must have equal lengths",
            ));
        }
        if output_bus_count == 0 {
            return Err(String::from("hosted processor has no output destination"));
        }
        let frames = input_left.len();
        output_left[..frames].copy_from_slice(input_left);
        output_right[..frames].copy_from_slice(input_right);
        self.process_with_events(
            &mut output_left[..frames],
            &mut output_right[..frames],
            parameters,
            midi,
        )?;
        output_left[frames..].fill(0.0);
        output_right[frames..].fill(0.0);
        Ok(())
    }

    /// Reported processing latency in samples at the prepared sample rate.
    fn latency_samples(&self) -> u32;

    /// Reported tail length in samples at the prepared sample rate.
    fn tail_samples(&self) -> u32;
}

/// One output bus reported by a hosted plug-in. `index` is the current position
/// in the plug-in's audio output list; `identity_key` distinguishes it from
/// other buses when the plug-in's layout changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostedOutputBus {
    pub index: u32,
    pub identity_key: String,
    pub name: String,
    pub channels: u8,
}

#[cfg(test)]
mod routed_output_tests {
    use super::*;

    struct Gain;

    impl HostedProcessor for Gain {
        fn process(&self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
            for sample in left.iter_mut().chain(right) {
                *sample *= 2.0;
            }
            Ok(())
        }

        fn latency_samples(&self) -> u32 {
            0
        }

        fn tail_samples(&self) -> u32 {
            0
        }
    }

    #[test]
    fn legacy_processor_maps_to_one_routed_bus_and_clears_extras() {
        let processor = Gain;
        assert_eq!(processor.output_buses().len(), 1);
        let (mut output_left, mut output_right) = ([0.0; 6], [9.0; 6]);
        processor
            .process_routed_with_events(
                &mut [0.25; 3],
                &mut [0.5; 3],
                &mut output_left,
                &mut output_right,
                2,
                &[],
                &[],
            )
            .unwrap();
        assert_eq!(&output_left[..3], &[0.5; 3]);
        assert_eq!(&output_right[..3], &[1.0; 3]);
        assert_eq!(&output_left[3..], &[0.0; 3]);
        assert_eq!(&output_right[3..], &[0.0; 3]);
    }
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
