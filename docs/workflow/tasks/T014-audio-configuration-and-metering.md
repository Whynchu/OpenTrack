# T014 — Audio configuration, Mackie ASIO, recording formats, and meters

Status: correction source implemented and x64 local-ASIO release built. Actual model: GPT-6.1 Sol. User requested leaving OpenTrack closed; visual and audible review remain pending.

## Correction pass after the 38EAF574 review

The user confirmed good response in WASAPI Exclusive, but reported missing Mackie ASIO selection, mismatched configuration layouts, combined L/R track meters, no Stereo Out meter, no discoverable existing-project recording settings, stopped input-bus meters when monitoring was off, and incorrect mono/stereo input menus/defaults.

Implemented source:

- Independent L/R callback peaks after track processing, fader, and pan; summed Stereo Out telemetry including direct MIDI output. Project, Mixer, and Channel Settings share decayed stereo snapshots. Input metering runs for an open project independently of audible monitoring.
- Optional local ASIO build (`build.ps1 -Architecture x64 -LocalAsio`). A vendored BSD Rust host opens the installed native driver through one COM owner thread and one duplex callback; metering, monitoring, and recording use that same capture. No SDK/driver binaries are copied. Reviewed dependency patches include callback-table lifetime, notification concurrency, native registry view, actual fallback buffer size, and corrected buffer granularity. See `vendor/maolan-asio/OPENTRACK-PATCHES.md`.
- Device Setup uses a left navigation tree with VST Audio System, the selected driver, and MIDI Port Setup. The selected driver's page exposes actual rates/buffer choices and the driver Control Panel. Connections uses the observed compact tabs/toolbar, full-height table, fixed-width headers, and indented L/R rows with real Analog 1/2 names for Mackie. Additional bus creation, editable physical maps, presets, Click routing, and advanced connection types remain unavailable.
- Project Setup is reachable from Project, Shift+S, the recording-status readout, and Device Setup. Existing projects can change record rate/encoding; original media is retained. v19 migrates legacy mono tracks storing Stereo In to Left - Stereo In. Stereo In is a direct choice for stereo tracks; mono tracks choose left/right. Mixer, Channel Settings, and Inspector operate on the same route fields.
- Explicit unavailable output selections remain selected instead of changing to another device. Driver change/reset events stop the session while preserving the project for reconnect.

### Mackie capability query (read-only, no stream)

The exact installed Mackie ASIO Driver opened successfully. Driver-reported version: 1071 (installed DLL version remains 4.47.0.0). Current rate 44100 Hz; preferred buffer 128 frames; limits 8..2048 with granularity -1 (doubling sizes). Analog 1 and Analog 2 appear as both inputs and outputs with Int32Lsb samples. Supported queried rates: 44100, 48000, 88200, 96000, 176400, 192000 Hz. Reported input/output latencies: 238/198 samples, approximately 5.397/4.490 ms at 44.1 kHz. These are driver reports, not a loopback measurement. The query did not change rate/buffer or start audio.

### Correction validation and review

The workspace checks and targeted regressions cover post-pan L/R peaks, master routing/No Bus, input metering without monitoring, one shared ASIO capture feeding monitoring/recording exactly once, recording formats, persisted rate/encoding edits, legacy mono migration, and valid driver buffer choices. The vendored host's fake-driver tests exercise duplex callbacks, formats, one-session ownership, and start/stop/disposal. Current build identity and results are maintained in the ledger.

No OpenTrack instance was launched or screenshot captured in this pass. The user chose to leave it closed while using the PC. Display resolution/scaling, actual window placement, menu appearance, meter response, ASIO audio/recording, buffer changes/control panel, background behavior, and sharing/recovery remain live review items. The reference dimensions are recorded below; no runtime parity is claimed.

| Coverage | Reference | Current image / conditions | Source | Action | Match |
| --- | --- | --- | --- | --- | --- |
| Device Setup tree/page | REF-A14-01, 1894 x 1033 full capture | Pending; app left closed by user request; scaling unknown | app/audio_configuration.rs | Devices > Device Setup | Unverified |
| Connections Inputs | REF-A14-02, 834 x 439 | Pending | app/audio_configuration.rs | F4 > Inputs | Unverified |
| Connections Outputs | REF-A14-03, 834 x 438 | Pending | app/audio_configuration.rs | F4 > Outputs | Unverified |
| Input/track/output meters | REF-A14-04/05/06 | Pending | audio/lib.rs, app/mixer.rs | Input 1/2; monitor off/on; pan L/C/R | Unverified |
| Existing-project rate/format | User requirement; exact reference dialog unknown | Pending | app/audio_configuration.rs | Shift+S or recording readout | Functional source; runtime pending |

