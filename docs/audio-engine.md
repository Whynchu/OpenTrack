# Audio engine and plug-in rewrite target

## T014 current audio path

The normal build uses WASAPI. `build.ps1 -Architecture x64 -LocalAsio` adds the optional local Rust ASIO host. The driver selector in Devices > Device Setup > VST Audio System enumerates native x64 registry identities; selecting ASIO opens one capture/render session on a dedicated COM owner thread. The same capture feeds input meters, track monitoring, and the recording worker. No driver DLL or SDK is copied. Dependency provenance, safety patches, and BSD notices are documented in `vendor/maolan-asio/OPENTRACK-PATCHES.md` and `docs/third-party-notices.md`.

The installed Mackie driver was opened for read-only capability queries: Analog 1/2 inputs/outputs, Int32Lsb, supported 44.1/48/88.2/96/176.4/192 kHz, buffer limits 8..2048 with power-of-two progression, preferred 128 frames, current rate 44.1 kHz. Streamed hardware monitoring/recording, buffer qualification, driver control panel, reset/hotplug, and coexistence remain pending. Current support is the first mono/stereo pair, fitting the Onyx Producer 2-2; arbitrary multichannel buses remain unimplemented.

Track meter telemetry now carries separate L/R peaks after track DSP, gain, and pan. Stereo Out receives the summed output meter. Raw input metering remains active with an open project even when track monitoring is off. Project, Mixer, and Channel Settings share the UI-owned snapshots and elapsed-time decay. The recording writer uses persisted project rate and PCM16/PCM24/Float32 format, with linear rate conversion when necessary; integer conversion clamps/rounds without dither. Existing files retain their format. These are source facts and offline checks, not live acceptance.

The following older design notes include historical limitations; T014 and its ledger own current delivery status.

## Backend choice

The `opentrack-audio` crate uses Windows WASAPI streams for native capture and render. It uses IAudioClient3 event-driven shared streams when available, exposes a visible standard-shared fallback, and provides explicit opt-in exclusive mode. CPAL remains for endpoint inventory and non-Windows streams. No ASIO SDK or vendor agreement is required for this path.

Shared mode is the default so other applications can use the Windows mixer. Exclusive mode may take ownership of the selected endpoint and can prevent other applications from using it. The application reports the selected mode and negotiated period details; actual device behavior and end-to-end latency still require hardware measurement.

## Implemented

