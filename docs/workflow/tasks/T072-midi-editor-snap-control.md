# T072 - Make MIDI editor Snap control effective

- Status: implemented; automated checks and release compilation passed; runtime interaction review pending.
- Scope version: 1.
- Roadmap: first small visible-control feature after source inventory.
- Desired outcome: MIDI editor Snap checkbox controls whether note/controller positions snap to the selected position grid; the choice persists as an application preference.
- Existing behavior: checkbox changed a local per-frame variable, while all editor operations snapped unconditionally to the grid.
- Change: add `midi-editor-snap-enabled` app-storage preference (default true), pass it into the MIDI editor, and use the chosen grid when enabled or floating-point precision when disabled for position-based quantization. Note-length dropdown remains separately controlled.
- State boundary: app preference only; no project schema or audio-thread change. Existing position and length grid preferences retain their keys.
- Acceptance checks: toggle Snap off and draw/move/resize/split notes and edit/add controller events at off-grid positions; re-enable and verify each action snaps to the selected position grid. Restart and confirm the choice persists. Verify length-grid behavior remains independent and existing MIDI project data is unchanged.
- Automated verification: after extracting preference ownership into T073, `cargo test -p opentrack --bin opentrack midi_editor_settings::tests` passed (3/3); the existing MIDI editor recovery render test passed (1/1); `cargo fmt --all -- --check` and `git diff --check` passed.
- Build: `build.ps1 -Architecture x64` compiled successfully with local ASIO enabled. Packaging could not replace `builds/x64/OpenTrack.exe` because an OpenTrack process was running. The newly compiled candidate remains at `.build-cache/package-staging/x64/OpenTrack.exe`, SHA-256 `39D9215497EA9D31F9A57C771A27B296511EDCC3F904598FA1E21076B182A7C3`; the distributable folder still has the earlier `E4798EC3593DAD4BD17A2C1A4984613F48AE580437A29A95ADD8FF67BAD33406` binary.
- Runtime/user review: pending. Native desktop control is unavailable in this session, so note/controller gesture behavior and preference persistence across restart were not independently exercised.
- Known limitation: the toggle applies to position quantization; note length continues to use the separate length grid.
