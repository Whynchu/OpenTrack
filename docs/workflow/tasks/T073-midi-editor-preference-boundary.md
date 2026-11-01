# T073 - Extract MIDI editor preference ownership

- Status: implementation complete; focused and integration tests pass; release compilation passed; packaging blocked by the running application.
- Scope version: 1.
- Roadmap: step 6, first focused application module.
- Goal: give MIDI editor user preferences a cohesive owner without moving project or audio-thread state.
- Change: `midi_editor_settings.rs` owns MIDI editor setting defaults, persisted keys, input validation, grid choices and position quantization policy. `OpenTrackApp` now composes a single settings value and delegates loading and saving.
- Compatibility: all existing eframe storage keys/defaults are retained. Controller selection remains session-only as before. No project data, audio processing, dependency or external behavior is intentionally changed.
- Verification: `cargo test -p opentrack --bin opentrack midi_editor_settings::tests` passed (3/3); `cargo test -p opentrack --bin opentrack editor_recovery_tests::expression_lane_is_visible_and_keeps_its_size_after_drag_release` passed (1/1); `cargo fmt --all -- --check` and `git diff --check` passed.
- Runtime review: not performed; native desktop controls are unavailable in this session. T072's separate Snap gesture and restart-persistence review is still pending.
- Release compilation: `build.ps1 -Architecture x64` compiled with local ASIO enabled. Staging replacement stopped because `builds/x64/OpenTrack.exe` was in use by process 35800. Candidate executable: `.build-cache/package-staging/x64/OpenTrack.exe`, SHA-256 `39D9215497EA9D31F9A57C771A27B296511EDCC3F904598FA1E21076B182A7C3`; packaged file was left untouched.
- Remaining boundary work: callers still pass individual setting fields into the large MIDI editor renderer. Further extraction should first specify an explicit editor command/state interface and avoid widening mutable access to project state.
