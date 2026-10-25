# T026 - Shared loop display, true pointer and group editing

- Date:2026-10-25. Status:implementation complete; live review pending user. The user resumed T026 after the plan-only checkpoint.
- Author:Codex current session; no model switch/subagents or independent review claimed.
- Scope:event-edit-v6; true-mouse-crosshair, shared-editor-cycle-display, arrange-marquee-selection, atomic-event-group-move.
- Preserve the delivered T025 build and pending review of file recovery/rate changes. User says Sample Editor controls feel great and approves snapped scissors placement; this is partial feedback, not an end-to-end milestone acceptance.

## Resampling clarification

A1-second44.1kHz recording contains44100 samples. A48kHz representation of that same1second needs48000 sample values. Resampling estimates values at the new timestamps; duration and pitch stay the same. It does not recover extra detail from the original recording. Simply playing the original44100 samples at48000 samples/second would instead make the duration0.91875seconds and raise pitch. That is speed/rate reinterpretation, not the intended project-rate behavior.

Current OpenTrack distinguishes source file rate, project sample grid/new-recording rate and active device/output rate. Existing media is already read at its own rate and interpolated against the active output clock. Project-rate changes do not rewrite existing WAVs or necessarily change the device clock. Therefore a clip not getting shorter is correct, and may have sounded identically rate-conformed before the project setting changed. Keep this accurate in user-facing descriptions; do not claim offline file conversion. The prior44.1/48/96kHz tone checks remain evidence for timing/pitch playback behavior, not a new file rewrite in this plan.

