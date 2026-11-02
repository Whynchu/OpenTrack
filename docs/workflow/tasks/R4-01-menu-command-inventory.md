# R4-01 — Reconcile the menu and command inventory

- Status: review_pending
- Scope version and requirement IDs: roadmap revision 4, step 1 (`docs/workflow/roadmap-32-steps-v4.md`)
- Desired observable outcome: the repository has one reproducible inventory of the project-open main menu, the empty-project shell and the bundled shortcut keymap; stale accessibility claims are corrected; disabled rows are counted from source rather than inherited reports.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed, current source: `crates/opentrack-app/src/app.rs` `show_menu` / `show_empty_project_menu` at planning baseline `12a01e0`.
  - Inherited reports (not rerun): 71 disabled entries and 147/152 dispatched IDs in `README.md`, `docs/workflow/current.md`, revision-3 checkpoints.
  - Stale report: `docs/workflow/accessibility-2026-11-02.md` claimed 43 undispatched shortcuts and that menus reach every missing command.
  - Historical registers: `docs/reviews/audit-menus.md` and `docs/reviews/audit-keyboard-commands.md` are snapshots of `7b898c6`.
  - Unknown: runtime rendering/click behavior; context-menu coverage beyond the curated register.
- Dependencies and decisions needed: none. This step adds tooling and documentation only; it does not enable or implement a command.
- Code scope and architecture boundaries: new test-only module `crates/opentrack-app/src/menu_inventory.rs`; `main.rs` test-module declaration; generated report in `docs/reviews/generated-source-inventory.md`. No production menu, command or persistence code changes.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. `cargo test -p opentrack generated_source_inventory -- --nocapture` regenerates the report and passes the exact-match check.
  2. `cargo test -p opentrack generated_inventory` asserts deterministic counts, no duplicate full paths, all eleven project-open top-level menus present, empty-shell rows present for MIDI/Scores/Media/Transport/Window, and the exact five undispatched bound IDs each have a literal unavailable menu row.
  3. `git diff --check` passes.
- Affected regressions: existing keymap coverage test remains; no product behavior changes.
- Essential gates (audible result, persistence, undo/dirty state, recovery as applicable): not applicable to this documentation/tooling slice. Runtime menu rendering remains unverified.
- Review owner and milestone stop: user review at the revision-4 milestone; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (provider `opencode-go`, model `deepseek-v4.1-flash`). No independent reviewer.
- Source identity (commit plus dirty diff/artifact identity) and build hash: planning baseline `12a01e0`; slice commit recorded in Git history after this packet. No product build was made; this is a test/documentation slice.
- Changes and reasons:
  - Added a source-text menu parser that walks `ui.menu_button` nesting, literal `menu_command`/`checked_menu_command`/`disabled_menu_commands` rows, `add_enabled`/`selectable_label`/`button` rows and the simple `for` array families. It records full path, shortcut, status, availability expression and source line.
  - Added the shortcut table from the bundled keymap and dispatch-arm check, replacing the stale numbers in the accessibility report.
  - Generated `docs/reviews/generated-source-inventory.md` so the counts are reproducible and drift fails a test.
  - Corrected the accessibility report and marked the two older source registers as historical snapshots.
- Commands/checks and actual results:
  - `UPDATE_SOURCE_INVENTORY=1 cargo test --offline -p opentrack generated_source_inventory -- --nocapture`: passed; 268 project-open leaves, 18 empty-shell leaves, 152 bound command IDs.
  - `cargo test --offline -p opentrack generated_inventory -- --nocapture`: passed (2 tests) after the count/invariant fixes.
  - `git diff --check`: passed.
- Runtime environment and evidence paths: Windows x64 host, no app launched. Generated evidence: `docs/reviews/generated-source-inventory.md` (app.rs FNV-1a `0x41eec79b394c843c`). Runtime menu rendering and click behavior are not claimed.
- Ledger IDs updated; report command/result: accessibility report and README counts updated; generated report is the source register.
- Review findings, rework, elapsed effort if available: self-review found and fixed a nested-block parser bug (0 leaves), a trailing-array-comma artefact and an informational-caption path collision before the report was accepted. Measured project-open main-menu state: 37 available, 131 state-dependent, 95 unavailable, 5 informational. Empty shell: 6 available, 12 unavailable. Shortcut coverage: 147 of 152 bound IDs have a dispatch arm; the five are `devices.video`, `edit.open_score_editor`, `project.open_tempo_track_editor`, `transport.retrospective_record`, `transport.use_external_sync`.
- Failures/unknowns/pending acceptance: the parser is source-based, not runtime proof; context menus remain in the curated register; `edit.open_score_editor` has an enabled Ctrl+R route through Scores > Open Selection that opens the in-place editor, not a score editor. Runtime review of the menu shell remains pending.
- Next ready task: revision-4 step 2, review the recent media pipeline (slices 41-56) before extending it.
