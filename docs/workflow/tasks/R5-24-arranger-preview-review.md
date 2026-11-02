# R5-24 — Arranger preview review

- Status: accepted for source and headless checks; audible review pending
- Scope version and requirement IDs: revision 5, step 24
- Desired observable outcome: preview refuses unsupported states with reasons, maps sequence position to source position across repeats, and stops cleanly.
- Reference evidence (IDs/paths): `start_arranger_preview`, `stop_arranger_preview`, `arranger_preview_positions`, `set_transport_playing` in `crates/opentrack-app/src/app.rs`.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: tests only; no behaviour change.
- Acceptance checks: empty sequence, Cycle on and an unprepared playback plan each refuse without starting; positions map correctly inside the first item, at a boundary, inside a repeat, and clamp past the end; stopping playback clears the preview and rewinds to zero; a content change stops preview with a reason.
- Affected regressions: none.
- Essential gates: no silent start, clean stop.
- Review owner and milestone stop: self-review; audible and MIDI-release checks stay with the user.

## Findings

- The preview's state machine is sound: `set_transport_playing(false)` takes the preview, rewinds to zero and reseeks the engine, and a project-content change calls `stop_arranger_preview` with a reason.
- Sequence-to-source mapping uses the play list with a clamp to the last item, so a position past the end resolves to the last source rather than a fabricated one.
- Two limits are real and recorded rather than fixed here: the recording guard uses `is_recording()` (a live session, which a headless test cannot construct), and MIDI note release on stop depends on the engine plan change; both need the desktop pass.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: three tests added; no product change was needed.
- Commands/checks and actual results: `cargo test -p opentrack arranger_preview_tests` → 3 passed; app suite 576 passing with the four known device failures.
- Runtime environment: Kubuntu Linux; no audio device in the harness.
- Ledger IDs updated: inventory regenerated.
- Review findings: my first test set `transport.recording`, but `is_recording()` reads the live session, so that assertion was wrong; the test no longer pretends to cover it and the gap is named.
- Failures/unknowns/pending acceptance: audible preview, rapid stop/seek and MIDI release need the desktop pass.
- Next ready task: R5-25.
