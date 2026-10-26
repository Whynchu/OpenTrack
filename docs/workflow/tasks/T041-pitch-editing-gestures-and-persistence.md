# T041 - Pitch editing gestures, reset, neighbor stretch and OPR persistence

- Status: accepted. Actual implementer/reviewer: GPT-6.1 Sol; no delegated or independent review.
- Scope version: `transport-v16-pitch-time-editing-workflows`.
- Requirement IDs: `note-body-pitch-modes`, `pitch-note-marquee-reset`, `pitch-note-edge-push`, `pitch-note-double-click-reset`, `pitch-edit-opr-roundtrip`.
- Desired observable outcome: Shift held before or pressed during a body drag makes continuous sub-semitone pitch changes without a mode-switch jump and shows cents from the nearest semitone. Dragging empty Sample Editor grid draws a marquee selecting every intersected note. Reset applies to all selected notes as a single reversible edit, preserves analysis and keeps neighboring regions valid. Stretching one note boundary moves an attached neighbor's nearest edge inward or outward while its far edge remains fixed; existing gaps stay open until crossed and both notes retain 10 ms minimum lengths. Double-clicking an edge restores that edge to its source boundary; double-clicking the body restores analyzed pitch and source timing. OPR save/reopen restores exact note IDs, source/destination bounds, splits, pitch/straighten values, analysis and enabled state without scheduling analysis again.
- Reference evidence: Cubase SE-05 `C:/Users/Samue/Pictures/Screenshots/Screenshot 2026-10-25 090626.png` and OpenTrack baseline `C:/Users/Samue/Pictures/Screenshots/Screenshot 2026-10-25 180823.png`; captures show note regions and editor organization, but do not establish these gestures or synthesis quality. Fine pitch, marquee/reset and ripple-neighbor rules are user-specified. No Cubase interaction was repeated.
- Dependencies and decisions needed: none for Batch 1. Pitch synthesis quality and adjoining-edge crossfades remain separate Batch 2 work, gated on the user's review of Batch 1. Project scissors, wheel remapping and Mixer fader/selection remain Batch 3, gated on Batch 2 listening review.
- Code scope: `pitch_editing.rs`, Sample Editor footer/readout and tests, ProjectFile round-trip test, workflow task/ledger and Sample Editor study. Keep Rust ownership and existing original-source playback. Do not modify the user's EXAMPLE project or audio, accepted Mixer styling, ASIO selection, or DSP in this slice.

## Acceptance checks

- EgUI pointer tests: Shift held before a drag and Shift pressed during an existing body drag both produce continuous fine pitch changes. Pressing Shift after an initial coarse movement does not jump; subsequent half-row movement produces a continuous 12.5-cent correction and a nearest-semitone cents readout.
- EgUI pointer test: blank-grid drag selects two note boxes; clicking/selection alone does not publish a project edit or create Undo history.
- Reset selected notes with two selected and one adjoining unselected note; both selected notes return to their analyzed/source boundaries and pitch state, only the unselected note's closest edge moves, its far edge remains fixed, and one Undo restores the full pre-reset state.
- EgUI pointer tests: left and right stretches propagate to an attached neighbor's closest edge both when extending into it and when pulling away; verify far edges, original source bounds, pre-existing gaps, 10 ms minimum lengths and valid ordered note state.
- EgUI pointer tests: double-clicking an edge resets only that boundary to its source position; double-clicking a note body resets pitch/straighten and destination bounds while preserving analysis. Each reset publishes one undoable edit.
- Create a temporary OPR with analyzed, split, pitched and retimed notes; write/read with the native ProjectFile; open the restored clip in Sample Editor; assert exact values and that PitchTask remains absent. Never use the saved EXAMPLE file.
- Release workspace tests and x64 app build with local ASIO enabled. Check `build-info.json`, process state before build, formatting, diff and parity ledger.
- User live review: open the rebuilt app and a disposable/test project, verify Shift-before and Shift-during drag, no pitch jump when Shift is pressed mid-gesture, cents readout, note marquee, multi-note Reset/Undo, neighbor stretch in both directions, edge/body double-click reset and Save/close/reopen persistence. Close OpenTrack through its UI afterward. Audio-quality listening remains pending for Batch 2.

## Handoff

- Actual model: GPT-6.1 Sol; no subagents or independent review.
- Current x64 local-ASIO executable: `4D34F3EEB6054E82B1CEBD722BD38F5C618B94A3DB45B14A32CE3285867D400D`; `build-info.json` confirms local ASIO is enabled.
- Changed-source identity: `pitch_editing.rs` SHA-256 `158E692A06D6281B80BEDE71A2F36219DA1CE22750E3161DAFC9BDB98436B2B2`; `sample_editor.rs` SHA-256 `0DEA45F5F20B66144626D1A3C5A7BA442AA36929D443E4B804E928E71E778E48`.
- Checks: `cargo fmt --all --check`; full `cargo test --release --workspace --features local-asio --target x86_64-pc-windows-msvc` passed (124 app, 58 audio, 16 core). All 11 focused Sample Editor note-editing tests pass, including mid-drag Shift, double-click reset, both directions of adjacent stretch, gaps and the OPR round trip. `python scripts/parity_report.py --check` and `git diff --check` pass. Logs: `builds/x64/T041-r2-workspace-stdout.txt`, `builds/x64/T041-r2-focused-stdout.txt` (ignored local build output).
- Test executable SHA-256: `F78B16E7BCE066924BC8F74F893ABDD4AA1C1A8FECD608B13B3AEF1146B1D00A`.
- OpenTrack was verified closed before build and remains closed. No live screenshot, ASIO listening, desktop check, saved EXAMPLE interaction or Cubase access occurred.
- User acceptance: after reviewing the T041 build, the user confirmed “I verify!! great work!” and then confirmed OpenTrack was closed. This accepts the Batch1 review gate; individual action-by-action results and visual comparison were not itemized, so reference parity remains unclaimed.
- Ledger requirements updated: `note-body-pitch-modes`, `note-gesture-history`, `note-click-boundary-stability`, `pitch-note-marquee-reset`, `pitch-note-edge-push`, `pitch-note-double-click-reset`, `pitch-edit-opr-roundtrip`; Batch2/3 gaps remain explicit in the denominator. Scope is `transport-v16-pitch-time-editing-workflows`; see `docs/workflow/metrics.md`.
- Automated egui/OPR checks are not desktop or listening acceptance. No visual or audible parity claim is made here.
- Batch 1 review is resolved by user acceptance. Continue with T042 Batch 2; stop for controlled-vocal listening acceptance before beginning Batch 3.
