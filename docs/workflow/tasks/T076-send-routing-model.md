# T076 - Send routing model and signal-flow contract

Status: model implemented; audible sends, runtime acceptance, and full UI remain open. Implementer: GPT-6.1 Terra. This is roadmap step 12 and the foundation for steps 13-15.

## User outcome

Each channel can use up to 24 independent send slots, choose a destination, enable or bypass it, adjust level and pan, and choose a pre- or post-fader tap. Existing projects and channel settings must remain readable. Edits participate in save, undo, copy/paste, and the playback plan without blocking the audio callback.

## Contract for the next implementation

- Each slot is bounded to -90 through +12 dB, pan -1 through +1, pre/post fader, and an enable switch. -90 dB means silence; 0 dB means unity. A missing settings entry uses the inactive default: 0 dB, centered, post-fader. Existing destination strings are preserved and are silent until explicitly activated.
- Stereo Out, Left, and Right retain the existing output mix semantics. Group destinations use stable `Group:<track id>` identities; malformed, missing, self-referencing, or cyclic group paths are rejected. Unknown legacy destination strings stay preserved and disconnected rather than becoming an unintended route. Group routes and sends participate in cycle detection even while disabled, so enabling cannot create a feedback loop.
- The pre-fader tap is after channel input processing, EQ, inserts, and delay, before source fader and source pan. The post-fader tap is after source fader and source pan. Both obey source mute and solo gating. Send gain and send pan follow the tap, with the existing channel pan law; the destination's normal bus processing happens once after accumulation.
- Slot removal shifts destinations and parameters together; presets and clipboard copy both. Mutations must be validated and participate in project history. Device input/output bus sends remain unavailable until their signal paths and safe destinations are explicitly implemented and tested.
- Construct and validate the routing graph outside the callback. Render groups in topological order across both outputs and sends; the current deepest-first group order alone does not cover arbitrary send edges. Preallocate all accumulation buffers and avoid allocation, locks, file I/O, or graph traversal in the callback. Live and offline exports must use the same routing semantics, including VST processing and delay.

## Evidence and remaining work

The project schema stores slot-aligned parameters and validates ranges and a combined send/output graph. Project and preset legacy reads use inactive defaults. Channel snapshot, mixer paste, and row removal preserve settings. Automated checks cover legacy reads, round trips, bounds, 24-slot capacity, feedback and channel snapshot behavior. No audio processing or send editing UI has been connected yet; therefore these fields do not make sends audible. Real-time, offline bounce, undo/redo gestures, and hardware listening remain unverified.
