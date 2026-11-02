# R4-10 — Open Drum Editor

- Status: review_pending (source/headless; desktop visual/interaction not reviewed)
- Scope version and requirement IDs: roadmap revision 4, step 10
- Desired observable outcome: MIDI > Open Drum Editor opens a focused editor over the selected MIDI part. Rows follow the project drum map and append any unmapped sounding pitches; notes keep real pitch/duration/channel. Add, select (Ctrl multi), move in time, velocity and delete are atomic labelled history steps; Escape cancels an in-progress move; selection is shared with the Key/List editors.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified by the roadmap: mapped pitch rows, existing part/note model, add/select/move/velocity/delete, preserve duration/channel, show unmapped notes, focus guards, grid/snap, multi-selection, Escape, empty parts, shared selection.
  - Observed: `selected_midi_part_id` and `selected_midi_note_ids` are the shared selection; `midi_editor_settings` supplies snap/grid/velocity; project history is the command wrapper.
  - Unknown: live pointer feel and visual density.
- Dependencies and decisions needed: step 8/9 map model (done). No new schema.
- Code scope and architecture boundaries: new `app::drum_editor` module; app window/gesture state; MIDI menu row enabled. No changes to the Key/List editors or core note model.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Row test: mapped order first, unmapped sounding pitch appended without duplicates.
  2. Edit test: snapped add is one "Add Drum Note" step; move, velocity and delete each change only selected notes and add labelled steps; duration/channel of other notes are untouched.
  3. Closing clears the transient drag and adds no history.
  4. Generated inventory drops Open Drum Editor; full app suite/formatting pass.
- Affected regressions: MIDI part selection, note IDs, project history.
- Essential gates: no project change on cancel/close; add/move clamps inside the part; shared selection IDs; no pitch rewriting.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `87fec8c` plus this slice; commit recorded in Git history.
- Changes and reasons: added a drum-grid window with map-ordered rows, unmapped-note fallback, snapped add, Ctrl multi-select, drag-in-time commit on release, Escape cancellation, Delete/Delete button, velocity field and Close. MIDI > Open Drum Editor is enabled only with a valid selected MIDI part.
- Commands/checks and actual results: focused tests 3 passed; full app suite 521 passed, 9 ignored, 0 failed; inventory now reports 271 leaves with 88 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless fixtures. No desktop/pointer review.
- Review findings, rework, elapsed effort if available: module had to live under `app/` because it uses app composition. Self-review only.
- Failures/unknowns/pending acceptance: visual layout, live drag feel, Escape during a real pointer drag, keyboard focus and cross-editor selection refresh are pending. The editor draws fit-to-width rather than a horizontal navigation control.
- Next ready task: revision-4 step 11, bounded Logical Presets.
