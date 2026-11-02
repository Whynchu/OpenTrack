# R5-13 — Per-event offline processing chain

- Status: accepted
- Scope version and requirement IDs: revision 5, step 13
- Desired observable outcome: an event can carry an ordered, validated chain of first-party offline processors, persisted with the project, without changing playback until it is applied.
- Reference evidence (IDs/paths): `crates/opentrack-core/src/project.rs` (`OfflineProcess`, `MAX_OFFLINE_CHAIN`, `AudioClip::offline_chain`, `set_offline_chain`, `offline_chain_of`); `project_file.rs` `FORMAT_VERSION`.
- Dependencies and decisions needed: none. Third-party plug-ins are deliberately excluded from the model.
- Code scope and architecture boundaries: core schema plus a migration test; no DSP and no UI in this step.
- Acceptance checks: core test for defaults, valid chain, every refused parameter, over-long chain, locked event, unknown event, round trip, legacy decode and invalid stored chain; migration test for a v37 file.
- Affected regressions: `AudioClip` gained a defaulted field; the only literal constructions (core clip creation and one app test helper) were updated.
- Essential gates: persistence, validation on load, no behaviour change until applied.
- Review owner and milestone stop: self-review.

## Contract

- Chain length is bounded to eight processors; values must be inside the same ranges the direct commands use (gain −60…+12 dB, normalize −60…0 dBFS, resample 8–192 kHz, stretch 25–400 %).
- A locked event refuses any chain edit, so a lock keeps meaning "this event will not change".
- Processors are named by kind and parameter; there is no executable or third-party element, so a chain cannot depend on an installed plug-in.
- OPR moved from 37 to 38; an older file loads with empty chains, and a file carrying an unusable chain is rejected by `validate`.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: added the enum, bounds, clip field, `set_offline_chain`/`offline_chain_of`, validation and the migration test.
- Commands/checks and actual results: `cargo test -p opentrack-core an_event_offline_chain` → passed; `cargo test -p opentrack-core` → 166 passed; `cargo test -p opentrack project_file` → 34 passed.
- Runtime environment: Kubuntu Linux; no rendering yet.
- Ledger IDs updated: none yet (OPR bump recorded here).
- Review findings: none.
- Failures/unknowns/pending acceptance: rendering (R5-14) and UI (R5-15) are the follow-on steps.
- Next ready task: R5-14.
