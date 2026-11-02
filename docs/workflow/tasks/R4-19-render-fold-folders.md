# R4-19 — Render and fold folders

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 19; user decision 2026-11-02 (folders are organizational; collapse does not affect playback or recording).
- Desired observable outcome: Project rows indent for folder nesting, folder headers get a fold control, Expand All / Collapse All include folders, and collapsed folder children are hidden without being removed from the project or the audio plan. Hidden children keep playing and recording. Folder collapse is persisted with the project and restored on reopen. Existing group folding, row hit testing/scroll geometry and automation-lane heights keep working.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: collapse is view-only for playback/recording; folders are organizational.
  - Observed: group rows already collapse by setting child row heights to zero, while the audio plan and recording selection keep the tracks. Project indentation was group-depth only. Automation lanes are added to the row height of visible rows.
  - Unknown: desktop visual row/control layout, scroll feel and user review.
- Dependencies and decisions needed: R4-17/R4-18 delivered.
- Code scope and architecture boundaries:
  - `Track::folder_collapsed` persisted with a serde default; `set_all_groups_collapsed` also folds folders.
  - Project row geometry adds folder depth to the existing group indent; child rows of a collapsed folder get zero height (and zero audio-event lane height) exactly like group children. Folder headers get a `▤F` collapse control; group control remains `▤G`.
  - No audio-plan, recording, selection or automation-data changes.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. A folder with a child indents the child; collapsing the folder zeroes the child row and lane height, and expanding restores it.
  2. The project still contains the hidden child; playback/recording plans are unchanged.
  3. Expand/Collapse All covers groups and folders; group behavior is unchanged.
  4. `folder_collapsed` round-trips through serialization and defaults false for older projects.
  5. Full app/core suites, inventory and formatting pass.
- Affected regressions: Project row scroll/hit geometry, group collapse, automation lane heights, serialization.
- Essential gates: hidden children remain in the project and audio plan; collapse is persisted; no playback/recording change.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `9f2a4df`; commit recorded in Git history.
- Changes and reasons: added persisted folder collapse, folder indentation, zero-height hidden child rows/lanes, a folder fold control, and folder participation in Expand/Collapse All. Group rendering and plan construction are otherwise untouched.
- Commands/checks and actual results: core folder tests 4 passed; app folder collapse test 1 passed; core suite 159 passed; full app suite 532 passed, 9 ignored; inventory 271 leaves, 84 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; no desktop visual or scroll review was performed.
- Failures/unknowns/pending acceptance: live indentation/fold visuals, scroll/hit testing, automation lane visuals and restoration after reopen remain for desktop review.
- Next ready task: revision-4 step 20, review hierarchy across views.
