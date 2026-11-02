# R5-14 — Staged chain render

- Status: accepted
- Scope version and requirement IDs: revision 5, step 14
- Desired observable outcome: applying an event's chain renders each processor to a staged file, publishes only a complete result, and removes every staged file when a step fails.
- Reference evidence (IDs/paths): `crates/opentrack-app/src/app/offline_processing.rs` (new); `crates/opentrack-audio/src/audio_process.rs` helpers.
- Dependencies and decisions needed: R5-13 (accepted).
- Code scope and architecture boundaries: new focused module; reuses the existing audio helpers so no DSP is duplicated; `app.rs` only declares the module.
- Acceptance checks: a two-processor chain bakes in order with the event repointed and one history step; a mono event with Stereo Flip fails with a reason, leaves the project identical and removes its staged file; an empty chain and each processor are refused/applied as documented.
- Affected regressions: none; the chain is cleared once baked, so a second apply is impossible without re-adding processors.
- Essential gates: no partial publish, staged cleanup, undo, chain consumed.
- Review owner and milestone stop: self-review.

## Contract

- Each processor reads the previous stage's whole file (after the first, which reads the event's used range) and writes a new staged file under the pool folder.
- Time Stretch scales the final event length by its percentage; Resample does not; a chain multiplies the stretch factors.
- A failure at any step deletes every staged file and leaves the project untouched.
- The event's chain is cleared, pitch analysis and source-fade lineage are dropped, and fades are clamped after a successful bake.
- Stereo Flip on a mono event fails the chain with the audio layer's message rather than silently skipping the processor.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: `Staging` tracks created files; `run_processor` maps each processor to its audio helper; `apply_offline_chain` reuses the source-process pattern (undo, lineage reset, fades, plan rebuild).
- Commands/checks and actual results: `cargo test -p opentrack offline_processing` → 2 passed, run three times to confirm stability after separating the tests' temp roots; app suite 561 passed / 4 known device failures / 9 ignored.
- Runtime environment: Kubuntu Linux; offline only.
- Ledger IDs updated: inventory regenerated (no menu change).
- Review findings: the first version had both tests sharing one temp root, which made the imports-count assertion flaky under parallel execution; each test now gets its own root.
- Failures/unknowns/pending acceptance: UI (R5-15) and the acceptance pass (R5-17) follow.
- Next ready task: R5-15.
