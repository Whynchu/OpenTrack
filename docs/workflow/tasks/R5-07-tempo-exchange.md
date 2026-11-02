# R5-07 — Tempo track import and export

- Status: accepted
- Scope version and requirement IDs: revision 5, step 7
- Desired observable outcome: File > Import/Export > Tempo Track exchange the project's tempo and signature through one documented, versioned file, applying an import as a single labelled history step.
- Reference evidence (IDs/paths): `crates/opentrack-app/src/app/tempo_file.rs` (new); the tempo editor contract from R5-04.
- Dependencies and decisions needed: R5-04, R5-05. This is OpenTrack's own format; no Cubase tempo-track compatibility is claimed (interchange decision 2026-11-02).
- Code scope and architecture boundaries: new `src/app/tempo_file.rs`; two menu rows become enabled; the editor's apply path is reused, so import cannot diverge from manual edits.
- Acceptance checks: JSON round trip; foreign format, unknown version and invalid values refused; a real file export then import changes the project as one "Tempo and Signature" step; a malformed file changes nothing.
- Affected regressions: the File > Import/Export Tempo Track rows move from unavailable to state-dependent in the generated inventory.
- Essential gates: persistence through the file, refusal semantics, undo.
- Review owner and milestone stop: self-review; file-dialog wording is reviewed on the desktop later.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: `TempoFile` with `format: "opentrack-tempo"`, `version: 1`, tempo, beats per bar and beat unit; `to_text`/`from_text` validate through the same setters the editor uses; `export_tempo_to`/`import_tempo_from` are split from the dialogs so tests need no picker; import routes through `apply_tempo_editor`, so the history label and no-op behaviour match manual edits.
- Commands/checks and actual results: `cargo test -p opentrack tempo_file` → 2 passed; inventory regenerated (275 leaves, 40 available, 148 state-dependent, 79 unavailable, 8 informational).
- Runtime environment and evidence paths: Kubuntu Linux; no desktop review of the file dialogs yet.
- Ledger IDs updated: README disabled-row count 79; inventory regenerated.
- Review findings, rework: none.
- Failures/unknowns/pending acceptance: the file dialogs themselves are untested on the desktop (default names, filters).
- Next ready task: R5-09.
