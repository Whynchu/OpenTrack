# R4-09 — Drum Map Setup

- Status: review_pending (source/headless; no desktop interaction)
- Scope version and requirement IDs: roadmap revision 4, step 9
- Desired observable outcome: MIDI > Drum Map Setup opens an editor for the project drum map. Rows can be added, renamed, reordered and removed; Apply validates and commits one history entry; Cancel changes nothing. The dialog explains that playback pitch is unchanged and that an empty map shows note numbers.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified by the roadmap: create/rename/reorder/remove through validated commands; Apply one history entry; Cancel no change; explain identity playback; duplicates/default/switching/undo/save-reopen.
  - Observed: step-8 `DrumMap` model; existing dialog/history patterns.
  - Unknown: desktop visual/interaction review.
- Dependencies and decisions needed: step 8 model (done). No new persisted decision.
- Code scope and architecture boundaries: new `app::drum_map_setup` dialog module; app fields/call wiring; MIDI menu row enabled. The model remains in `opentrack-core`.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Apply valid rows -> one "Drum Map Setup" history step; undo clears and redo restores.
  2. Duplicate pitch -> draft error, project and history unchanged, dialog stays open.
  3. Re-applying the same map -> no step; reset to empty -> content step.
  4. Changing the selected track while the dialog is open does not alter the draft.
  5. Generated inventory drops the unavailable row; full app suite/formatting pass.
- Affected regressions: MIDI menu, project history, OPR validation.
- Essential gates: no partial map on validation failure; cancel/closing no dirty state; identity presentation explained; no pitch change.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `287c0c5` plus this slice; commit recorded in Git history.
- Changes and reasons: added a draft-based dialog with pitch/name rows, Up/Down/Remove/Add Row and Reset to Note Numbers; Apply goes through `DrumMap::set_rows`, one labelled history step; Cancel/outside close discards the draft.
- Commands/checks and actual results: focused tests 4 passed; full app suite 518 passed, 9 ignored, 0 failed; inventory now reports 271 leaves with 89 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless app tests. No desktop review.
- Review findings, rework, elapsed effort if available: first placement at crate root could not see app composition; moved under `app/`. A borrow of the draft length during row rendering was fixed by capturing it first. Self-review only.
- Failures/unknowns/pending acceptance: visual layout, live keyboard focus and app save/reopen of a populated map remain pending. Core serde round trip covers persistence.
- Next ready task: revision-4 step 10, Open Drum Editor.
