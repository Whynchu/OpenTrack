# Devices > Control Room (monitor preferences)

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: Control Room Mixer and Control Room Overview open one window with persisted listen-source and talkback preferences plus a read-only routing readout, without claiming monitor-bus DSP that does not exist.
- Reference evidence: `readout`, `open_control_room`, `set_monitor_listen`, `toggle_monitor_talkback`, `show_control_room` and the tests in `crates/opentrack-app/src/app/control_room.rs`; storage keys `control-room-listen-v1` and `control-room-talkback-v1`.
- Dependencies: none for the preferences. A real monitor bus/talkback input needs engine work and a device to verify.

## Scope decision

OpenTrack has no separate monitor bus, cue sends or talkback input. Rather than wiring the rows to a fake mix, the window states the limitation and stores what can be honest today:

- **Listen source:** one channel (any non-folder track or bus) or Off, persisted by id across restarts. A stale id resolves to "not in this project" instead of inventing a channel.
- **Talkback:** a persisted on/off preference labelled "preference only; no input DSP".
- **Mixer tab:** the listen picker, talkback checkbox, and the selected source's kind, output route and incoming send count.
- **Overview tab:** master output name, group/FX/send counts and the current listen/talkback state.
- Setting a channel that is not in the project is refused; clearing the listen source is allowed.

## Verification

- `cargo test -p opentrack control_room` -> 3 passed: readout resolution and counts with a stale id resolving to nothing; listen validation, clearing and the talkback toggle plus tab selection; the window drawing its title, Listen, Talkback and the selected FX name.
- Inventory regenerated: 277 leaves, **55 available**, 174 state-dependent, **40 unavailable**, 8 informational (both Control Room rows were unavailable).
- App suite 620 passed / 4 known device-dependent roll failures / 9 ignored (633 tests).

## Not claimed

No monitor-bus audio, cue routing or talkback input. The preferences are stored and shown, not applied to the output path; saying otherwise would be false.
