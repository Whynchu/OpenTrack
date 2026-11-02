# R5-25 — Secondary ruler review

- Status: accepted
- Scope version and requirement IDs: revision 5, step 25
- Desired observable outcome: the optional second ruler reads the same clock as the primary one at any sample rate, and its format choice survives a workspace round trip.
- Reference evidence (IDs/paths): `ruler_label` and `timecode_tests` in `app.rs`; `ArrangeViewState::secondary_ruler_format` with its round-trip test in `project_file.rs`; `toggle_second_ruler`.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: one test; no product change.
- Acceptance checks: seconds, timecode (30 and 60 fps) and bars agree at 44.1 and 96 kHz; timecode honours the project offset at both rates; samples scale with the rate; the format persists off/on across a workspace round trip (existing test).
- Affected regressions: none.
- Essential gates: none beyond persistence, already covered.
- Review owner and milestone stop: self-review; visual alignment review pending on the desktop.

## Findings

- Both rulers go through the same `ruler_label`, so a format cannot disagree between rows. Samples are the only format whose labels legitimately change with the rate, and the new test pins that difference (352,800 vs 768,000 at eight seconds).
- The format choice is workspace state and defaults off, which the existing round-trip test already covers, so the second row cannot appear in an older project by surprise.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: added `the_second_ruler_reads_the_same_clock_at_any_sample_rate`; no product change.
- Commands/checks and actual results: `cargo test -p opentrack the_second_ruler` → passed; app suite 578 passing with the four known device failures.
- Runtime environment: Kubuntu Linux.
- Ledger IDs updated: inventory regenerated.
- Review findings: none.
- Failures/unknowns/pending acceptance: visual alignment of the two rows against the grid at extreme zoom needs the desktop pass.
- Next ready task: R5-30.
