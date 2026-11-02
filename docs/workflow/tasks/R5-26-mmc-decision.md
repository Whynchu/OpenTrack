# R5-26 — MMC evaluation

- Status: accepted (decision record; no implementation)
- Scope version and requirement IDs: revision 5, step 26
- Desired observable outcome: a recorded decision on MMC master/slave support with the exact missing capability.
- Reference evidence (IDs/paths): `crates/opentrack-audio/src/midi.rs` (input only: `MidiInputDevice`, `MidiInputStream`, `MidiInputMessage`); `handle_midi_input_events` in `app.rs`; the Devices/MIDI menus.
- Dependencies and decisions needed: a MIDI *output* path and a device to test against.
- Code scope and architecture boundaries: none.

## Finding

OpenTrack has no MIDI output: the MIDI module opens input streams (WinMM `midiIn`), and live MIDI is routed to the internal monitor instrument, not to a port. MMC is a transport-control protocol carried over MIDI, so both master and slave modes require a new output stream, message encoding (MMC sysex), device enumeration for destinations, and a hardware loop test that cannot be performed here.

## Decision

Keep `MMC Master` and `Project Synchronization Setup` unavailable and record the requirement: implement only after a bounded MIDI output path exists (destination selection, bounded queue, failure reporting) and a second machine or loopback device is available to verify start/stop/locate and clock behaviour. No placeholder dispatch and no code change in this revision.
