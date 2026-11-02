# R4-12 — Enable CC Automation Setup

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 12; user decisions 2026-11-02 (one selected CC lane, project-persisted view choice, undoable content edits, no history for view/selection state).
- Desired observable outcome: MIDI > CC Automation Setup configures the single visible/editable MIDI controller lane. The user can choose any CC 0–127, sees a readable lane label, and the chosen lane is remembered when the project is saved and reopened. Selecting a lane invents no controller data, hides none of the project's other CC events, and creates no undo step or dirty content. Drag/add/delete edits in the lane keep using the existing validated core CC path, the existing undo history, live playback plan and MIDI export.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: one selected lane at a time; persist the view choice in the project; CC content edits are undoable.
  - Observed: `MidiControllerEvent` already stores controller 0–127 at part-relative beats; `Project::add_midi_controller_event`/validation accept 0–127; the MIDI editor lane already draws/drags/adds/removes the events for one selected controller; playback plan and `midi_file` export/import already carry arbitrary controller numbers. `MidiEditorSettings.controller_number` was hardcoded to 11 and never persisted; only four common controllers appeared in the lane selector.
  - Observed owner: `ArrangeViewState` is the per-project view/workspace record stored in the `.opr` document (`arrange_view_state_v1`). It already holds ruler/zoom/selection view choices and normalizes older values.
  - Unknown: live dialog and lane interaction on the desktop; this slice is headless.
- Dependencies and decisions needed: none. No new dependency. No OPR format bump: the added field is an optional workspace view value with a serde default, like other view fields added after format 34.
- Code scope and architecture boundaries:
  - New `crates/opentrack-app/src/app/cc_automation.rs`: controller naming/options, lane event filter used by the editor, setup dialog and apply/open actions.
  - `ArrangeViewState` (`project_file.rs`) gains `cc_lane_controller: u8` with default 11, normalization to 0–127, and round-trip/legacy tests. This is view state, not project content and not an undoable edit.
  - `app.rs`: enable the MIDI menu row, register the dialog, pass the persisted lane number to the existing MIDI editor, and use the shared label/filter helper. No core model or serialization change; no changes to playback or export behavior.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. MIDI > CC Automation Setup opens a dialog; picking any CC 0–127 and Apply updates the lane header and list label without touching project content or history; Cancel changes nothing.
  2. The lane selector offers all CC 0–127 with readable names for common controllers and a numeric fallback label.
  3. The lane shows only the selected controller's existing events; switching lanes leaves other CC events intact and does not create an event. A non-default controller (e.g. CC 74) round-trips through project save/reopen as view state, and its events round-trip through the project document.
  4. A CC content change produces one labelled undo step and Undo restores the value; a lane switch alone produces no history entry and no dirty project content.
  5. Existing playback-plan conversion and MIDI export consumers remain unchanged and continue to carry controller/channel/value; full app and core suites plus formatting pass; generated menu/keyboard inventory is regenerated.
- Affected regressions: MIDI lane editing, undo/dirty state, project save/reopen, MIDI export, menu inventory.
- Essential gates: view-only change creates no history; content edits remain atomic/undoable; save/reopen restores the lane choice; old projects default 11.
- Review owner and milestone stop: user review. Self-review only in this run; no independent reviewer claimed.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: pending commit; based on `cc9971f`.
- Changes and reasons: new `app::cc_automation` owns controller naming (25 named controllers plus numeric fallback), lane filtering and the setup dialog; MIDI menu row is enabled; the editor lane selector now offers all CC 0–127 and writes the persisted `ArrangeViewState.cc_lane_controller` (default 11, normalized to 0–127); the old hardcoded `MidiEditorSettings.controller_number` was removed. No schema/OPR bump because the field is optional workspace view state; project content, playback plan and MIDI export code are unchanged.
- Commands/checks and actual results: `cargo fmt --all`; `git diff --check` clean. Focused CC tests 3 passed. Project-file view/content round trip 1 passed. Full app suite 528 passed, 0 failed, 9 ignored. Core suite 152 passed, 0 failed. Inventory regenerated: 271 project-open leaves, 38 available, 139 state-dependent, 86 unavailable, 8 informational; 147/152 bound IDs dispatched unchanged.
- Runtime environment and evidence paths: Windows x64 host; headless tests only. No desktop review, listening or MIDI-device check.
- Ledger IDs updated; report command/result: menu inventory regenerated with `UPDATE_SOURCE_INVENTORY=1 cargo test -p opentrack generated_source_inventory`; README/current.md updated.
- Review findings, rework, elapsed effort if available: self-review only; corrected the test helper's controller/channel argument order. The only behavior extension is the lane selector growing from four named controllers to all 128 numbered lanes.
- Failures/unknowns/pending acceptance: live dialog/lane interaction, visual review, save/reopen in the running application, playback audition and MIDI export listening remain pending.
- Next ready task: revision-4 step 13, FX-channel identity.
