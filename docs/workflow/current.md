# Current assignment

Updated: 2026-10-26. Actual implementer: Codex session; no independent review.

## Current review - T042 Batch 2

Batch 1 T041 was accepted by the user. After listening to the first T042 iteration, the user asked for pitch targets to blend toward adjacent notes inside the notes themselves, with a maximum reach of one sixteenth note, and reported bubbly/choppy sound when shortening notes.

The renderer now shares an event-wide source-pulse map and continuous output clock across note cuts. It reacquires real waveform peaks at voiced onsets, adapts overlap windows to source harmonic content and time expansion/compression, and uses the same tempo-aware pitch contour in playback and the Sample Editor. Edited and adjacent unedited notes ease toward a shared boundary pitch for up to one sixteenth-note on each side.

The 2026-10-25 audit found a hard wet-to-dry switch at the inner endpoint of an unedited neighbor's rendered edge. The fix adds a 5 ms endpoint crossfade, shortens the audio handoff to 5 ms while keeping the long pitch ramps, and makes one-unchanged-note pairs resolve to the unchanged edge pitch. Sample-continuity tests cover both orientations. Headless synthetic and read-only Vocal 01 measurements now pass; user listening is still pending.

This checkpoint also adds semitone-row note colors, track-color cohesion for meters/EQ/spectrum with blue default Stereo In/Out, and a dedicated note-drag audition voice that works while the transport is stopped without moving the playhead.

## Build and checks

- x64 release build with local ASIO succeeded; WASAPI remains available. OpenCode's completed Batch 3 and VST3 work is included.
- Executable SHA-256: `ECB1A99A63297037933EED125AD2FECE0CBA449BFCE6502FF9CFB2C14FE707AC`.
- Full Windows x64 workspace tests pass: app 136, audio 78, core 18, plug-in scan 16, plug-ins 11, VST3 13; hardware/manual VST smoke tests remain ignored.
- Headless production-renderer suite: 6 passed; 1 explicit-source test ignored in the ordinary run. Read-only Vocal 01 matrix passed separately. Details and artifacts are in T042.
- OpenTrack was checked closed before the build and was not launched afterward. No desktop, device, or user-listening review has been performed.

## User review checkpoint - pending

The user will review pitch quality with their own audio. The current build also includes note colors, track-color cohesion (including blue default Stereo In/Out and the Mixer color picker), note-drag audition, and Batch 3.

Batch 3 implementation is complete in source: contextual Project cut band, snap-aware cursor/guides, Ctrl+wheel horizontal pan, rectangular Mixer fader caps, and shared Mixer/Project track selection. Its x64 build and live interaction/visual review are pending. Keep the app closed until the user chooses to open it.

## VST3 lane - insert hosting source complete; review pending

VST3 class enumeration is committed as `8ce842c`. T043 now wires real VST3 effects into the existing eight insert slots: component/controller lifecycle, stereo main buses, 64-frame hosted processing, PDC, parameter changes, opaque state, generic sliders and a native editor window. Plans without hosted inserts retain the original per-sample path; built-in effects remain alternative insert choices.

TDR Nova and a WaveShell CLA-76 Stereo child load and process in the smoke checks. OpenCode completed and pushed the current VST3/Batch 3 source; `9f4dde1` is on `origin/main`. The combined x64 local-ASIO build is `ECB1A99A63297037933EED125AD2FECE0CBA449BFCE6502FF9CFB2C14FE707AC`. VST3 instruments, multi-out/sidechain, sandboxing, catalog UI and per-module scanner timeout remain pending. See [T043](tasks/T043-vst3-insert-hosting.md), [vst3-status](vst3-status.md) and [plugin-hosting-plan](../plugin-hosting-plan.md).

## Preserved direction

Keep pitch analysis/editing/processing in Rust, preserve original audio and the accepted Mixer, and maintain the gated sequence: Batch 1 accepted; Batch 2 listening review now; Batch 3 Project scissors/wheel behavior and Mixer fader/selection only after Batch 2 acceptance. Scope remains `transport-v16-pitch-time-editing-workflows`; workflow readiness and reference parity remain separate from implementation and build success. See [metrics](metrics.md) and [T042](tasks/T042-vocal-quality-and-note-transitions.md).