## Correction build identity

Artifact: `builds/x64/OpenTrack.exe` (local-asio enabled), SHA-256 `634F3128A3939259BCAB8C4E255FA4FAFD7C0E61C0108797416557754F69060C`.

Build command: `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64 -LocalAsio` (process-scoped invocation; machine policy unchanged). The build script verified no OpenTrack process before copying. Default WASAPI and local-ASIO compile checks passed. Workspace tests: 46 app + 31 audio + 9 core = 86 passing. Vendored host: 34 unit tests + one doc test passing. Final small idle-release and Debug-format safety changes compiled and rebuilt after those tests. No live app/stream was opened; hardware work was limited to the read-only capability query above. User-requested visual/audible review stays pending.

## User direction

Use the installed Mackie ASIO Driver on this PC through an optional local backend. Keep the existing WASAPI path available for the normal application build and shared Windows audio. Driver installation is already handled; the work is host integration. Do not copy or redistribute the installed driver DLL.

Restore the configuration hierarchy shown in the supplied captures: Devices → VST Connections (F4) and Devices → Device Setup. Audio remains the audio-editing menu. Allow selecting recording resolution (16-bit PCM, 24-bit PCM, 32-bit float), project sample rate including 48 kHz, and an actual supported device buffer. Display resolved device names and actual settings. Correct sluggish/frozen Mixer meters and restore the Project track meter beside the color strip. Preserve the accepted Mixer geometry, startup without a project, project-folder workflow, and working audible monitoring.

## Evidence reviewed

