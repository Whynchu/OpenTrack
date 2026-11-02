# R5-01 — Reconcile inventory and QA baseline

- Status: accepted
- Scope version and requirement IDs: revision 5, step 1
- Desired observable outcome: one reproducible set of counts and known failures that README, current.md, the accessibility report and the generated inventory all agree on, including the manual-review fixes.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown): `docs/reviews/generated-source-inventory.md`; `docs/workflow/accessibility-2026-11-02.md`; `README.md`; the manual-review notes supplied 2026-11-02 (observed); the four roll-test failures reproduced on the pristine revision-4 checkpoint (observed, environment).
- Dependencies and decisions needed: none. The interchange decision (2026-11-02) is recorded in the revision-5 packet and does not affect this step.
- Code scope and architecture boundaries: documentation and measurement only; no product code.
- Acceptance checks (action, expected result, fixtures, evidence to retain): regenerate the inventory with `UPDATE_SOURCE_INVENTORY=1` and confirm no diff; run the shortcut audit; count disabled labels in source; run the app, core and audio suites; record the four baseline failures.
- Affected regressions: none (no code change).
- Essential gates (audible result, persistence, undo/dirty state, recovery as applicable): not applicable.
- Review owner and milestone stop: self-review at the end of the step; runtime review of the review fixes stays with the user in R5-02.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity (commit plus dirty diff/artifact identity) and build hash: `9a5e7dd` plus this documentation commit; release build `c34a3f664a2b565effbaba16be94fd9511ab85681ebf054470d0615604dd8d81` from the review-fix commit.
- Changes and reasons: no product change. Measured and recorded the baseline; confirmed the generated inventory is deterministic; confirmed README, accessibility and inventory agree; recorded the review-fix commit and the environment failures.
- Commands/checks and actual results:
  - `UPDATE_SOURCE_INVENTORY=1 cargo test -p opentrack generated_source_inventory` → passed, and `git status docs/reviews` stayed clean (regeneration is a no-op).
  - `cargo test -p opentrack keyboard_coverage -- --nocapture` → `147 of 152 distinct bound commands have a dispatch arm (198 chords); 5 unhandled`.
  - Disabled-label count in source: 58 unique `(label, shortcut)` pairs across `disabled_menu_commands` blocks.
  - Generated inventory: 275 project-open leaves — 40 available, 145 state-dependent, 82 unavailable, 8 informational; 18 shell leaves; two "No duplicate full paths" sections present.
  - `cargo test -p opentrack` → 549 passed, 4 failed, 9 ignored. `cargo test -p opentrack-core` → 164 passed. `cargo test -p opentrack-audio` → 170 passed, 3 ignored.
  - The four failures are `transport_locate_tests::{pre_roll_uses_both_selection_edges_and_writes_no_history, post_roll_uses_both_edges_with_independent_lengths_and_no_history, post_roll_interruption_clears_the_target, roll_scheduling_refuses_unsupported_state_and_cancels_on_seek}`; all four fail identically on the pristine revision-4 checkpoint with these changes stashed, because no audio output device is available to `schedule_roll`.
  - Format version: OPR 37 (`FORMAT_VERSION` in `crates/opentrack-app/src/project_file.rs`).
- Runtime environment and evidence paths: Kubuntu Linux, no audio output device available to the test harness; no desktop or listening review performed in this step.
- Ledger IDs updated; report command/result: `docs/workflow/current.md` baseline paragraph updated; no ledger IDs in this step.
- Review findings, rework, elapsed effort if available: README's 82 unavailable rows, the accessibility report's 147/152 and the inventory agree, so no numbering correction was needed; the only reconciliation was recording the review fixes (`9a5e7dd`) and the environment-failure baseline, which current.md previously did not state.
- Failures/unknowns/pending acceptance: the four roll tests stay red on this host until a device is present; runtime review of the review fixes is pending with the user (R5-02).
- Next ready task: R5-02 (user runtime review checklist), with R5-03 (export/delivery contract review) able to run in parallel because it does not depend on the desktop pass.
