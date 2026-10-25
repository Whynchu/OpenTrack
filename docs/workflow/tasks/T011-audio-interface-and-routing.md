# T011 ? Audio interface, buses and live track monitoring

- Status: partial source/build implementation; user reports inputs unusable. Superseded next-work order: [T012](T012-wasapi-low-latency.md).
- Actual audit/specification model: gpt-6.1-sol.
- User direction: after approving the T010 presentation pass in general, prioritize actual audio interface integration, functional track input/output routing, and monitoring. First target confirmed by user: Mackie Onyx Producer 2-2 USB.
- Scope: audio I/O implementation queue, independent of VST3 plug-in hosting. The transport-v4 metrics denominator is unchanged. No whole-DAW percentage is inferred.

## Source audit ? 2026-10-24

| Area | Current source evidence | Gap |
| --- | --- | --- |
| Backend | opentrack-audio/Cargo.toml pins CPAL 0.18.2 without asio feature | ASIO backend absent from current build |
| Output | AudioEngine::open_default_output uses default_host/default_output_device | No explicit output device, driver, sample-rate or buffer configuration |
| Input | recording.rs AudioRecorder enumerates stable input device IDs and opens a selected device | Default-host capture only; accepts only mono/stereo default configurations |
| Capture | start_input_paths fans one captured stream out to all take files | Armed tracks do not independently select physical input channels |
| Monitoring | AudioMonitorBuffer bridge exists; app attaches it only to a recording session | No stopped/ordinary playback audio monitoring through track Monitor toggles |
| Track routing | Track persists input_routing/output_routing strings and monitor bool | Labels are not a resolved hardware-bus graph |
| Live mix | render_frames adds monitor audio after the playback track mix | Monitor audio bypasses the selected track processing path |
| Output channels | render_f32/render_i16/render_u16 write the stereo mix to channels 0/1; others are zero | No selectable physical output bus mapping |

Read-only registry audit found x64 Mackie ASIO Driver and Generic Low Latency ASIO Driver; x86 entries also exist. Registry presence does not establish that a device is connected, can open, or passes audio. No driver was opened or system audio settings changed. WMI hardware-name query was denied; no retry or elevation was needed because user supplied the model.

## External evidence and dependency decision

- Steinberg's October 29, 2025 announcement: VST3 SDK uses MIT; ASIO SDK offers GPLv3 alongside proprietary licensing. https://ocl-steinberg-live.steinberg.net/_storage/asset/819253/storage/master/Press%20Release%20-%202025-10-29%20-%20VST%203.8%20-%20EN.pdf
- Existing CPAL version has an optional asio feature via asio-sys 0.4.0. Evaluate the pinned release's duplex-stream lifecycle, ASIO channel names/control-panel access, SDK/build requirements and sample formats before selecting it for the production session. Do not blindly enable a feature and assume routing/monitoring follows. https://github.com/RustAudio/cpal
- Mackie Windows driver instructions cover Onyx Producer 2-2 and ASIO/WDM hosts: https://mackie.com/img/file_resources/Mackie_USB_Driver_Instructions.pdf
- No repository LICENSE/COPYING file was found at the root. Resolve and document the chosen SDK/license route before distributing SDK-linked builds; this audit does not adopt a project license or accept a proprietary SDK agreement.

## Architecture and bounded delivery order

1. **Device/session foundation:** choose driver/backend and device, expose actual ports and supported configurations, apply sample rate/buffer, driver control panel when supported, visible stream/error state and failed-apply recovery. One owner manages the duplex device lifetime and audio clock; input and output must not be opened as unrelated competing ASIO sessions. Keep WASAPI as an explicit supported alternative. Device preferences are machine-local; project buses hold stable identities.
2. **Functional bus routing:** named mono Input 1/Input 2 and stereo Input 1+2 mapped to discovered ports, stereo output mapped to chosen physical outputs; stable bus IDs independent of display names. Track No Bus silences/disconnects that branch. Project, Mixer and Channel Settings issue the same routing commands. Preserve old project routing names through explicit migration/default resolution and report unavailable routes.
3. **Live track monitoring:** capture runs while needed independently of recording. Each track's Monitor switch routes its selected input through the documented track gain/polarity, inserts/EQ, fader/pan/mute/solo path into its selected output. Monitor works stopped, playing and recording; input and output meters reflect actual signals. Hardware direct monitoring is a separate interface facility and must not be falsely presented as software monitoring.
4. **Independent recording and reliability:** record each armed track's selected bus to its own source, retain a synchronized start, bounded writer queues, overrun reporting and safe stop/finalization. Measure latency/record offset, verify save/reopen/undo, test missing devices/reconfiguration and sustained operation.
5. **VST3 hosting:** separate later slice for scan, instantiate, bus/parameter negotiation, editor lifecycle, processing, state recall and failure handling. It uses the same track processing path; it is not required to communicate with the interface.

