# T043 - VST3 insert hosting

Date: 2026-10-26. Status: implementation in progress.

## User direction

Use the existing eight insert slots on every channel for VST3 plug-ins. Do not
create a separate VST rack or alternate channel path. Built-in effects remain
other choices in the same slot picker. Original media, the accepted Mixer,
Project event behavior, local ASIO and WASAPI remain protected.

## Scope

- VST3 audio effects and analyzers in stereo insert slots.
- Component/controller lifecycle on the control thread, stereo main bus only.
- Block processing through the existing channel strip, after EQ/built-in inserts.
- Automatic delay compensation across tracks with hosted inserts.
- Power/bypass in the existing insert slot, selection and Undo integration.
- Generic parameter list with normalized sliders and gesture-safe controller calls.
- Native floating editor window owned by OpenTrack.
- Component/controller state saved in OPR and restored on reopen.
- Missing/failed plug-in diagnostics without silent fallback.
- Existing insert picker gains `VST3 Plug-in...`.

## Architecture

- `opentrack-core`: persisted `InsertEffectKind::Vst3` with class ID, module hint,
  display names and opaque component/controller chunks.
- `opentrack-plugins`: `HostedProcessor` block trait.
- `opentrack-vst3`: real component/controller instance, buses, block process,
  parameter queues, state streams, native editor view.
- `opentrack-audio`: `HostedInsert` in `PlaybackTrackSettings`; hosted plans use
  64-frame blocks and PDC. Plans without hosted inserts keep the existing path.
- `opentrack-app`: registry keyed by `(track_id, insert_slot, class_id)` so plan
  rebuilds do not recreate plug-ins; module/class picker; parameter/editor UI.

## Not in this milestone

- VST3 instruments, MIDI routing to hosted instruments.
- Multi-out, sidechain, surround, automation lanes or offline bounce.
- True process isolation/sandboxing and per-module scanner timeout.
- Universal compatibility claims.

## Acceptance

1. Add a VST3 effect to an existing insert slot on an audio track.
2. Play audio: the plug-in processes the channel; power/bypass works without a
   separate channel path; fader/pan/meter and Project playback remain correct.
3. Two tracks with different plug-in latency stay aligned.
4. Move a parameter in the generic UI; save/reopen restores it.
5. Reopen the project and restore component/controller state.
6. Open the native editor, change a parameter, close it; audio and saved state
   remain coherent.
7. Undo/redo insert add/remove/bypass and parameter gestures predictably.
8. A missing module shows an explicit failure and preserves the slot/state.

## Verification status 2026-10-26

- Source/compile: app, VST3, hosted audio and core paths compile.
- App suite: 141 passed, 5 ignored installed-plugin tests.
- VST3 suite: 13 passed, 2 ignored installed-plugin tests.
- Hosted block/PDC suite: 6 passed.
- TDR Nova: 75 parameters, 184-sample latency/tail, native editor reported,
  3,608-byte component state, finite block output, fresh-instance restore.
- WaveShell CLA-76 Stereo: loads, enumerates parameters and processes a block.
- Gateway loads despite its optional `setHostContext` returning `kNotImplemented`.
- WaveShell CLA-76 Stereo passes load, audio-thread processing and native-editor open/close.
- x64 local-ASIO build: `BE2E7B5A80D374DE2C535D683D288DA516E78C8A8A6EF6289163E63353CD2BA2`.
- Not yet accepted: live listening, editor visual/interaction review, multi-track
  PDC by ear, save/reopen in the desktop app, and the ignored-plugin slot tests.

## Verification checklist

- `cargo fmt --all`
- `cargo test -p opentrack-vst3`
- `cargo test -p opentrack-audio`
- `cargo test -p opentrack-core`
- `cargo test -p opentrack`
- x64 local-ASIO build
- Real installed plug-in smoke test (TDR Nova / Gateway / WaveShell child)
- User listening/visual review.