- Select capture and render endpoints independently by persistent endpoint ID. Input testing runs without Record and exposes separate channel meters. Active endpoint enumeration errors are surfaced in the Device Setup panel.
- Capture F32, I16, or U16 input through a bounded, preallocated queue. WASAPI callbacks normalize and publish samples; a worker thread writes 32-bit float WAV data. Capture packet timestamps, discontinuities, queue drops, and timestamp errors are retained for recording summaries.
- Per-track input monitoring selects Stereo, Left, or Right from the first two input channels and flows through that track's processing and output route. Output choices are Stereo, Left, Right, and Disconnected. Monitoring uses an atomic stereo ring and output-rate conversion; separate capture/render clocks and monitoring latency remain unmeasured.
- Recording creates numbered WAV takes under the managed project folder's `Audio/Recordings` directory and creates separate track-named files for multiple armed tracks. There is no destination dialog when Record starts. On stop, takes are finalized, added as clips at the record-start position, and included in the native project.
- Report recording file/stream errors and dropped input frames. The queue holds 262,144 stereo samples (about 2.73 seconds at 48 kHz); full queues drop whole frames, report the count, and the writer inserts silence at those frame positions to preserve take duration.
- Import mono or stereo PCM/float WAV files. External files are copied on a worker thread into the managed project folder's `Audio/Imports` directory using unique destination names; files already inside the project remain in place. Sources up to 512 MiB of decoded float samples are loaded in memory; larger sources use a per-clip 32,768-frame atomic sample ring fed by a background reader. Large-file waveform peaks are scanned separately in the background.
- Place WAV clips on audio tracks and save their source paths in the native project. Paths remain absolute, and legacy linked media is not copied during project-folder migration.
- Relink unavailable or unreadable WAV sources from the File menu. The replacement is decoded before project paths change; all clips sharing the old path update together and the edit can be undone.
- Mix clip playback through the default output device with track mute, solo, Solo Defeat, gain, four-band EQ, and an ordered insert chain that users can reorder in Mixer Channel Settings. EQ and insert state are serialized per track.
- Add an OpenTrack- Compressor insert with a stereo-linked peak detector, threshold, ratio, attack, release, and makeup controls. Its detector uses attack/release smoothing and a hard-knee static curve; all parameters are validated before playback-plan construction. This is an original native effect and has not been compared with a Cubase compressor or plug-in.
- Show live normalized input peaks on the armed track while recording and post-fader channel peaks during playback. Playback peaks are accumulated by track over each device buffer and published atomically; the UI shows a -60 to 0 dBFS scale, a clip indicator above 0 dBFS, and a short peak falloff. These meter positions and response are OpenTrack- choices, not Cubase measurements.
- Render persisted MIDI notes through a Rust instrument factory/processor interface. Instrument tracks can select sine, square, sawtooth, or triangle starter oscillators. The playback plan creates processors off the callback; each processor renders stereo frames through a bounded 64-voice note scheduler and track EQ, inserts, mute/solo/Solo Defeat, and gain. These oscillators are not Cubase instrument rewrites and have no selectable presets, expressive envelopes, or MIDI-device output.
- On Windows, enumerate and open one MIDI input using the WinMM API through `windows-sys`. Its callback parses note-on, note-off, and control-change messages into a bounded queue; the UI thread records them to every armed MIDI/Instrument track and can feed a separate bounded single-producer/single-consumer queue for MIDI Thru. The live starter oscillator follows the armed track's waveform and gain, but bypasses its EQ and inserts. MIDI input timing, callback behavior, and MIDI Thru have not been accepted with physical hardware.
- Persist part-relative MIDI controller events and schedule them on the project beat timeline. The piano-roll lane exposes CC 1 modulation, CC 7 volume, CC 10 pan, and CC 11 expression; the built-in oscillator applies those controls, and seeks restore control state from checkpoints. Same-frame events are coalesced by controller and callback work is bounded. Other valid controller numbers are retained but ignored by the starter oscillator. Controller response is an OpenTrack- implementation choice pending reference comparison.
- Add TestGenerator as an insert on an audio track. Its sine, square, sawtooth, triangle, and noise modes can generate a signal even when the track has no audio clips. The Rust DSP is an approximation and has not been compared against Cubase.
- The built-in EQ provides four bands, ±24 dB gain, 20 Hz–20 kHz frequency, outer-band shelf/pass filters, and two parametric middle bands. Current Q range and filter formulas are implementation choices pending reference comparison.
- Stereo Delay exposes time (1–2,000 ms), feedback (0–0.95), and wet mix (0–1). Its audio buffers are built on the UI thread and processing runs in the audio callback. It adds no algorithmic latency; its tail advances during playback and is not rendered after transport stops.
- The Compressor exposes threshold (−60–0 dB), ratio (1:1–20:1), attack (0.1–200 ms), release (5–2,000 ms), and makeup gain (0–24 dB). It uses a shared peak envelope for stereo-linked gain reduction. It has no knee, lookahead, sidechain input, or Cubase parity evidence.
- The transport clock advances from the audio callback, and playback uses linear resampling when source and device sample rates differ.
- Cycle playback uses project-persisted left/right positions in quarter-note beats. When Cycle is enabled, the playback plan converts those locators to frames and the callback wraps at the right locator without locking or allocating. There is no crossfade at the boundary, and recording does not create cycle takes.
- Emit silence unless the user starts project playback or explicitly selects Audio menu: Play Test Tone.
- Generate a 440 Hz diagnostic tone at a low fixed level without allocating, locking, or reading files in the audio callback.
- Open a user-selected output endpoint in shared low-latency, shared-compatible, or opt-in exclusive mode. If low-period shared initialization is unavailable, retry standard shared mode and surface the fallback. The shared-low-latency path uses the Windows mix format; project-rate conversion and user-selectable input formats remain incomplete.

Playback plans are immutable snapshots with callback-owned EQ, insert, and streaming-source state. The UI builds replacements, and the callback reads a snapshot without locking; the callback gate prevents simultaneous mutable DSP access. Replaced plans are reclaimed on the UI thread after the callback has finished using them. Large-file reads, waveform scans, and file seeking happen on per-source worker threads. The audio callback reads only atomic sample-cache entries; unavailable frames render as silence and a later UI update reports stream underruns or read errors.