Reference images remain outside Git at `C:\Users\Samue\Pictures\Screenshots\`. No reference application or driver was opened. Display scaling and capture times are unknown. These are static images; they do not establish hover timing, context menus, meter ballistics, or driver sharing behavior.

| ID / file | Pixels | Observed facts | Unknowns |
| --- | --- | --- | --- |
| REF-A14-01 `full window.png` | 1894 × 1033 | Project above Mixer; separate VST Connections and Device Setup windows; Mackie ASIO Driver selected; reported input/output latency and 44100 Hz shown | Actual hardware loopback latency; buffer setting; project record resolution |
| REF-A14-02 `VST connections -Inputs.png` | 834 × 439 | Inputs/Outputs/Group-FX/External FX/External Instruments/Studio tabs; Add Bus and presets row; columns Bus Name, Speakers, Audio Device, Device Port; Stereo In with left/right children mapped to Analog 1/2 | Add Bus dialog contents, context menus, preset actions |
| REF-A14-03 `VST connections - Outputs.png` | 834 × 438 | Matching output hierarchy; Stereo Out with left/right Analog 1/2; additional Click column | Click routing behavior and all unsupplied interactions |
| REF-A14-04 `mixer input decibel meter.png` | 84 × 370 | Stereo In bus strip has a narrow stereo meter alongside its fader, color progression, scale/peak readout | Meter tap, falloff, hold duration, reset gesture |
| REF-A14-05 `project window.png` | 199 × 45 | Narrow vertical meter at track header's right edge beside color strip | Exact tap and timing; mono/stereo display rules |
| REF-A14-06 `mixer track decibel meter.png` | 83 × 397 | Audio 02 has Left - Stereo In input, Stereo Out output, mono label, monitor active, and its own fader-side meter | Exact meter mode and peak-hold/reset behavior |

Read-only local registry/file inspection confirms `Mackie ASIO Driver`, CLSID `{AE035966-7908-4E7C-B724-E358222D9718}`, registered in the 64-bit ASIO registry. Server: `C:\Program Files\LOUD Technologies Inc\Mackie USB Driver\W10_x64\loudusbaudioasio_x64.dll`, file version 4.47.0.0, PE machine 0x8664 (x64). The driver has not been initialized or queried, so supported rates/buffers, port labels, and multi-client behavior remain unmeasured.

Prior delivered build identity associated with the meter report: EXE SHA-256 `50632E097BE3518FF49379575B40FA4C87EED2A549AADA414DE3DAC16C36FB13`; app.rs SHA-256 `470D8E9948D8428C540E2EAD334E4249829A8AD0DF648983E11D7ADA2BA81E47`. User reports promptly audible input but sluggish Mixer meter, missing Project meter, and background freeze. This is partial hardware feedback, not an independently measured latency pass; the interface's direct-monitor path must be excluded during software-monitor acceptance.

## First-slice source findings (historical)

- The latest source updates meter decay from monotonic elapsed time and schedules 20 ms repaints while monitoring, recording, playback, or decay is active. Capture exposes independent left/right peaks; Project uses its track snapshot, and the Mixer input bus draws separate left/right meter fills. Raw input peaks no longer replace post-processing track peaks. Focus/background behavior remains for live user verification.
- `show_project` now receives the track-meter snapshot and paints it beside the track color strip. This is source implementation only; visibility and timing remain unverified in the running app.
- Audio menu no longer owns device configuration. Device Setup is reachable through Devices, including without a project. Devices exposes VST Connections and F4. The current Connections window reports the active WASAPI stereo endpoints and channel positions read-only; bus creation, port assignment, and advanced bus types are not implemented. Empty-project Devices correctly leaves project Connections unavailable.
- Device Setup resolves the active capture name and shows actual input/output engine modes, rates, periods, and buffers where the current WASAPI backend exposes them. The Mackie ASIO driver is not yet selectable or opened by OpenTrack. No driver capabilities, rates, buffers, latencies, or control panel have been queried.
- Project Setup persists project rate and recording format; new projects default to 48 kHz / 24-bit PCM while older serialized projects retain 44.1 kHz / 32-bit float defaults. The WAV writer creates 16-bit PCM, 24-bit PCM, or 32-bit float files and linearly resamples to the project rate with rounded frame counts. Integer encoding clamps and rounds directly; no dither is applied. Offline tests cover the writer formats and 44.1-to-48 kHz duration. Hardware recording/reopen remains unverified.
- Track routing remains string-based (`Stereo In`, `Left - Stereo In`, `Right - Stereo In`, `Stereo Out`, `No Bus`); persistent typed bus identities and editable physical mappings are not implemented.

## First review build (historical)

- x64 release output: `builds/x64/OpenTrack.exe`, SHA-256 `38EAF574F1C51C66A5FB9124440A015660636241FA697DA145E9C11B729BBF25`.
- `cargo check --workspace --target x86_64-pc-windows-msvc` and the x64 release build passed. The workspace suite passed before the final UI-only Connections and stereo-meter presentation edits; those final edits passed compile checks.
- OpenTrack was not launched. No current-app screenshot or hardware results exist for this build. Ask the user to review the binary before recording runtime acceptance.
- WAV integer encoding currently clamps and rounds without dither. Project sample rate and encoding are persisted; exact quality of linear interpolation at nonmatching device rates is not professionally qualified.

## Intended settings ownership

| Location | Responsibility |
| --- | --- |
| Devices → Device Setup → Audio System | Backend, installed driver/device, resolved active name, supported rate/period/buffer, driver Control Panel, reported input/output latency, release policy, errors and reopen |
| Devices → VST Connections (F4) | Project input/output buses, mono/stereo layout, physical channel assignments, names, No Bus, unavailable routes; same IDs used by Project/Inspector/Mixer |
| Project → Project Setup | Project sample rate and WAV recording format; recommended new-project default 48 kHz / 24-bit PCM, existing projects retain their interpretation |
| Audio | Existing audio-editing commands; remove device-configuration duplication after the Devices workflow is usable |
| Project status readout | Actual recording configuration and storage-derived remaining record time |

Keep our preferred palette. Follow the supplied positions, density, tables, tree nesting, fader/meter grouping, and window hierarchy. Preserve unsupplied behavior as unknown. Advanced Connections tabs may retain their place only with clear unavailable states until implemented; no controls that appear functional without a processing consumer.

## Delivery queue

### A. Correct live meters first

1. Define explicit taps: physical input bus before track processing; track output after track processing/fader; master after summing. Confirm any desired reference meter-mode differences separately. Stereo buses retain two channel peaks; mono track meters reflect the selected mono route.
2. Publish bounded channel peaks from audio callbacks. UI owns smoothing/hold/decay. One root update consumes telemetry and shares a timestamped snapshot with Project, Mixer, and Channel Settings; child windows must not consume/reset one another's peaks.
3. Schedule 30–60 Hz repaints while monitored input, playback, recording, or meter decay is active, including visible native child viewports when the main app loses focus. Use monotonic elapsed time for falloff and stop repainting when all meters settle. Do not add locks, GUI calls, or disk work to callbacks.
4. Fill the existing Project meter slot beside the color strip using the same track snapshot as Mixer. Maintain existing strip widths/track layout. Remove the raw-recording-peak overwrite of post-processing track meters.
5. Validate separated input signals, silence decay, mute/fader changes, clipping and background focus. Meter ballistics are an initial explicit design until reference timing is measured.

### B. Put configuration in the correct menus

Enable Devices → Device Setup and Devices → VST Connections/F4; keep Device Setup usable without an open project. Preserve the Audio menu's editing structure. Build a Device Setup navigation tree with Audio System and the selected driver page, plus existing applicable MIDI settings. Implement only actual options, showing an unavailable reason for unsupported settings. Move the existing configuration controls through shared application commands, avoiding duplicate setting state.

### C. Optional local Mackie ASIO backend

Add a Windows-only host adapter behind an explicit backend/session interface. Keep first-party control/audio logic Rust. Evaluate the exact host binding/dependency and its terms before adding it; this plan does not change the project license or redistribute the SDK/driver. An optional local build feature is the initial target, while WASAPI remains available.

Discover installed drivers by stable registry identity; select the exact Mackie driver, initialize one full-duplex capture/render session, query ports/sample representations/supported rates/buffer constraints and reported latencies, and expose the vendor Control Panel. Convert native sample representations at the boundary, including 24 valid bits in a 32-bit container. Respect driver callback lifetime, stop/dispose order, rate/reset/buffer-change callbacks, thread ownership, and failures. A driver reset must preserve the open project and release stale buffers safely.

Show the actual active buffer as frames and milliseconds; offer only validated choices. If a driver's buffer is configured through its panel, open that panel and re-query after applying rather than pretending a requested number became active. A system-default selection displays the resolved endpoint name; explicit Mackie ASIO selection displays Mackie ASIO Driver and its physical ports.

ASIO multi-client coexistence is an acceptance test, not an assumption. Keep shared WASAPI selectable for listening to games/browser audio. The same-device/shared-rate combination, different-device outputs, and deliberate driver release are separate cases. Do not silently switch backend or system-wide format.

### D. Real buses and VST Connections

Introduce stable typed input/output bus IDs with named mono/stereo channel assignments. Physical port mapping belongs to the selected backend; project buses and track routing are persisted project state. Migrate existing Stereo In/Left/Right/Stereo Out labels explicitly; missing devices retain unavailable routes instead of selecting another microphone.

Implement Inputs/Outputs tables from REF-A14-02/03: parent bus rows and child channels, physical port selectors, Add Bus for mono/stereo, remove/rename, No Bus, and routing validation. Project, Inspector, Mixer and Channel Settings use those bus IDs. Input-bus Mixer strips and their raw-input meters represent real buses. Presets, Click, and advanced tabs require separate behavioral evidence or explicit user specifications before claiming complete parity.

### E. Project sample rate and WAV recording resolution

Persist independent project fields for sample rate and recording encoding. Offer 16-bit PCM, 24-bit PCM, and 32-bit float. Internal DSP remains floating point; this does not force a 32-bit recording or physical hardware mode. Integer files require explicit scaling/clipping and a documented dither policy applied once on the writer side when reducing precision. Existing WAVs are never rewritten by changing this setting.

Show 44.1/48 kHz and higher standard rates when supported by the relevant path. Query driver support; unavailable rates explain why. Distinguish project rate, capture rate, render rate, and model/plugin rate. If capture and project rates differ, recording requires continuous conversion with correct frame counts, timestamps and duration; a header-only change is invalid. Low-latency WASAPI continues to respect the Windows mix format unless the user explicitly changes backend/mode. Reuse an evaluated streaming converter for capture/render/model boundaries; avoid introducing several conflicting conversion paths.

Update remaining-record-time calculations for selected file encoding, mono/stereo buses and armed track count. Block format/rate changes while recording. Save/reopen retains the selections; older projects migrate with their previous recording interpretation rather than receiving a silent new default.

## Acceptance and review

For each slice: source/build identity, targeted offline tests where meaningful, exact requested/actual settings, and user hardware checks. Close the existing app through its UI before building; one instance; close after review. No desktop action is authorized by the supplied images alone; coordinate a live session with the user. Keep EXAMPLE untouched during destructive checks.

- Meters: feed input 1 only, then input 2; verify bus separation and correct track route, Project/Mixer agreement, predictable decay and clipping, focus another app and verify visible meters continue. Prove software monitoring by changing track processing with interface direct monitoring disabled.
- Driver: confirm active Mackie ASIO identity, Analog 1/2 mapping, supported 44.1/48 kHz and buffer choices; successful reopen/reset/release; no second monitoring path. Record actual driver version and settings.
- Formats: write deterministic 16/24-bit PCM and 32-bit float WAVs; independently decode header and samples, verify channel mapping, quantization/clipping/dither policy and exact duration. Then record/reopen real disposable takes at 44.1 and 48 kHz.
- Persistence: save/reopen buses, record format/rate and track assignments; no save prompt from merely metering; migration preserves source files.
- Audio: test browser/game coexistence per backend, loopback latency, discontinuities/drops, and sustained monitoring/recording before claiming a qualified buffer size.
- Visual review: compare full windows and the three meter areas using supplied image IDs, record display scaling/window positions, and store comparisons outside Git. Stop at each bounded user review.

## Metrics and unanswered questions

New inventory: `audio-workflow-v2`, separate from the earlier limited `audio-interface-v1` snapshot and transport metrics. Membership and weights are declared in audio-workflow-ledger.json. Source baseline is incomplete, so implementation percentage is suppressed; reported meter failures stay failed until verified. Static screenshot matches cannot pass audible behavior. No whole-DAW percentage is available.

Unknown: exact driver-supported settings and multi-client behavior; hardware vs software monitoring in the user's report; reference meter timing/taps/context menus; unsupplied Connections dialogs/preset semantics; reference Project Setup details. Use already collected notes before requesting another reference session.

Primary sources: [Mackie Onyx manual](https://mackie.com/img/file_resources/Onyx_USB_Interfaces_OM.pdf) specifies 24-bit/192 kHz hardware; this is not a measurement of the installed driver's active configuration. [NAM plugin source](https://github.com/sdatkinson/NeuralAmpModelerPlugin/blob/main/NeuralAmpModeler/NeuralAmpModeler.h) reads each model's expected rate and assumes 48 kHz for older models without one; modern plugin rate conversion is separate from OpenTrack's device integration.

Current handoff: use the correction build after it is published below. Review the new menus/meters and select Mackie ASIO Driver through Device Setup > VST Audio System > Audio Driver. Visual, audible, persistence, and stability acceptance stay pending until exercised on this build. The remaining substantial work is typed persistent buses and editable Connections, then measured latency/recovery/coexistence qualification. Preserve EXAMPLE.opr; use disposable recording projects.


## Device Setup crash correction (2026-10-24)

User report: selecting Device Setup crashed EXE 634F3128. Reproduced without a window/audio stream: egui panicked with "You can only indent vertical layouts" because the left navigation Frame inherited the outer horizontal layout. Explicit top-down layout fixes the navigation tree. A regression opens and renders all three pages over repeated frames, including shape tessellation; it failed before the layout change and passes afterward. Added a local panic log for diagnostic evidence from console-free launches.

Validation: 87 workspace tests passed with local-asio; standard WASAPI workspace cargo check passed; x64 release built using build.ps1 -Architecture x64 -LocalAsio. The Visual Studio update temporarily removed linker/SDK files; builds resumed after they returned, without installing a competing toolchain. OpenTrack remained closed; EXAMPLE.opr untouched. Desktop appearance, actual UI opening, and hardware audio still await user review.

Current identity: x64 local-asio EXE SHA-256 5E3A608D48C11B7A4B7AA97A654FE7927487BA7C26E884FA76DC12CE92F0603B; crates/opentrack-app/src/app.rs: 5D3DDF1007BB538FBDE5392E1E0AA5E073D59B95FD84D417B5EB15AEBC54D2ED; crates/opentrack-app/src/audio_configuration.rs: 97997CB901B550C7F7DC2DA94097181EEB2F27E4DF6573424D0F001128071E5C; crates/opentrack-app/src/crash_log.rs: B731326ABE2E1D0F2B374FB8F233D0E557F1FA52B2B8927BC82AB6DD42D730E3; crates/opentrack-app/src/main.rs: 63D48BEBA6FCCED247AF750C07DF34E5022EF4D05E87CE18DEF27CC925F5B1BF

Review: open Devices > Device Setup, select each navigation page, then check the Mackie selector. This is a crash correction, not visual or audio acceptance.
