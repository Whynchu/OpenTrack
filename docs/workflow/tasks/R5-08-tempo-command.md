# R5-08 — Wire the tempo editor command

- Status: accepted
- Scope version and requirement IDs: revision 5, step 8
- Desired observable outcome: Ctrl+T (`project.open_tempo_track_editor`) opens the same Tempo Track editor as the menu row, and the shortcut report reflects it.
- Reference evidence (IDs/paths): `crates/opentrack-app/src/app.rs` dispatch match; `menu_inventory.rs` curated guard; `docs/reference-keymap-windows.csv`.
- Dependencies and decisions needed: R5-05 (accepted).
- Code scope and architecture boundaries: one dispatch arm plus the inventory guard that pins the tempo row as reachable; no duplicated logic.
- Acceptance checks: the guard test asserts the tempo row is state-dependent and its command dispatched; the shortcut audit reports one fewer uncovered command; the app suite stays green apart from the documented device failures.
- Affected regressions: the curated "unavailable rows" list no longer includes the tempo row, which is the intended change.
- Essential gates: none beyond the previous step.
- Review owner and milestone stop: self-review.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: see the commit carrying this packet.
- Changes and reasons: added the `project.open_tempo_track_editor` arm; updated the curated inventory guard to expect four uncovered commands and to assert the tempo row is reachable, so a regression fails loudly.
- Commands/checks and actual results: `cargo test -p opentrack keyboard_coverage -- --nocapture` → 148 of 152 (4 unhandled); `cargo test -p opentrack generated_` → both inventory tests pass after regeneration.
- Runtime environment and evidence paths: Kubuntu Linux.
- Ledger IDs updated; report command/result: `docs/reviews/generated-source-inventory.md` regenerated (275 leaves, 40 available, 146 state-dependent, 81 unavailable, 8 informational); accessibility report regenerated (148/152); README counts updated.
- Review findings, rework, elapsed effort if available: the curated guard had to move with the feature, which is the point of the guard.
- Failures/unknowns/pending acceptance: none beyond the pending desktop review.
- Next ready task: R5-06.
