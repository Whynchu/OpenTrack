# R4-05 — Enable Pre-roll to Selection Start/End

- Status: review_pending (partial: per-frame stop granularity)
- Scope version and requirement IDs: roadmap revision 4, step 5
- Desired observable outcome: Transport > Pre-roll to Selection Start and Pre-roll to Selection End run the step-4 scheduler, with visible labelled pre/post length controls and a reason when the selection cannot resolve.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified by the roadmap: both disabled entries become live; use the existing audio/MIDI selection resolver; labelled roll-length controls; missing/zero-length/near-zero reasons; no project undo step from transport changes.
  - Observed: `schedule_roll(false, edge)` from step 4; `transport_commands::selected_event_span` covers selected audio events plus the selected MIDI part; menu rows at `app.rs` Transport.
- Dependencies and decisions needed: step 4 scheduler (done). Sample-accurate stop remains a timing decision; this slice does not claim it.
- Code scope and architecture boundaries: menu wiring and a `Roll Lengths...` submenu with two DragValues; no new module or dependency. Length edits are app preferences, not project content.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Both pre-roll menu rows are built from `menu_command` with a project/non-recording gate, not from `disabled_menu_commands`.
  2. App tests: pre-roll from both edges sets the expected start/stop; near-zero target clamps to 0; target at 0 refuses; no history count changes.
  3. Generated inventory shows the two rows no longer unavailable; full app suite, formatting and diff checks pass.
- Affected regressions: Transport play-from/until, locate selection, project dirty state.
- Essential gates: no project undo step; missing/zero selection gives status text, not a silent no-op; selection scope documented as audio events + selected MIDI part.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `a62f0d0` plus this slice; commit recorded in Git history.
- Changes and reasons: enabled the two pre-roll rows; added `Transport > Roll Lengths...` with labelled pre/post fields (0-64 beats) and a scope caption. The existing scheduler supplies all refusal messages. No keybinding was added because the keymap has no pre-roll command.
- Commands/checks and actual results: focused pre-roll tests 3 passed; full app suite 510 passed, 9 ignored, 0 failed; generated inventory now reports 271 project-open leaves (37 available, 133 state-dependent, 93 unavailable, 8 informational).
- Runtime environment and evidence paths: Windows x64 host, headless fixtures. No desktop/audio-device review.
- Review findings, rework, elapsed effort if available: none beyond the step-4 limitation. Self-review only.
- Failures/unknowns/pending acceptance: live menu interaction, focus behaviour, audible start/stop placement and the block-granularity overshoot remain pending. Post-roll rows stay unavailable until step 6.
- Next ready task: revision-4 step 6, enable Post-roll from Selection Start/End.
