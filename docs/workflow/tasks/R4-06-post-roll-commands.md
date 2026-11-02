# R4-06 — Enable Post-roll from Selection Start/End

- Status: review_pending (partial: per-frame stop granularity)
- Scope version and requirement IDs: roadmap revision 4, step 6
- Desired observable outcome: Transport > Post-roll from Selection Start and Post-roll from Selection End start at the named boundary and stop after the configured post-roll interval. Pre and post lengths remain independent. Manual interruption, empty selection and end-of-project handling are defined.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified by the roadmap: both disabled entries become live; same scheduler as pre-roll; check manual interruption, empty selection, end-of-project, independent settings.
  - Observed: post plans and refusals from step 4; `schedule_roll(true, edge)`; Transport menu rows.
- Dependencies and decisions needed: steps 4-5 (done). Sample-accurate stop remains a timing decision; not claimed.
- Code scope and architecture boundaries: menu wiring only; the scheduler and preferences already exist. No schema or dependency change.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Both post-roll rows are built from `menu_command`, not the disabled list.
  2. App tests: post-roll from both edges uses post length only (pre set differently), no history change; a late selection plays the full tail past the last event; stop/seek clears the target; missing selection is refused by the shared scheduler.
  3. Generated inventory drops the two unavailable rows; full app suite, formatting and diff checks pass.
- Affected regressions: pre-roll, play-until, locate, transport stop.
- Essential gates: no project undo step; transport start/stop is observable; end-of-project play is defined as silence through the roll tail, not a clamp.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `d7d5d1b` plus this slice; commit recorded in Git history.
- Changes and reasons: enabled the two post-roll rows; removed their disabled placeholders. No new control was needed because the step-5 `Roll Lengths...` submenu already exposes the independent post length.
- Commands/checks and actual results: focused post-roll tests 3 passed; full app suite 512 passed, 9 ignored, 0 failed; inventory now reports 271 leaves with 91 unavailable.
- Runtime environment and evidence paths: Windows x64 host, headless fixtures. No desktop/audio-device review.
- Review findings, rework, elapsed effort if available: end-of-project is intentionally allowed to run into silence for the full post length. Self-review only.
- Failures/unknowns/pending acceptance: live listening/interaction and block-granularity overshoot remain pending. Step-4 timing limitation applies to both rolls.
- Next ready task: revision-4 step 7, MIDI Reset.
