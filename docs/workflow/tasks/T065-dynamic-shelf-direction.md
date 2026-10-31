# T065 — dynamic shelf direction correction

- Status: review_pending
- Scope: v1, dynamic low/high shelf audio and matching animation.
- Desired outcome: dynamic attenuation does not boost the opposite Q flank; positive movement does not cut it. Preserve static Q shape and existing knob/layout behavior.
- Evidence: user screenshot and explicit approval to fix audio and drawing together. Old Classic low shelf at 500 Hz, Q 0.3, gain -3 dB, movement -9.2 dB boosted 750 Hz by an additional 4.047 dB.
- Decision: retain the static shelf and cascade a monotonic dynamic shelf at the same frequency, using Q=1 (S=1 in the existing formulation). All shelf flavors retain their static character. Effective endpoint remains clamped to -24…24 dB.
- Code scope: Rust audio channel processor/shared response calculation and Channel Settings dynamic envelope/node calculation. Reuse existing secondary filter storage and coefficient smoothing; no callback allocation, new dependency or project schema change.
- Acceptance checks: generated audio matches shared plotted response; both movement signs across all 12 shelf variants and representative Q/gain/frequency ranges; actual painted envelope mesh moves in the requested direction; disable/re-enable/bypass and band disable clear the dynamic stage; Bell/Parametric behavior unchanged; existing project round trip preserved.
- Persistence: existing parameters and undo/dirty handling remain unchanged; saved settings select the corrected DSP after reopening.
- Review owner: user listening and live animation review pending.

## Handoff

- Actual implementer/reviewer: GPT-6.1 Sol.
- Source identity: `901de54` plus T064 and this local diff; unrelated workspace/user files preserved.
- Audio tests: `cargo test --offline -p opentrack-audio` — 110 passed, 3 manual tests ignored. Four new tests include measured ChannelProcessor output.
- UI tests: `cargo test --offline -p opentrack channel_settings::bus_tests` — 19 passed, including shelf envelope mesh direction.
- Persistence: `cargo test --offline -p opentrack built_in_eq_and_tonal_reference_round_trip_through_opr_without_source_media` — 1 passed.
- Local-ASIO compile, formatting and diff checks pass.
- Visual evidence: actual egui graph rendered offscreen for low/high shelves; reviewed `/tmp/shelf-response-review.png`. Temporary renderer removed. No screenshot added to repository.
- Linux release: `cargo build --offline --release -p opentrack --features local-asio` passed; staged in `builds/x64/OpenTrack`, SHA-256 `8eea778ca70b347677c1d9bdbfd0d6fab52db0ecb6614f0c1ca5f39234d0d7f3`. Source/staged hashes match and metadata confirms local_asio=true. Linux uses the native Cargo path because the PowerShell build script is Windows-specific.
- Ledger: current.md and this packet record the corrected behavior; no broader milestone acceptance changed.
- Remaining limits: no live device listening, live desktop animation review, Windows runtime or callback cost measurement. User acceptance pending; source/tests/offscreen evidence do not establish those passes.
- Next ready task: review the packaged build with dynamic shelf playback; preserve T064 window-ordering review needs.