The recording worker has a synthetic end-to-end check that writes float WAV data, decodes it through the clip-import decoder, builds a playback plan, and verifies its first stereo frame without opening an audio device. Synthetic checks also cover input peak capture during queue overflow, callback publication of channel peaks, dBFS meter scaling, monitor-ring fanout, sample-rate conversion, and output callback mixing while transport is stopped. Manual capture, monitor latency, meter response, reopen, and playback on the target PC remain unverified.

Large recorded WAV takes can be attached and played through the same streamed-source path. The current stream uses a fixed 32,768-frame ring per arranged clip and has automated sample, seek, waveform, and playback-plan checks. Hardware stress testing, performance under many simultaneous streamed clips, and waveform behavior on multi-gigabyte files remain open.

The audio recorder supports multiple armed audio tracks with Stereo/Left/Right input selection and separate synchronized files; capture recording supports up to two channels. MIDI recording supports multiple armed MIDI/Instrument tracks from one selected Windows MIDI input. Audio and MIDI can record together. Arbitrary multi-channel routing, per-endpoint channel maps, project-rate conversion policy, cycle takes, pre-record buffer, latency compensation, endpoint hot-plug recovery, and fully transactional capture/render reconfiguration are not implemented. MIDI output ports and MIDI bus routing are also absent. Cycle-loop playback exists, but its locators and behavior are OpenTrack choices that have not been checked against reference evidence. Input stream errors and actual device recovery have not been accepted on the target hardware.

## Not implemented

MIDI output ports and typed MIDI/audio bus routing, arbitrary endpoint channel maps, endpoint hot-plug recovery, measured latency compensation, richer instrument controls and presets, a general-purpose plug-in ABI/host, automation, and time stretching are not implemented. The current WAV decoder handles mono or stereo PCM and IEEE float WAV files; compressed audio and surround sources are unsupported. The test tone is a diagnostic signal, not an instrument or project track.

## Bundled plug-in rewrite target

Bundled plug-ins are not a 1:1 rewrite target. Future effects and instruments will be designed individually with the user, and purchased plug-ins may be studied when requested. A general-purpose third-party plug-in host remains a separate future requirement. TestGenerator and the built-in effects are OpenTrack implementations; their DSP behavior has not been compared against a reference. REA findings about installed binaries are documented in [the plug-in inventory](plugin-rewrite-inventory.md), but static inspection does not establish ABI or runtime behavior.

## References

- CPAL supported platforms and backends: https://github.com/RustAudio/cpal#supported-platforms
- CPAL ASIO build requirements: https://github.com/RustAudio/cpal#compiling-for-asio
- CPAL DeviceTrait: https://docs.rs/cpal/0.18.2/cpal/traits/trait.DeviceTrait.html


## T015 device bus processing

Stereo In processing precedes capture fanout to monitor and recording. The input channel meter is post bus processing; Device Setup retains raw device meters. Stereo Out processing follows the track sum, live MIDI and test tone, before device conversion/clipping. Both bus strips persist in OPR v20, reuse the existing channel editor, and do not create arrange tracks. Capture graph swaps retire on the control thread; callbacks serialize DSP state without taking its retirement mutex. Built-in inserts/EQ are available; bus sends, automation, Listen/Solo, extra buses and arbitrary physical maps remain unavailable. Hardware/visual acceptance is pending; see T015.


## T016 waveform data and recording history

The recording writer publishes growing signed per-channel envelopes about every 33 ms, using 64-frame bins and shared immutable chunks. The UI paints a growing event without waiting for WAV finalization. Completed decoded assets use exact samples at close zoom; streamed assets build 64-frame envelopes in the background. The Project painter draws connected stereo silhouettes at visible pixel density. Hardware/UI cadence and long-take resource behavior remain pending user qualification.

Recording-created takes carry file effects with the undo entry. Undo removes their original filenames; redo uses Metadata/Undo recovery. Recovery is discarded when no longer needed by redo or the saved document. Only recorder-created files are registered. Saved-project Discard restores saved references, and file conflicts fail without replacing another file. Crash recovery has a path manifest but no automated replay. See T016.

T017: waveform drawing uses full event height with a small border margin. Discard tracks and removes unsaved recorder-created takes independently of undo-history trimming, restores saved recording references, and preserves imports. Take naming reserves undo/saved-recovery paths to prevent collisions. Filesystem regressions pass; live user checks remain pending.
