# T047 - Pitch edge handles and Project take lanes

- Status: implementation complete; x64 build passed; user review pending. Implementer: Codex session; no independent review.
- Scope version: `transport-v17-edge-pitch-take-lanes`.
- Requirement IDs: `pitch-note-edge-bend-anchor`, `pitch-note-bottom-time-handles`, `project-track-take-lanes`.
- Desired outcome: Pitch-note top corners bend the note's beginning/end relative to a selectable anchor on the top guide. Separate squares below the note move its left/right timing edges. Corner hit areas stay clear of broad body-pitch dragging. The Project Take Lanes control expands overlapping events into vertically stacked rows, with playback priority represented by the bottom lane, and the old Project Read/Write controls are replaced by Effects and Take Lanes.
- Feature decisions: user direction in `docs/workflow/README.md#t047-direction-2026-10-26`; implementation baseline is the current Sample Editor and Project track view. The lane ordering follows the existing playback rule: the last active event in track order wins an overlap, with existing intentional crossfades retained.
- Dependencies and decisions: additive pitch fields default old OPR notes to a neutral 0.5 anchor and zero bends. Take-lane indices are derived deterministically from overlapping audio events and track order; only the expanded/collapsed view is stored in ArrangeViewState. The control is disabled on non-audio tracks. No separate lane-selection or event-reordering command is introduced.
- Plan: (1) extend the shared pitch model with a persisted piecewise contour around a movable anchor; (2) add top pitch squares, an anchor guide, and separate lower timing squares while preserving body tuning and existing edge resize rules; (3) expand Project rows and place overlapping events according to the same last-active-event priority the mixer uses; (4) replace the Project header's Read/Write controls with Effects and Take Lanes; (5) build in an isolated x64 output directory and leave visual, listening, and save/reopen review to the user.
- Code scope: `pitch_time.rs`, `pitch_editing.rs`, Sample Editor contour consumers, Project track header/layout, audio-event drawing, ArrangeViewState, workflow task and ledger. Preserve original media, T042 pitch synthesis and current Project/Mixer behaviors outside the requested controls.

## Acceptance checks

- In Sample Editor, verify the top horizontal guide selects the anchor without starting a broad pitch drag; its marker follows the clicked position.
- Drag the top-left/top-right squares up and down. Each changes only its corresponding pitch edge, the contour passes through the anchor, body pitch tuning remains reachable, and stopped-transport audition and playback use the same contour.
- Drag the bottom-left/bottom-right squares horizontally. They change only the note start/end, preserve snapping and attached-neighbor rules, and do not alter edge pitch.
- Verify pitch-edge values, anchor position, Undo/Redo, split, Reset, OPR save/reopen and old OPR notes with missing added fields.
- In Project, toggle Take Lanes on a track with overlaps. Verify lane rows expand in event priority order, the bottom overlapping lane is the playback winner, the view restores after reopen, and the E button opens track effects while the lane button toggles the extension.
- Run the x64 local-ASIO release build in a task-specific output directory so the existing `builds/x64/OpenTrack.exe` is preserved. Do not launch OpenTrack without a separate desktop review.
- User visual, interaction, listening and save/reopen acceptance remain required; compilation alone is not acceptance.

## Handoff

- Actual model: GPT-6; no subagents.
- Source identity/build hash: `opentrack.exe` SHA-256 `3E1A39B3D90EF52CE8688BBC3C387706265E6868B395A680484F7904CD457411`; source hashes are recorded in ledger evidence `t047-source`. Output: `builds/x64/T047-build/x86_64-pc-windows-msvc/release/opentrack.exe`.
- Changes and reasons: note edits now retain start/anchor/end bend offsets and the renderer/editor consume the same contour; the four separated handles preserve pitch/body and timing gestures; Project audio overlap rows follow track clip order so the latest active clip is lowest within its overlap stack; non-audio tracks disable the Take Lanes control; the header E control opens Channel Settings with Inserts as the first visible section.
- Commands/checks and actual results: `cargo fmt --all` succeeded; `cargo fmt --all -- --check` succeeded; `cargo build --release -p opentrack --features local-asio --manifest-path Cargo.toml --target x86_64-pc-windows-msvc --target-dir builds/x64/T047-build` succeeded (optimized release, 36.23 s); ledger JSON parsed successfully. No tests were run.
- Runtime environment and evidence paths: Windows x64, local-ASIO feature; no desktop session or user media was used. OpenTrack was not launched and no OpenTrack process was present after the build. Existing `builds/x64/OpenTrack.exe` was not replaced.
- Ledger IDs updated; report command/result: T047 requirements, workflow and pending acceptance gates are recorded; `python scripts/parity_report.py` succeeded, validated `ledger.json`, and wrote `docs/workflow/metrics.md`.
- Review findings, rework, elapsed effort if available: no independent review.
- Failures/unknowns/pending acceptance: user visual and live gesture review, listening, Undo/Redo and old/new OPR save/reopen review, and Project take-lane playback review remain pending. Compilation alone is not acceptance.
- Next ready task: user review of the built artifact; T042's separate listening acceptance remains pending.
