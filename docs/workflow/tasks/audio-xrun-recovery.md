# Audio xrun recovery and buffer size

- Status: fix accepted for source and automated checks; audible/desktop verification pending
- Scope version and requirement IDs: post-revision-5 audio defect reported 2026-11-02 ("ALSA dropouts and the system can't recover without releasing and opening audio again")
- Desired observable outcome: a recoverable output underrun produces a click at most, keeps the engine alive, and is visible as a count; only a genuinely lost device stops playback.
- Reference evidence: user report (2026-11-02); `crates/opentrack-audio/src/lib.rs`; `crates/opentrack-app/src/app.rs`; cpal 0.18.2 ALSA backend.

## Symptom

The user hears ALSA dropouts and afterwards the application stays silent until the audio device is released and opened again (re-applying the output in Device Setup).

## Root cause (observed in source)

1. cpal's ALSA backend calls the stream error callback on **every** underrun, then recovers the stream itself:
   `output_stream_worker` in `cpal-0.18.2/src/host/alsa/mod.rs` matches `ErrorKind::Xrun`, calls `error_callback(err)`, then `stream.handle.prepare()`. It also reports `ErrorKind::DeviceChanged` for a rerouted default-device stream, which cpal documents as "the stream remains active and no rebuild is required", and `RealtimeDenied` when promotion is refused while audio still plays.
2. OpenTrack's error callbacks ignored the error and latched a single boolean for all of them (`lib.rs`, three sample-format branches).
3. The app treats that boolean as fatal: `advance_transport_inner` stops the transport, clears `audio_input_probe`, sets `self.audio_engine = None` and reports "Audio device changed or stopped. Open Audio in Device Setup to reconnect" (`app.rs`).

So one underrun tore down a stream the backend had already recovered, which is exactly the reported "needs releasing and reopening".

## Fix

- `note_output_stream_error` sorts the report by `cpal::ErrorKind`: `Xrun`, `DeviceChanged` and `RealtimeDenied` increment `output_stream_xruns`; everything else latches the fatal flag as before.
- `AudioEngine::output_xrun_count()` exposes the counter (not consuming), and the DSP readout in the project header now shows `· N output xruns` beside `N render overruns` so the two are not confused.
- No change to device-open behaviour, buffer handling or the WASAPI/ASIO backends.

## Test

`cargo test -p opentrack-audio output_stream_health` → 1 passed. It feeds `cpal::Error::new(ErrorKind::Xrun)`, `DeviceChanged` and `RealtimeDenied` and asserts the fatal flag stays clear and the counter reaches 3, then asserts `DeviceNotAvailable` latches fatal and does not count as a glitch.

## Buffer size: what is available

- Device Setup → VST Audio System → the driver page has a **Buffer Size** control (`audio_configuration.rs`, `audio_rate_buffer_fields`): "Driver default" or 64, 128, 256, 512, 1024, 2048, 4096 frames, shown with the equivalent milliseconds at the current rate.
- The choice is saved as `audio-output-buffer-frames`, validated against the device's supported period range when the engine opens, and applied with `cpal::BufferSize::Fixed` on Linux (`lib.rs`, non-Windows branch). cpal configures the ALSA buffer as 2× the requested period.
- Caveats found while answering this:
  - In **Shared - Low Latency** mode the list comes from `engine.output_period_choices`, which the CPAL path always leaves empty, so the dropdown is disabled there on Linux.
  - The preset list is filtered by the device-declared period range; when the device reports `SupportedBufferSize::Unknown`, the list is empty and a requested size is refused at open with "the selected output does not expose a configurable buffer range".
  - The CPAL path reports the *requested* frames, not the negotiated period, so "Driver default" displays `0 frames` in the driver page and in `output_period_frames`.
  - The Windows CPAL path ignores the requested buffer; WASAPI and ASIO honour it.

## Follow-up: the buffer size read "0 frames"

The user reported that the Driver Setup buffer size showed `0 frames`, which is not a real period. The CPAL path stored `requested_buffer_frames.unwrap_or(0)`, so leaving the buffer at the driver default (the common case) reported zero even though the stream was running with a real period.

Fixed by reading the negotiated period back from the stream:

- `StreamTrait::buffer_size()` is available immediately after the stream is created and returns the backend's estimate (the negotiated hardware size for a fixed request, the backend default otherwise). The engine stores that as `output_period_frames` / `output_buffer_frames`, falling back to the requested value, and only reports 0 when the backend cannot say.
- The device's declared period range is captured once at open and turned into the preset list ([64, 128, 256, 512, 1024, 2048, 4096] filtered to that range), so on Linux the Buffer Size dropdown now has real choices - including in Shared - Low Latency mode, whose list is the engine's.
- The Driver Setup label shows "Driver default (unknown)" instead of "0 frames" if a backend still cannot report a period.

Coverage: `period_choices_follow_the_device_range` pins the list filtering. The negotiated-value path itself needs a real device, so it is verified by inspection and stays a desktop check.

## Why underruns happen here (contributing, not fixed)

- **No real-time priority on Linux.** cpal is declared with default features (`crates/opentrack-audio/Cargo.toml`), which excludes cpal's `realtime` feature, and only the ASIO backend promotes its callback thread (MMCSS). A normal-priority thread competing with the UI and the rest of the system misses deadlines under load.
- **Small buffers.** With 64–256 frames at 48 kHz the deadline is 1.3–5.3 ms, and the callback does the whole mix: 32-band static and dynamic EQ, meters, metronome, live MIDI, and VST3 `process` when inserts are hosted.
- **Routing.** The ALSA `default` PCM usually goes through the PulseAudio or PipeWire plugin, which adds its own scheduling and latency on top.
- Enabling cpal's `realtime` feature (which pulls `audio_thread_priority` and `alsa-sys`) is the direct fix for the scheduling half, but it is a new dependency and needs the project's dependency evaluation before adoption; it is not added here.

## Diagnosing the next occurrence

Run with `OPENTRACK_REALTIME_DIAGNOSTICS=1`; the DSP readout then adds render interval and duration p95/max in microseconds. A rising interval with a short duration means the thread was not scheduled (raise the buffer or get real-time priority); a rising duration means the callback itself is too slow (fewer/simpler inserts, larger buffer).

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Changes: classifier + counter in `opentrack-audio`, counter in the DSP readout, one test.
- Commands/checks and actual results: `cargo test -p opentrack-audio output_stream_health` → 1 passed; audio suite 171 passed / 3 ignored; app suite 579 passed / 4 known device-dependent roll failures / 9 ignored (the inventory was regenerated after the app edit).
- Not claimed: no desktop run and no listening test on this host (no display or audio device in the harness), so the fix is source- and test-verified only.
