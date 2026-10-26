# T031 — Editable transport timing and independent click output

- Status: source implementation complete; desktop and listening review pending.
- Implementer: GPT-6.1 Sol; no subagents or independent review.
- User direction (2026-10-25): keep the click out of Stereo Out; edit tempo and time signature from Transport; choose metronome pitch with left/right controls; remove missing-symbol squares in pitch labels; keep bars.beats.subdivision.tick at two tick digits and carry 100 into the next subdivision.
- Scope: compact transport timing controls, metronome setup/persistence, audio callback routing, shared musical-position formatter.
- Project meter and tempo are serialized in OPR and are project edits. Tempo entry is directly editable in Transport; clicking the time-signature field opens the numerator/denominator editor. Both rebuild the playback plan and preserve the current musical position. Metronome pitch is a global preference; vector left/right buttons change the downbeat in semitones, and following beats stay one octave lower. Pitch labels use ASCII flats (Eb) to avoid missing font glyphs.
- The metronome is added after Stereo Out processing and Stereo Out metering, directly to the device stream. It is not included in Stereo Out gain, mute, EQ, inserts, meter or recording capture. Live listening on the target device remains necessary.
- The transport position formatter now scales bars/beats with the displayed denominator and carries rounded tick 100 to the next subdivision; ticks remain `00`–`99`. Project arrangement grid, ruler and locator math still have broader non-4/4 gaps and are not claimed fixed here.
- Automated checks: cover two-digit tick carry at sixteenth, beat, bar boundaries; 6/8 transport positions; ASCII Eb6/Eb5 names; chosen pitch; cycle/block independence; click bypass of Stereo Out meter and gain; disabled/stopped silence.
- Live acceptance: open the delivered single x64 build, change tempo by typing and dragging, edit numerator/denominator, seek/play and verify timing/click pitch, compare Stereo Out meter/processing with CLICK on/off, listen with click. Save/reopen or Undo/Redo tempo and meter changes. Project grid/ruler behavior in 3/4, 6/8 and other meters must be reviewed separately.
- OpenTrack was not launched for this source/build pass. Do not infer live or visual acceptance from passing tests or build.

## Build and checks

- `cargo test --release --offline --workspace --features opentrack/local-asio --target x86_64-pc-windows-msvc`: 168 passed (101 app, 51 audio, 16 core); 0 failed.
- `cargo fmt --all -- --check`, `git diff --check`, and all three parity-report checks pass.
- Built x64 with `build.ps1 -Architecture x64 -LocalAsio`; build-info confirms `local_asio=true`.
- EXE SHA-256: `E404670BABD6B0E50449FFAD6FE6CA4FA631A19B50C51664BF04E73932B30EA8`.
- Source SHA-256: `transport_panel.rs` `E526264E5A4CC11717DC7E8341A92252957ADC86480FA0BB3DFB6944D3696452`; `app.rs` `BAEEB1ECED4B6F2D23DD1A690FEF390CCA239D8B9F90120BAFC1094C11F70779`; `sample_editor.rs` `0CDCDDAB127E6516EDD1114F3FDB6BC5B9E9DF393CB178AF3E1256AE05654AD1`; `metronome.rs` `7F45EAB73BD85C406D55DECE1A856EFF89820CA19AEC8D53D79871EEEA76AB14`; audio `lib.rs` `0CDE2DA0AB5AE425894557C4A5F90379DE5598BBEB19407E5A934055CC33F96B`.
