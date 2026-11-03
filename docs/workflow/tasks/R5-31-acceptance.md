# R5-31 — Revision acceptance

- Status: accepted for source and automated checks; desktop, listening and hardware review pending
- Scope version and requirement IDs: revision 5, step 31
- Desired observable outcome: the revision's suites, formatting, release build and migration behaviour are run and recorded, with source and runtime evidence kept separate.
- Reference evidence (IDs/paths): this packet, `docs/workflow/current.md`, `builds/x64/build-info.json`.
- Dependencies and decisions needed: R5-01 to R5-30 (R5-02 is the user's runtime review and remains open).
- Code scope and architecture boundaries: no product change.
- Acceptance checks: full suites, `cargo fmt --all --check`, `git diff --check`, release build through `./build.sh`, OPR migration tests across versions, and a saved-then-reopened project carrying the new fields.
- Affected regressions: none.
- Essential gates: separate source vs automated vs runtime claims.

## Results (2026-11-02, `e0712f3`)

| Check | Result |
| --- | --- |
| app suite | 579 passed, 4 failed, 9 ignored (592 tests) |
| core suite | 166 passed |
| audio suite | 171 passed, 3 ignored |
| `cargo fmt --all --check` | clean |
| `git diff --check` | clean |
| release build | `builds/x64/OpenTrack`, SHA-256 `4c0034ac2992150c65483d39cb61b2a5b1e6ebd8727bbabd10b2bdd444b13a40` |
| OPR | 38; migrations tested for v29→30 (timecode), v30→31 (pool folders), v31→32 (regions), v32→33 (pool record folder) and v37→38 (offline chains) |
| save/reopen | new fields validated by `Project::validate` and covered by project-file round trips |

The four app failures are `transport_locate_tests::{pre_roll_uses_both_selection_edges_and_writes_no_history, post_roll_uses_both_edges_with_independent_lengths_and_no_history, post_roll_interruption_clears_the_target, roll_scheduling_refuses_unsupported_state_and_cancels_on_seek}`. They fail identically on the pristine revision-4 checkpoint in this container because `schedule_roll` needs a real audio output device; this is a recorded environment baseline, not a regression, and no assertion was weakened.

## Not claimed

No desktop run, no listening test, no VST3 compatibility run, no Windows/ASIO build and no user acceptance. R5-02 (the runtime checklist for the manual-review fixes), the audible chain/keyboard/arranger results and the Windows deliverable stay pending on the user.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: `e0712f3` plus this packet's commit; build hash above.
- Changes and reasons: none; the pass records results and separates claims.
- Failures/unknowns/pending acceptance: the four device-dependent tests, and every runtime item listed above.
- Next ready task: R5-32.
