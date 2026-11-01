# T074 - Audio mixdown/export

- Status: source implementation complete; focused renderer/WAV checks and local-ASIO compilation pass; active playback parity, real plug-in/device listening, and in-app review remain.
- Scope version: 1.
- Roadmap: step 10.
- User outcome: render the current project to a stereo WAV without routing the render through a physical audio output.

## Initial contract

- The render range is explicit in beats. The dialog defaults to project start and the computed end of the project content; the user may edit both positions. Invalid, empty, or unbounded ranges fail before creating a destination file.
- Output sample rate defaults to the project rate and offers 44.1, 48, 88.2, 96, 176.4, and 192 kHz. Output encoding offers PCM16, PCM24, and 32-bit float, matching the recording format vocabulary.
- Mixdown uses the project playback graph: audio clips, MIDI/instrument tracks, mute/solo, channel faders/pan, built-in EQ and inserts, output strip, and enabled hosted VST3 effects. Live input monitoring, test tone, metronome, note audition and meter/analysis side effects are excluded.
- The offline graph owns its processing state. It must not call a live VST3 instance at the same time as the device callback; instantiate isolated effects from a captured project-state snapshot on the render worker. Missing media or a hosted effect that cannot initialize/process is reported and fails the export instead of silently omitting a channel effect.
- Render in bounded chunks on a worker, publish progress, and support deterministic cancellation. Write to a temporary sibling file and rename only after the WAV header is finalized. A failed/cancelled render must not leave a file that looks complete.
- The first milestone is one stereo interleaved WAV mix. Stems, normalization, dither, automatic loudness reporting, and non-WAV formats are follow-ups, not hidden partial promises.

## Source findings

- File > Export > Audio Mixdown was disabled; it now opens the export dialog and runs a cancellable background render.
- `PlaybackPlan` and both playback render paths contain the processing needed for parity, but plan construction and render callbacks are private to `opentrack-audio`; there is no offline renderer API.
- `OpenTrackApp` still assembles `PlaybackTrackSettings` during playback-plan synchronization. It now retains a UI-independent snapshot, so export also works when no physical output device is open; extracting plan preparation from the coordinator remains a follow-up architecture seam.
- `HostedProcessor` is shared state with no clone-for-offline contract. `vst3_host::Runtime` can initialize an instance from project state, which is the intended isolation path; the worker must own this separate runtime and its processing thread.
- `PlaybackClip` can hold streaming source readers. Offline rendering must use an independent reader per clip/render so exporting does not disturb the live playhead or streaming state.

## Progress

- Added `OfflineRenderer` in `opentrack-audio`. It constructs a separate playback graph, renders a half-open beat range in caller-bounded stereo chunks, excludes live input/test tone/metronome/audition, uses independent streaming readers, and rejects incomplete or failing hosted-effect graphs.
- `OpenTrackApp` retains the prepared playback settings separately from the live engine, so export works with no output device and does not share the live stream readers, meters or VST3 processors.
- `cargo test -p opentrack-audio offline_render::tests` passes (3/3) for range validation, bounded/final chunks, and generated MIDI audio.
- Added bounded-buffer stereo WAV output, PCM16/24 and float32 conversion, finite-sample checks, progress and cancellation, temporary sibling cleanup, and replace-on-success file commit on Windows and Unix.
- Added File > Export > Audio Mixdown dialog for beat range, sample rate and encoding. A worker owns a separate VST3 runtime initialized from captured project state; export does not require an audio output device and checks unresolved media before starting.
- Export startup now clones the latest prepared immutable playback snapshot and does not call `rebuild_audio_plan`; this prevents an export request from replacing or restarting the active device graph. If playback setup has not prepared a snapshot, export reports that it is not ready yet instead of mutating the live graph to force one.
- The full audio suite passes (121 passed, 3 manual checks ignored), including renderer range/signal checks, WAV encoding, cancellation cleanup and destination replacement. Native and Windows x64 local-ASIO `cargo check` pass; `cargo fmt --all -- --check` and `git diff --check` pass. Native runtime listening and app interaction remain unreviewed.
- Remaining acceptance includes full playback-plan equivalence, active-playback non-mutation under real-time load, installed VST3 behavior, save/reopen preservation, and review on WASAPI and Kubuntu Linux.

## Acceptance evidence

- Render known audio and MIDI fixtures at each supported encoding/rate; verify duration, channel count, non-silence, peak and sample conversion.
- Compare rendered samples with the live graph for built-in DSP, routing and hosted plug-ins where deterministic.
- Verify missing media/effect errors are actionable, temporary files are cleaned on failure/cancel, and completed WAVs open in an independent reader.
- Verify active playback is not mutated by rendering, save/reopen preserves project data, and cancellation leaves the project unchanged.
- Review the dialog and render progress in the running application; test listening on WASAPI and Kubuntu Linux output.
