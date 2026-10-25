# T012 — Usable inputs and low-latency Windows audio

Date: 2026-10-24. Status: first implementation slice builds for x64; hardware acceptance pending. No device or listening result is claimed.

## User requirements

Lowest stable latency, with game/browser/other-app audio audible by default. Exclusive mode is optional. First hardware target: Mackie Onyx Producer 2-2 USB. The current input behavior was reported unusable. Physical I/O precedes third-party plug-in hosting. This path must not require a vendor agreement. Bundled effects/instruments are deferred.

## Source implementation in this slice

- Windows device I/O uses the Rust `wasapi` adapter for capture and render. CPAL remains for device inventory and the non-Windows stream path. The dependency version and license are in `crates/opentrack-audio/Cargo.toml` and the Cargo registry metadata; this does not add an ASIO dependency or SDK.
- WASAPI enumerates active capture and render endpoints with persistent endpoint IDs, labels, and enumeration diagnostics. The UI exposes input/output selection, shared low-latency, shared-compatible, and opt-in exclusive modes, plus an explicit Release/Open action and an idle-release preference.
- Shared low-latency requests IAudioClient3 event-driven periods and reports the selected mode/period detail. When unavailable, the stream retries in standard shared mode and reports the fallback. Exclusive mode is explicit; it does not silently fall back.
- The input test opens capture independently of Record and exposes separate meters for available channels. Capture and monitoring use bounded buffers and a writer/worker thread rather than disk writes on the device callback.
- Track monitor input is mixed through the selected track's processing chain and output routing. Tracks can select Stereo, Left, or Right from the first two captured channels, and Stereo, Left, Right, or Disconnected output. Recording writes synchronized per-track takes; channel selection can choose Stereo, Left, or Right. Input and output endpoints can be selected independently.
- Device changes reopen streams and restore requested monitoring. Recording now starts in the project folder at `Audio/Recordings/Take NNN.wav`; it no longer asks for a destination when Record is pressed.
- The Windows COM/device workers initialize COM on their own threads. Device callbacks use event-driven WASAPI buffers. Capture packet timestamps and discontinuity/timestamp-error counts are retained in the recording summary. MMCSS scheduling is requested for the audio workers.

## Known source gaps and unknowns

- The Onyx endpoint list, channel exposure, usable input signal, and supported periods on this PC remain unverified. The earlier user report that input was unusable is still an unresolved failure, not a pass.
- Input recording currently supports at most two channels. Input test meters can display up to 64. There is no typed multi-channel bus matrix or device-specific channel map.
- Shared low-latency configuration follows the Windows endpoint mix format; a project-rate conversion policy and user-selectable input sample format are not complete. Different input/output device clock drift and the adaptive monitor resampling behavior need measurement.
- Settings application is not yet an atomic transaction across both capture and render streams. Endpoint hot-plug notifications and automatic reopen/recovery are absent. Device failures are not yet mapped into a complete actionable error taxonomy.
- The output route is limited to the first stereo output pair. Record and monitor routes do not yet model arbitrary endpoint channel counts. Input/output latency compensation, pre-record buffer, cycle takes, and offset calibration are absent.
- Shared-mode coexistence with a game/browser, exclusive acquisition/release, and analog round-trip latency are not tested. A buffer period alone is not an end-to-end latency result.

## Acceptance sequence

1. **Input discovery and diagnostics:** Onyx input 1 and 2 produce independent meters, selected endpoint and actual format/period are visible, and missing/locked/unsupported cases explain the failure without silently substituting a microphone.
2. **Shared audio:** monitor and record from the Onyx while browser/game audio remains audible on the selected output. Confirm actual periods and no unintended second monitor path.
3. **Track processing and routing:** monitor stopped, playing, and recording; verify gain, polarity, EQ, inserts, fader, pan, mute/solo, and output route affect only their assigned track/channel.
4. **Recording integrity:** two different inputs create isolated, synchronized takes in a disposable project; save/reopen and playback succeed. Measure alignment before adding compensation.
5. **Exclusive mode and recovery:** explicit exclusive acquisition/release, unsupported/busy handling, return to shared, and endpoint recovery.
6. **Qualification:** capture and monitor for 30 minutes with another app active, then a longer endurance run. Measure reported latency separately from physical loopback correlation; publish median, p95, maximum, and method. Include device disconnect/reconnect and no-dropout observations.

All acceptance steps are pending. User inspection should use the release build recorded below. OpenTrack is not launched as part of this source/build handoff.

## Source/build identity

- Build command: `.\build.ps1 -Architecture x64`.
- Process check before build: OpenTrack absent; no instance was launched.
- Formatting and compile check: `cargo fmt --all` and `cargo check --workspace --target x86_64-pc-windows-msvc` pass. No tests were run.
- Release artifact: `builds/x64/OpenTrack.exe`.
- Release SHA-256: `50632E097BE3518FF49379575B40FA4C87EED2A549AADA414DE3DAC16C36FB13`.
- Source SHA-256: `app.rs` `470D8E9948D8428C540E2EAD334E4249829A8AD0DF648983E11D7ADA2BA81E47`; `project_file.rs` `B6347D91EF91AB8E346B5340D7D0DB075DB0F5F1C77827CD544C90FAA9A2F732`; `wasapi_backend.rs` `3CB60C4CDB1056A33F905DEC9677926252CB3200537B473722E780F6EFFEAB5B`; `recording.rs` `0E296A105CAE61A3260EC705C5E55B79174A8CBD3CD08EEC31CD956F86387E76`; `lib.rs` `770726C5B968D7207F3A5C9535585123EEF9C10F206D0393F0D8B2C37EDAF24D`.
- T012 ledger/report: [audio ledger](../audio-ledger.json), [metrics](../audio-metrics.md).

## Measurement contract

Keep this audio scope separate from the transport-v4 ledger. Use weights 3/2/1 for core/common/polish outcomes. Source coverage, runtime validation, measured reference coverage, workflow readiness, and user acceptance remain distinct. No DAW-wide percentage is available. Onyx format, actual channels/periods, round-trip latency, clock drift, stability, and exclusive coexistence remain unknown until measured on hardware.

## Primary technical references

- Microsoft low-latency audio: https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/low-latency-audio
- Shared initialization and period constraints: https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient3-initializesharedaudiostream
- Exclusive ownership and shared-mode recommendation: https://learn.microsoft.com/en-us/windows/win32/coreaudio/exclusive-mode-streams
- Rust WASAPI adapter: https://docs.rs/wasapi/0.26.0/wasapi/
