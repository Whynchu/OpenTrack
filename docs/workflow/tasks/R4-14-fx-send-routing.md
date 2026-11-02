# R4-14 — Route sends to FX channels

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 14; user decision 2026-11-02 (FX channels are shared returns accepting sends from multiple tracks; reject feedback cycles; hold changes to existing behavior).
- Desired observable outcome: any channel with a send can select an FX channel as its destination. Multiple tracks may send to the same FX return. The return's inserts, EQ, fader, pan and mute/solo contribute before it reaches its own output. Dangling, self and cyclic FX targets are refused by the same validation and graph preparation already used for group routing; existing group route spellings and behavior are unchanged. Add FX Channel menus remain step 15.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: FX returns accept sends from multiple tracks; feedback cycles are rejected; already implemented group/FX behavior must not silently change.
  - Observed: group sends already pass through `PlaybackOutputRoute::Group(index)` in both live and offline plans, with pre/post tap, gain, pan, bypass, latency compensation and cycle handling in `opentrack-audio`. `opentrack-core::routing` parses persisted route strings once. `opentrack-app::routing::playback_sends` resolves them to plan indices. Channel Settings owns the send destination menu and cycle filter.
  - Unknown: desktop send-menu interaction and audible FX/insert review; step 16 owns the end-to-end review and listening procedure.
- Dependencies and decisions needed: R4-13 FX identity (delivered). No dependency addition.
- Code scope and architecture boundaries:
  - `opentrack-core::routing`: add `OutputRoute::Fx`, persisted spelling `FX:<id>`, `fx()`/`bus()` helpers, `send_destinations`, and explicit FX routing errors. Main-output routes keep using group-only destinations.
  - `opentrack-core::project::validate_routing`: validate FX send targets against FX channels, reject main outputs to FX, self targets, missing FX and FX/group feedback loops; preserve existing group error variants.
  - `opentrack-app::routing`: resolve FX sends to the existing bus index and include FX edges in the cycle walk. `playback_output_route` disconnects a persisted FX main output instead of falling back to master.
  - `opentrack-app::channel_settings`: list non-cyclic FX returns under a separate `FX Channels` send submenu and show their names in the slot. No new engine path, no change to group route spelling or existing output menus.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Two audio tracks send to one FX channel, the project validates, and the resolved plan contains two sends to the return bus index.
  2. Missing FX, self FX, a main output to FX, and FX/group or FX/FX feedback loops return the typed routing errors.
  3. Existing group send spellings, group cycle errors, and group output menus behave as before (regression tests).
  4. FX-authored OPR saves and reopens with the send intact; full core/app suites and formatting pass.
  5. Engine/export behavior is covered by the existing shared bus tests (`PlaybackOutputRoute::Group` path); no second DSP path was added.
- Affected regressions: routing parse/persist, group sends/cycles, Channel Settings send menu, playback/export plan preparation, project save validation.
- Essential gates: multiple senders to one return; cycle refusal; dangling refusal; no silent master fallback; unchanged group behavior.
- Review owner and milestone stop: user review. Self-review only in this run; audible FX workflow remains step 16.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: based on `927476e`; commit recorded in Git history.
- Changes and reasons: added typed `FX:` destinations, FX-specific routing errors, FX-aware cycle walking and validation, send-plan resolution through the existing bus index, and an `FX Channels` send submenu. Main output menus deliberately still exclude FX. Packet prepared after the source edits for this slice; the scope above is the implemented scope.
- Commands/checks and actual results: core routing/FX test 1 passed plus routing parser tests; core suite 155 passed; app focused send test 1 passed; full app suite 529 passed, 9 ignored after inventory regeneration (regenerate result: 271 leaves, 86 unavailable). Formatting/diff checks run.
- Runtime environment and evidence paths: Windows x64 host; headless only. No live send-menu, audible return, or hardware review.
- Ledger IDs updated; report command/result: current.md updated; README unchanged because Add FX Channel is still disabled until step 15.
- Review findings, rework, elapsed effort if available: self-review only; existing group tests were preserved and the old send test was extended with an FX destination.
- Failures/unknowns/pending acceptance: send-menu visuals, insert processing on a return, offline/live listening and automation behavior remain pending; step 16 reviews the workflow end to end.
- Next ready task: revision-4 step 15, enable Add FX Channel throughout the UI.
