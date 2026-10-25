# T022 - Preferences evidence and functional VST Metering

Date: 2026-10-25. Implementer: Codex current session, no model switch/subagents. Status: review_pending. User reports EQ/meter speed unlike expected DAW behavior, supplies32 captures and requests File Preferences, documentation excluding Video/Scores, and immediate functional VST Metering.

## Evidence and boundaries

See [complete inventory](../../reviews/preferences-reference-inventory.md) for all32 reference IDs, visible controls, values, exclusions, gaps and dependency plan. Source PNGs were copied, named and organized with SHA-256/dimensions in `G:/dev/Opentrack/_analysis-tools/reports/preferences-2026-10-25/`. Originals were not renamed/deleted. No reference app interaction. Desktop resolution/scaling unknown. Static observed values are not proven factory defaults or timing measurements. Broadcast Wave child has no capture. Existing T020/T021 source preserved; no project file opened/saved.

Source diagnosis: previous filled-meter decay was fixed at40dB/s (10^(-2*seconds)), no separate timed peak marker; FFT frames were painted without temporal smoothing. Root redraw scheduling omitted output-only residual meter decay. These are source findings; they do not quantify the user's perceived lag or prove reference ballistics.

## Delivered scope

File Preferences is available in the empty shell and projects. Left hierarchy excludes Video/Scores; selected page on first open is VST Metering. Other pages expose only clearly marked planned descriptions; preset Store/Rename/Delete are visibly unavailable. Working meter settings use a separate draft: Defaults stages, Apply commits without closing, OK commits/closes, Cancel/title close discard unapplied changes. Global preferences persist through eframe application storage, without .opr schema, project undo/dirty or engine/device reconfiguration.

Captured3000ms peak hold,20dB/s slow,40dB/s fast are the chosen initial settings. Filled meter release uses slow rate; stereo peak markers hold then release using fast rate, never below the filled signal. This slow/bar-fast/marker assignment is our explicit implementation choice pending reference timing measurement. Numeric maximum dB still holds until clicked as separately requested. All Project/Mixer/Channel Settings use shared data/paint helpers. Output and marker residual decay schedule repaint. Timing uses elapsed seconds, so frame rate does not determine decay.

A separated OpenTrack Spectrum Analyzer setting adds release smoothing, default180ms;0 disables it. Immediate attack, exponential release of displayed FFT dB values. This is a display-only improvement rather than a reference screenshot control; neither audio buffers nor latency changes. FFT remains on UI thread with bounded callback capture. T021 direct-live-MIDI track-analysis gap remains.

Map Input Bus Metering in hardware Direct Monitoring is disabled and explained: hardware direct monitoring is not implemented, and falsely mapping the global stereo bus to every track would report the wrong channel. Current software-monitored track meters retain real routing/pan/gain behavior. This reference feature remains absent, not accepted. Meter numeric ranges are chosen implementation limits, not inferred screenshot ranges.

## Checks and review

Release tests, final build/source identity and exact counts appended at handoff. Check timed hold/release under different frame partitions, reference-value defaults, numeric validation, spectrum release/raw option, preference persistence encoding, tree exclusions/rendering, and actual Apply/Cancel/Defaults/OK events. Affected recording/Project/fader/analyzer regressions must pass.

OpenTrack was detected running; user explicitly confirmed closing it before build. No OpenTrack/hardware GUI launched by agent. Leave application closed. Live meter feel/background performance, hardware, screenshot fidelity and restart preferences acceptance remain pending. No screenshot parity claim or whole-DAW percentage. Current scoped audio ledger includes preferences/timing requirements and explicit unavailable direct monitoring.

User review: File > Preferences > VST > Metering. Change slow fallback from20 to5 then60dB/s and Apply while monitoring; bar release should change, attack remain quick. Change peak hold to500ms then3000ms and fast rate; transient marker hold/fall must change while maximum number stays retained. Try spectrum0ms/180ms/500ms. Apply then edit/Cancel: last applied values stay. Defaults alone should not apply until Apply/OK; restart checks global persistence without dirtying a project. Check empty-shell access, no Video/Scores and clear unsupported pages. Preserve prior T020/T021 review cases and stop for feedback.

## Final evidence

134 release workspace unit tests pass (78 app,43 audio,13 core), no compiler warnings. Actual button-event test caught the expanding vertical separator clipping the footer; a fixed-height divider corrects it, and Apply/Cancel/Defaults/OK passes with rendered controls. x64 local-asio release build succeeds. No GUI launch/hardware/visual acceptance claimed. Identity: builds/x64/OpenTrack.exe: BDB8BAA55753030063BD86B7E1E96AD9774AD212B660BFF4F0E86BEB0E4B4E7E; crates/opentrack-app/src/app.rs: BD60610803AFD08E71583BB3C118EA00E7A0DBC4BD46437EE3BF49DF1122C6C9; crates/opentrack-app/src/preferences.rs: DC0C923B2E897B77CC522C4A8B9317B13E865C1DD0ED47BB24DD441F821F42E1; crates/opentrack-app/src/metering.rs: 7EBDD2ACAA436C99930C6866B98AD4DB9B4DF21A1CF3788097C94B34CE5A92A6; crates/opentrack-app/src/analyzer_view.rs: 6DC281C9EB85152E7525ED44EDC91FE3E5F54ACEB25184B7C39E35AE639DDB5F; crates/opentrack-app/src/mixer.rs: 9064F9888C6C4A5BC34E00D59BA30C3881D435C8939B278EF7A113BB73A4DA91; crates/opentrack-app/src/channel_settings.rs: E5E01F4CC736B17C40E36EAA0BC5D8CFBB95D6129E611C1F0048F433C68073B8
