# R4-11 — Bounded Logical Presets

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 11
- Desired observable outcome: MIDI > Logical Presets offers exactly three named operations: select notes by pitch range, select notes by velocity range, and set the selected notes' MIDI channel. Selection presets change only the shared transient selection. Set Channel is one atomic labelled history step and leaves other notes and CC events untouched.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified by the roadmap: small explicitly named set; show matched scope; no executable scripting; atomic content transforms; selection-only no dirty state; channel limits; undo; non-target CC preservation.
  - Observed: `selected_midi_note_ids` is shared with the Key/List/Drum editors; MIDI note/CC model stores channel per event.
  - Unknown: live dialog interaction.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: new `app::logical_presets` dialog/handlers; MIDI menu row enabled. No new core model, schema or scripting dependency.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Pitch/velocity selectors set the matched IDs, report the count, leave project/history unchanged; a no-match leaves the old selection.
  2. Set Channel changes only selected notes, preserves other notes and controller events, is one "Logical Preset" step, and Undo restores channels.
  3. No selection or invalid channel is refused without history.
  4. Generated inventory drops the row; full app suite/formatting pass.
- Affected regressions: MIDI note selection, note channels, CC data, history.
- Essential gates: selection-only dirty state unchanged; content transform atomic; CC untouched.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `5583afa` plus this slice; commit recorded in Git history.
- Changes and reasons: added the three-preset dialog and handlers; selection presets operate on all notes in the selected part and only assign `selected_midi_note_ids`; Set Channel clones the project, validates 0-15, updates only selected notes, and commits one labelled step.
- Commands/checks and actual results: focused tests 3 passed; full app suite 524 passed, 9 ignored, 0 failed; inventory now reports 271 leaves with 87 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless fixtures. No desktop review.
- Review findings, rework, elapsed effort if available: the controller-event test helper signature was corrected. Self-review only.
- Failures/unknowns/pending acceptance: live dialog visuals and interaction pending. Locks do not apply to MIDI notes in the current model.
- Next ready task: revision-4 step 12, CC Automation Setup.
