# R5-17 — Offline processing acceptance

- Status: accepted for source and headless checks; audible and desktop review pending
- Scope version and requirement IDs: revision 5, step 17
- Desired observable outcome: multi-event, locked, failing and reopened cases behave as specified, with no partial publish and correct history.
- Reference evidence (IDs/paths): `acceptance_multi_event_locked_and_reopen_behaviour` in `src/app/offline_processing.rs`; the earlier chain tests.
- Dependencies and decisions needed: R5-13 to R5-16 (accepted).
- Code scope and architecture boundaries: one acceptance test; no product change.
- Acceptance checks: a locked event refuses with a reason; unlocking bakes it; two events keep distinct results that exist on disk; a failing stage leaves the project identical and removes staged files; a stored chain survives serialization and validates.
- Affected regressions: none.
- Essential gates: no partial publish, per-event isolation, persistence.
- Review owner and milestone stop: self-review.

## Known limitation

The chain render runs synchronously on the UI thread; unlike the stem and mixdown exports it has no worker, progress bar or cancel button. A long event with eight stages can therefore hold the frame for the length of the file work. The per-stage work is bounded by the file and the chain is capped at eight, so the risk is a pause rather than unbounded work, but it is a real limitation and is recorded here instead of being described as cancellable. Backgrounding it is a follow-up in the performance step (29) or a later packet.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: added the acceptance test; recorded the synchronous limitation after checking the R5-14 implementation.
- Commands/checks and actual results: `cargo test -p opentrack acceptance_multi_event` → passed; app suite at this checkpoint 564 passing with the four known device failures.
- Runtime environment: Kubuntu Linux; offline only.
- Ledger IDs updated: none.
- Review findings: R5-14's packet text said "cancel on request"; the implementation is synchronous, so the contract is corrected here rather than overstating the code.
- Failures/unknowns/pending acceptance: desktop review of a bake, and the audible comparison of a baked chain against the direct commands.
- Next ready task: R5-18.
