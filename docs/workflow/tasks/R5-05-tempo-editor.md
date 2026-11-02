# R5-05 — Tempo Track Editor

- Status: accepted
- Scope version and requirement IDs: revision 5, step 5
- Desired observable outcome: Project > Tempo Track opens a focused editor for the project tempo and time signature, applying as one labelled undo step and refusing values the contract rejects.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown): `crates/opentrack-core/src/project.rs` (`set_tempo`, `set_time_signature`); `crates/opentrack-app/src/app/tempo_editor.rs` (new); the existing metronome accent preference in `app.rs`.
- Dependencies and decisions needed: R5-04 (accepted). Accent scope decided in R5-04: application preference, not project content.
- Code scope and architecture boundaries: new `src/app/tempo_editor.rs`; `app.rs` only declares the module, holds the draft, and routes the menu row. No change to `opentrack-audio`.
- Acceptance checks (action, expected result, fixtures, evidence to retain): unit test for seeding, apply, undo label, no-op and refusals; a render test for the window's controls and the accent note.
- Affected regressions: tempo/signature semantics unchanged for untouched projects; the plan rebuild keeps playback in step.
- Essential gates: persistence (values are project fields), undo (one step), no-op writes nothing, recording guard.
- Review owner and milestone stop: self-review; desktop review of the dialog stays with the user.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: see the commit carrying this packet.
- Changes and reasons: `tempo_editor::Draft` + `open_tempo_editor`, `apply_tempo_editor`, `show_tempo_editor`; the Project > Tempo Track row is enabled while a project is open; recording disables the controls and Apply; the window states that accents are application settings and shows the current accent mask read-only.
- Commands/checks and actual results: `cargo test -p opentrack tempo_editor` → 2 passed; `cargo test -p opentrack` → 551 passed, 4 known device-dependent roll failures, 9 ignored.
- Runtime environment and evidence paths: Kubuntu Linux; no desktop review yet.
- Ledger IDs updated; report command/result: README interface counts (81 disabled rows, 148/152), accessibility report regenerated, inventory regenerated.
- Review findings, rework, elapsed effort if available: none.
- Failures/unknowns/pending acceptance: desktop review of the dialog (layout, focus, Enter-to-apply) is pending.
- Next ready task: R5-06 (beat/sample conversion audit).
