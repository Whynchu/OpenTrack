# R5-29 — Performance pass: background the offline chain render

- Status: accepted
- Scope version and requirement IDs: revision 5, step 29
- Desired observable outcome: the known frame-blocking hotspot recorded in R5-17 (a synchronous chain render) is removed, and the change is measured rather than assumed.
- Reference evidence (IDs/paths): `apply_offline_chain`, `poll_offline_chain`, `render_chain`, `ChainRenderTask` in `crates/opentrack-app/src/app/offline_processing.rs`; R5-17's limitation note.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: the chain renderer moves to a worker; publishing stays on the UI thread.
- Acceptance checks: the render runs on a worker with stage progress, cancelling publishes nothing, and every existing chain test still passes by waiting for the published result.
- Affected regressions: the five existing offline tests now poll the worker through a helper; no behaviour change to the result.
- Essential gates: cancel, no partial publish, one history step, undo.
- Review owner and milestone stop: self-review; the desktop should confirm the frame stays responsive on a long event.

## Measurement

- Before: `apply_offline_chain` ran every stage inline, so an eight-stage chain on a multi-minute file blocked the UI thread for the length of the file work (recorded in R5-17 from code inspection, not measured on a device).
- After: the work runs in `opentrack-offline-chain`; the UI shows `Rendering offline chain: stage N of M` and the frame is free. Stage counts are reported in the status, and cancelling drops the task, which sets its cancel flag.
- The change is structural and verifiable in code and tests; no frame-time number is claimed because this host cannot run the desktop under load.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: split the render into `render_chain` (worker) and `publish_offline_chain` (UI), added `ChainRenderTask` with progress/cancel and a `Drop` that cancels, added `poll_offline_chain` and `cancel_offline_chain`, and made the tests wait for the worker.
- Commands/checks and actual results: `cargo test -p opentrack offline_processing` → 6 passed (cancel test new); app suite 577 passed / 4 known device failures / 9 ignored; core 166; audio 170.
- Runtime environment: Kubuntu Linux.
- Ledger IDs updated: inventoried.
- Review findings: the status text previously lacked the processor count after the change; it now reports `Applied N offline processor(s) into <file>`, which the existing assertions rely on.
- Failures/unknowns/pending acceptance: frame-time on a long event needs the desktop pass.
- Next ready task: R5-25 (secondary ruler review) or R5-30 (accessibility).
