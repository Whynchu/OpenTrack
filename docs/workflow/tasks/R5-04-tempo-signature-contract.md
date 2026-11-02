# R5-04 — Tempo and time-signature contract

- Status: accepted
- Scope version and requirement IDs: revision 5, step 4
- Desired observable outcome: one validated, documented owner for project tempo and time signature, with explicit refusal of unsupported values and a stated boundary for the metronome accent pattern.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown): `crates/opentrack-core/src/project.rs` (`tempo_bpm`, `beats_per_bar`, `beat_unit`, `validate`); `crates/opentrack-app/src/app.rs` (`metronome_accent_mask` stored through eframe application storage); revision-5 packet step 4; user direction that the packet should run without further questions.
- Dependencies and decisions needed: none. Scope decision taken and recorded below.
- Code scope and architecture boundaries: `opentrack-core::project` only; no app or audio change.
- Acceptance checks (action, expected result, fixtures, evidence to retain): unit test covering defaults, valid edits, every refused form, loaded-file rejection and round trip; full core suite.
- Affected regressions: none expected; bounds and messages previously inlined in `validate` are now shared with the setters.
- Essential gates: persistence (round trip), refusal (invalid values leave the project untouched).
- Review owner and milestone stop: self-review.

## Scope decision

- Multiple tempo or signature events stay out of scope. One tempo and one signature describe the project, and a file carrying unsupported values is rejected rather than flattened.
- **The metronome accent pattern stays an application preference**, not project content. It already lives in eframe storage and drives the click; moving it into the project would silently change how an existing project sounds when opened. The tempo editor therefore edits tempo and signature only and points at the existing metronome setup for accents. This is recorded rather than implemented because it is a behavioural choice, not a wiring gap.
- **No OPR bump was needed.** Tempo and signature were already persisted fields with validation; step 4 added the missing single contract (setters + constants) and tests, not a schema change. OPR remains 37.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity (commit plus dirty diff/artifact identity) and build hash: see the commit that carries this packet.
- Changes and reasons: added `MIN_TEMPO_BPM`/`MAX_TEMPO_BPM`/`MIN_BEATS_PER_BAR`/`MAX_BEATS_PER_BAR`/`BEAT_UNITS` and `Project::set_tempo` / `set_time_signature`; `validate` now uses the same constants and the stricter `BEAT_UNITS` list (1, 2, 4, 8, 16, 32) instead of any power of two up to 32, so 32 stays valid and nothing previously accepted in practice is refused.
- Commands/checks and actual results: `cargo test -p opentrack-core tempo_and_time_signature` → passed; `cargo test -p opentrack-core` → 165 passed.
- Runtime environment and evidence paths: Kubuntu Linux, no desktop review in this step.
- Ledger IDs updated; report command/result: none.
- Review findings, rework, elapsed effort if available: the only behavioural tightening is the beat-unit list, which matches the documented values.
- Failures/unknowns/pending acceptance: none.
- Next ready task: R5-05, the Tempo Track Editor on this contract.