Primary supporting explanation: [Audacity Tracks Menu - Resample](https://manual.audacityteam.org/man/tracks_menu.html). Existing source evidence:T025 packet/source hashes and mix_track_frame source-time conversion. No audio-engine change is needed for this clarification.

## Evidence and source audit

User-specified: retain a native crosshair at true mouse position while scissors/guide remain snapped; show the shared loop field on the Sample Editor timeline; drag a box from Project void to select multiple recorded items and move them as a group. Prior clarification distinguishes recorded event rectangles from whole track rows. This plan interprets group selection as audio events across lanes; row selection/reordering stays separate. Exact new marquee modifier/cross-track behavior is our proposed design, not an observed reference capture.

Source findings:
- system_tool_icons::tool_cursor_at first requests native Crosshair but hides it for valid scissors artwork. Retain Crosshair; do not warp the OS pointer. Continue painting the Windows scissors glyph at the independently computed cut hotspot.
- Sample Editor shares Project transport but has no loop/cycle drawing. Project owns loop_start_beats/loop_end_beats/cycle_mode; enabled cycle state is in shared TransportState. No new persistent loop copy is needed.
- Audio-event state already supports multiple selected IDs for X/Delete, but void press clears selection and there is no marquee gesture.
- Existing drag state holds one original/candidate event. Existing edit_audio_clip validates and commits one event. Group movement requires one validated batch command, not a loop that can leave partially applied moves.
- Arrange grid also handles seeking, middle pan, ruler gestures, headers, automation and scrollbars. Void drag must be explicitly arbitrated so it cannot seek or pan at the same time.

## Slice A - Native pointer and shared loop display

1. Keep native OS Crosshair at raw mouse coordinates in Project and Sample Editor scissors mode, with snap on/off. Scissors glyph and cut guide remain anchored to the exact snapped/sample-aligned cut position. Do not move the real pointer or change cut placement.
2. Read existing Project loop/skip bounds/mode and shared Cycle enabled flag each frame. Paint the same region in Sample Editor ruler, faintly through waveform and in overview. Use existing purple repeat/orange skip palette, dim when disabled, matching boundary markers and clipping.
3. Convert global beats through event start and view zoom/scroll; source trim offset must not shift the global range. Ranges can be partly or wholly outside the edited event; never clamp/mutate shared bounds merely to fit the source.
4. This slice synchronizes display. Retain current Sample Editor ruler seeking/zoom and independent snap; adding loop draw/move/resize gestures there is a separate follow-up unless requested. Source Project remains the range editor, and updates appear immediately in both views.

Acceptance: raw pointer and snapped guide visibly separate between grid lines; no-snap guide follows sample-aligned pointer; cut matches guide. Test repeat/skip, Cycle on/off, range changes in Project, nonzero event position/source offset, zoom/pan, partial/offscreen ranges and overview. Opening/changing views does not dirty Project, alter locators or change playback behavior. No extra schema fields.

## Slice B - Void-drag box selection

1. Object Selection tool only: primary press in empty arrange space enters a pending gesture. Empty media-lane areas and blank grid below tracks qualify; rulers, track list, scrollbars, events/handles, active automation controls and floating panels keep priority.
2. Drag past a small explicit threshold to activate a translucent selection box. Plain drag replaces event selection; Shift adds; Ctrl toggles hits. These modifier choices are proposed OpenTrack defaults and require behavioral review.
3. Select audio events whose rectangles intersect the box, across multiple lanes. Highlight whole events, preserve stable IDs and keep track-row selection a separate target. MIDI-part marquee participation is outside this first audio-event slice; do not break existing MIDI hit targets.
4. Store the anchor in logical timeline/content coordinates so scrolling does not move it. Paint clipped to the arrange viewport; allow held-edge autoscroll with correct selection after viewport movement.
5. Delay existing void-click behavior until release when threshold was not crossed. Marquee must not seek/zoom/pan the transport. Escape cancels and restores previous selection. Selection alone creates no undo entry or dirty state.

Acceptance: drag all four directions; select partial/full intersections across lanes; start below last track; empty click, additive/toggle selection, Escape, scrolled view, held-edge movement, automation boundaries and excluded hit targets. Existing single-event/handle/ruler/middle/right-click behaviors remain intact.

## Slice C - Atomic group movement

1. Dragging the body of any selected event moves the selected group; grabbing an unselected event without modifiers replaces selection and retains single-event behavior. Trim/fade/gain handles keep their current per-event operation.
2. Capture every member's original event, source track and stable ID. Snap the dragged reference event using the Project grid and apply one common beat delta to the group; preserve all relative timing. Bound the common delta to prevent any negative start. At beat0, preserving group spacing/project bounds takes priority if an off-grid group cannot also snap its reference exactly.
3. Horizontal group movement works across mixed audio widths. For a vertical move, preserve relative row offsets and validate every destination: existing audio track, matching each member's channel width and in range. Reject the whole drop if any destination is invalid. No automatic track creation or mono/stereo conversion in this slice.
4. Preview all originals as ghosts and candidate positions as opaque, retaining track/event colors. Invalid groups show unavailable feedback; no partial model changes during preview. Edge scrolling uses the same logical coordinate conversion and held-pointer repaint.
5. Add one application/core batch edit command that validates the entire candidate project before mutation. On release commit once, rebuild audio once and create one Undo step. Preserve source paths, durations, source offsets, gains, local/inherited fades and pool membership. Escape/invalid drop leaves the document/files untouched.

Acceptance: noncontiguous events/lanes, shared sources, mixed mono/stereo horizontal moves, compatible vertical shifts, invalid/instrument/out-of-range targets, snap on/off, off-grid original spacing, beat0 limit, scrolling, stationary edge hold, Escape and one-step Undo/Redo. Save/reopen preserves moved positions; original WAV bytes and pool membership remain unchanged. Subsequent X/Delete/Shift+Delete still use the selected IDs and shared-source protections.

## Architecture and implementation order

Rust only; no new dependencies expected. Gesture/preview state stays in app audio_events; stable IDs and batch validation belong to core/application command layer. Loop rendering consumes shared model state; no duplicated persistent transport/range. Do not refactor unrelated Mixer, audio engine, file recovery or Sample Editor controls.

Implement/test Slice A, then B, then C as bounded steps, recording results and unresolved cases. Release gate is the combined box-select/group-drag/Undo scenario plus crosshair/range checks; a build alone is not acceptance. Preserve T025 file recovery and all accepted Project/Mixer/ruler/recording interactions.

Checks: meaningful headless pointer tests, atomic command/boundary tests, unchanged Project/bytes for canceled/selection gestures, persistence and affected regression suite. Close old app before any later build. Desktop comparison only when authorized/free; keep one instance and close when done. Record resolution/scaling, positions/panels/theme, source/reference IDs, current image and match status; comparisons outside Git. Stop at user review before another substantial stage.

## Implementation handoff

- Actual implementer:Codex current session; no model switch/subagents or independent review claimed.
- Pointer: scissors mode leaves the native OS Crosshair visible at the actual pointer while continuing to paint the Windows scissors glyph and snapped cut guide at the sample-aligned location. No cursor warping or cut coordinate change.
- Shared loop display: Sample Editor reads Project locator beats, loop/skip mode and shared Cycle enabled state each frame. It paints a faint mode-colored range through waveform and overview, clipped to those views, with locator lines/flags in the ruler. Beat-to-event-local conversion does not use source trim offset. Drawing does not change loop data, seek state or project dirty state.
- Marquee: Object Selection primary-drag from blank arrange space creates a translucent box and selects intersecting audio events across rows. Plain replaces, Shift adds, Ctrl toggles; Escape restores the prior selection. Empty click clears selection. Track automation, MIDI parts and audio events retain their own hit targets. The anchor uses content time/vertical scroll coordinates, with held horizontal edge scrolling.
- Group move: drag any selected event body. One common snapped beat delta preserves event spacing; vertical row offset is preserved and every destination must be an existing Audio track of matching channel width. Preview ghosts originals and paints candidates; no Project mutation occurs before release. The core batch API validates identities, widths, finite project data and destinations on a candidate Project, then replaces state once; invalid groups leave the whole project unchanged. Source paths, event metadata, media-pool membership and file bytes are retained.
- Checks: `cargo test --release --offline --workspace --features opentrack/local-asio --target x86_64-pc-windows-msvc` passed (95 app,45 audio,16 core;156 total). Added egui-input tests for marquee and group drag; editor tests verify Loop/Skip and enabled/dim overlays without project mutation; core tests verify mixed mono/stereo batches and atomic rejection; Windows cursor test verifies scissors keeps Crosshair. `cargo fmt --all`, x64 local-ASIO build and `git diff --check` passed. Existing CRLF conversion notices are informational.
- Build: OpenTrack process was absent before build and remains closed. `builds/x64/OpenTrack.exe` SHA-256 `9A4372DF20D891BFC3D5319DE6A62E7BC762474AE46BEA141DFE170BF4ABA1C9`.
- Source identity: `audio_events.rs` `80C5E2D76E95F2675848CFCFD317223DB8F1384213A5A99F93181FBAA5A126B9`; `sample_editor.rs` `5DADA4D30926BF5ACE3C4A983120869B72E8E5F69D6C8A5AC7E16EACB63C9BC5`; `system_tool_icons.rs` `03AB54B3B642969D83D45050F675F630E71AFC5FB30F53CC13E559200E6C1E9C`; `app.rs` `6102D51C126E8B432859A39C4ABE02BC005D7BDDD408D27024C6A095C8D7ED58`; core `project.rs` `A69082045121C8511721FDD82F6DB4624079407807EBD622A8D0AFD7AE13DDC0`.
- Runtime limits: no OpenTrack GUI session or current screenshot was captured; no user file or EXAMPLE project was opened or changed. Live edge scrolling, actual pointer/glyph separation, marquee modifiers/hit priority, vertical group rejection, one-step Undo/Redo, save/reopen, and visual loop/skip parity remain for user verification. Compilation and headless event tests do not establish visual parity or acceptance.
- Publication: current T020-T026 work remains uncommitted; no commit or push performed. Stop at the user review checkpoint.

## User review steps

Open the rebuilt executable and a disposable audio project. In scissors mode, hover between grid lines with Snap on/off and confirm the OS crosshair tracks the mouse while scissors/guide show the snapped cut point and the cut matches. Enable Loop then Skip in Project and verify the Sample Editor waveform/overview/ruler show the same bounds/colors, dim when Cycle is off, and follow Project range changes after pan/zoom and when locators fall partly outside the event. In Object Selection, drag a box in all directions across event lanes and blank area; test Shift-add, Ctrl-toggle, empty click, Escape and event/MIDI/automation hit targets. Select multiple events and move horizontally and vertically with snap; test mixed mono/stereo destinations, invalid rows and beat zero. Confirm ghost originals, unchanged spacing, one Undo/Redo, save/reopen, and unchanged source files. Keep the app closed after review.
