# R5-06 — Beat/sample conversion audit

- Status: accepted
- Scope version and requirement IDs: revision 5, step 6
- Desired observable outcome: rendered and scheduled lengths follow `60 / tempo * sample_rate` everywhere, with no hard-coded tempo or rate in conversion math.
- Reference evidence (IDs/paths): `crates/opentrack-app/src/audio_export.rs` (`render_export`, `locator_range`); `crates/opentrack-audio/src/lib.rs` (`PlaybackPlan::new`, clip/note scheduling); `crates/opentrack-app/src/midi_file.rs`.
- Dependencies and decisions needed: R5-04 (accepted). The audit uses the constant-tempo contract; multi-tempo stays out of scope.
- Code scope and architecture boundaries: one regression test across three tempo/rate combinations; no product change.
- Acceptance checks: rendered frame counts equal the computed value at 60/44.1k/2 beats, 120/48k/4 beats and 174/96k/1 beat; suites unchanged otherwise.
- Affected regressions: none (test only).
- Essential gates: offline render length; audible output present in the rendered files.
- Review owner and milestone stop: self-review.

## Findings

- Grepping the audio crate for literal tempo/rate values found only fallbacks for invalid input (`PlaybackPlan::new` defaults to 120 BPM when given a non-finite or non-positive tempo) and test fixtures; production conversion paths take tempo and rate as parameters.
- App-side conversions use `tempo_bpm.max(1.0)` consistently (35 sites), so a corrupt tempo cannot divide by zero.
- The new test pins the arithmetic at three combinations, including a non-integer beat/rate ratio (174 BPM at 96 kHz, 33,103 frames), which is where rounding errors would show.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: `rendered_length_follows_tempo_and_rate_consistently` renders a MIDI sine through `render_export` and asserts the WAV frame count and rate.
- Commands/checks and actual results: `cargo test -p opentrack rendered_length` → passed; app suite 554 passed / 4 known device-dependent roll failures / 9 ignored.
- Runtime environment and evidence paths: Kubuntu Linux, offline render only (no device needed).
- Ledger IDs updated: none.
- Review findings, rework: none.
- Failures/unknowns/pending acceptance: live playback timing at non-default tempos still needs a listening review.
- Next ready task: R5-07.
