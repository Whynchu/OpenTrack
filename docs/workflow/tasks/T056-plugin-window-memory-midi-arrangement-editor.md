# T056 — Plug-in window memory and MIDI arrangement/editor workflow

- Status: in_progress
- Scope version and requirement IDs: v1; user request (2026-10-27).
- Desired observable outcome: VST Audio Channel Settings returns to its previous position after close/reopen; native VST3 editors retain their position per project workspace and remain above OpenTrack's Project and lower-zone panels; the Project pencil draws a MIDI part on MIDI/Instrument tracks, and opening its editor selects a MIDI Editor tab beside Mixer and Audio Editor in the lower zone.
- Reference evidence: existing `.opr` workspace snapshots already persist VST Audio Channel Settings bounds. Existing MIDI parts, note editing, part selection, and double-click-to-open exist; the piano-roll is currently a separate top-level viewport. User-directed behavior defines the interaction.
- Dependencies and decisions needed: none. A pencil drag defines the part's start and duration on the existing project snap grid; a click creates a one-grid part. Creation leaves the part unselected and does not change the open lower-zone tab; double-click selects the part and opens it in the docked MIDI Editor.
- Code scope and architecture boundaries: app window adapters and VST3 native host; preserve project-owned MIDI part/note data and `Project::add_midi_part`; keep desktop coordinates in workspace window snapshots, not core project state; reuse the existing piano-roll view and edit commands. Do not add dependencies or change MIDI playback/routing.
- Acceptance checks (action, expected result, fixtures, evidence to retain): (1) move and close VST Audio Channel Settings, reopen it, and verify same position; save/reopen project and repeat; (2) move a VST3 editor, close/reopen it, verify same position and that Project/Mixer/Audio Editor cannot cover it while OpenTrack is active; (3) with Draw selected, drag on empty MIDI and Instrument rows and verify snapped part bounds and note preview; click creates one grid unit; the created part remains unselected and the current lower-zone tab does not change; Select tool does not create parts; (4) double-click a part and verify it becomes selected and MIDI Editor opens as a lower-zone tab beside Mixer and Audio Editor; notes edit and persist in the same part; undo/redo and project save/reopen retain part and notes; (5) remove the separate MIDI editor viewport and check no stale shortcut/window behavior.
- Affected regressions: lower-zone tab geometry and mixer/audio editor input; existing MIDI recording and MIDI note editing; project workspace serialization and close/save behavior; VST3 window owner/z-order, resize and message pumping; undo/dirty state.
- Essential gates: compile and targeted command/persistence checks; live UI validation of both window position and z-order, MIDI drawing, docking, note editing, and save/reopen. Runtime and visual acceptance remain user-owned.
- Review owner and milestone stop: user reviews the x64 build; do not claim the desktop behavior accepted from code/build checks alone.

## Handoff

- Actual implementer/reviewer model: GPT-6 Luna.
- Source identity (commit plus dirty diff/artifact identity) and build hash: workspace source `app.rs` pencil-create selection behavior (repository commit not recorded); x64 executable SHA-256 `346747147196593CF3202966D63FBC59D602E94E8C093EAB8D8521686F5D2ECC`.
- Changes and reasons: pencil-created MIDI parts no longer select themselves or open the MIDI Editor; preserve the current track/part selection and tab, as requested. Double-click continues to select/open.
- Commands/checks and actual results: `cargo fmt --all` and `cargo check --offline -p opentrack` were denied under the initial read-only sandbox. After explicit user authorization, `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64` succeeded; release completed and packaged `builds/x64/OpenTrack.exe` at 2026-10-27 10:39:59 PM.
- Runtime environment and evidence paths: Windows x64; live review pending.
- Ledger IDs updated; report command/result: pending.
- Review findings, rework, elapsed effort if available: pending.
- Failures/unknowns/pending acceptance: no live GUI interaction check; user should verify new parts remain unselected and double-click opens the docked MIDI Editor. The source formatting/check commands were not separately rerun after the successful release build.
- Next ready task: user review after implementation.