## Acceptance gates

- Device enumeration is source/build evidence until the Mackie driver successfully opens on the actual Onyx.
- Verify Input 1 and Input 2 independently, stereo mapping, No Bus, chosen outputs, stereo balance and channel isolation.
- Toggle the same track Monitor control in Project, Mixer and Channel Settings and hear the same routed result while stopped, playing and recording. Fader/mute/pan/EQ must affect that monitored signal as specified.
- Record two different physical inputs to different tracks; confirm isolation and aligned starts, then reopen a disposable project and play the takes through the chosen output.
- Log actual sample rate, buffer size, reported/measured latency, driver identity, channel names, dropouts, duration, build hash and hardware conditions. Compilation and driver registration do not count as passing audio gates.
- Verify unplug/reconnect and failed configuration preserve the project and produce a useful status. Real-time callbacks allocate no unbounded data, do no file I/O, and do not wait on GUI or driver-control operations.
- User acceptance at each delivery slice; preserve the approved Mixer appearance and the saved EXAMPLE.opr. Use disposable recording projects.

## Handoff

### Device/session foundation implementation — 2026-10-24

Implemented in the x64 source/build:

- Added cross-backend device enumeration using CPAL's compiled hosts. Stable CPAL IDs remain machine-local; input selection now resolves an ID through its owning host rather than forcing it through the default host.
- Added output device selection, supported sample-rate choices, and buffer-size presets when the active backend reports a supported range. The app displays the active device, sample rate, and channel count.
- Saved output device/rate/buffer preferences in application storage, separate from project data.
- Output reconfiguration closes the previous stream before opening the requested configuration. It restores the previous device and transport position/play state if the new stream fails. Asynchronous output-stream errors stop transport and appear in the status message.
- Audio callbacks only set an atomic error flag; device enumeration and reconfiguration happen outside the callback.

Build evidence: `cargo fmt --all` and `cargo check -p opentrack-audio -p opentrack --target x86_64-pc-windows-msvc` passed without warnings. The x64 release build passed after confirming no OpenTrack process was running. Executable SHA-256: `5785DBD36E6EBA6EC0CF26A905C60EF61D6A02025A08C28DC0092F9242498A59`. No tests were run, no app was launched, and no hardware was opened; user/live/audio acceptance is pending.

Still unknown or not implemented in this slice:

- The current CPAL build has no ASIO feature. The Mackie ASIO driver is not selectable from this executable. WASAPI device use is available only if Windows enumerates the Onyx endpoints; that has not been checked in the app. The user has not selected the ASIO SDK licensing route, and no project license file was added.
- The app does not yet own one duplex ASIO session, expose physical channel names/control panel, or measure actual callback/buffer latency. Buffer controls appear only when CPAL reports a supported range.
- Project routing labels are not resolved into stable hardware buses. Mono Input 1/Input 2, stereo pairs, output bus mapping, No Bus, and routing synchronization across Project/Mixer/Channel Settings remain for the next slice.
- Track Monitor still depends on recording's input stream and bypasses the track processing path. Independent per-track capture, monitoring while stopped, input meters, and one-input-per-track isolation remain unimplemented.
- The current recording path still fans one selected input stream to multiple armed tracks. Recording clock alignment and sustained hardware reliability have not been measured.

The existing EXAMPLE.opr was not opened or changed. The current build is staged for the user to inspect; keep the acceptance gate open until the selected Onyx input/output and saved configuration are checked. Next packet: functional stable-ID buses and physical channel mapping, after the backend/licensing decision for ASIO is resolved as needed.

## Updated direction

The user requires low-latency shared audio with other apps audible and optional exclusive mode, without making an ASIO agreement a prerequisite. T012 supersedes the historical ASIO-first decision gate and next packet above. Missing input functionality remains unresolved; diagnose endpoints and expose independent capture before claiming this foundation complete.
