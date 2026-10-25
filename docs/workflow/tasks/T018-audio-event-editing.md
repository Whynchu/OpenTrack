# T018 - Audio event editing and supplied reference styling

- Status: review_pending. Implementer: Codex current session; no model switch performed.
- Scope: event-edit-v1, separate from audio-workflow-v6 and transport-v4. No whole-DAW percentage.
- User direction: event color bounds reach lane edges; left-drag events along the timeline and across compatible Audio tracks; same snap/quantize as transport; ghost original and opaque destination; selected lower name band black; trim, fades and event gain from supplied images. Follow-up overrides captured blue handle squares: all trim/fade/gain squares must be white. Also investigate startup white flash and make the File/Edit strip background match the current dark app background with white text.
- Dependencies: existing grid/quantize/time conversion, event audio sources, history/recording ownership, channel-width domain fields and OPR schema. No third-party dependency added; windows-sys enables its GDI bindings for startup erase handling.

## Reference coverage

All five user-supplied cropped images were viewed directly. Full-desktop resolution, scaling, original window positions and capture theme configuration are unknown. No Cubase session or repeat reference capture was used. OpenTrack remains closed for user review; no current desktop screenshot/comparison is claimed.

| Reference | Source under C:/Users/Samue/Pictures/Screenshots | Observed / explicit intent | Source / action | Current match status |
| --- | --- | --- | --- | --- |
| E18-01 | unselected audio track.png | Solid event body and lower colored name strip | audio_events.rs paint; select/deselect | Source + headless geometry; visual pending |
| E18-02 | Changing track bounds (on right) notice the white swaure- thats the grab point for cutting the end and begginning.png | Selected black name strip, lower white edge square for trim | lower-left/right drag | Source + pointer regression; live pending |
| E18-03 | moving track it shows the old one while the new position is shown as well. we should make OUR old one a ghost, the new one being not transparant.png | Drag preview; user explicitly requests old ghost/new opaque | left-button event body drag | Full Project-grid pointer test passes; live/visual pending |
| E18-04 | changing the fade out using the tooling in the window notice the blue square and lines indicating the fade- this should be at start and at end.png | Fade guide line and upper handle; mirror at start | upper corner fade handles | Source/DSP/pointer tests; user overrides square color to white |
| E18-05 | notice the middle blue square and the line across, this is while dragging that middle part down to effectively make the track db less- dragging up increases db- should snap to 0 easily when near it.png | Horizontal event gain line, center handle and dB feedback | center handle vertical drag; double-click reset | Pointer/DSP tests; user overrides square color to white |

## Implementation and decisions

Audio event painting/gestures live in audio_events.rs. Colored bounds fill the entire lane height, without the previous six-point vertical inset or rounded event corners. The lower 13-point band uses black on selection. Waveform detail occupies the body above that name band, retaining signed per-channel samples and actual amplitude. Growing recording bounds also reach lane edges.

Left-button body drag preserves the pointer grab offset and event length. It previews without committing a move until release; the old event is translucent and the candidate opaque. Audio tracks must have matching configured mono/stereo width; incompatible Audio widths and non-Audio tracks reject the drop. The same enabled Grid Type/Quantize interval used by the transport snaps the new start (and trimmed boundaries); Snap off remains free. Horizontal edge holding requests scrolling; this is headless/source checked, not desktop-qualified. Empty-grid click deselects; Escape cancels a gesture.

White lower corner squares trim beginning/end non-destructively. White upper fade squares edit fade durations with straight guide lines. The white center gain square changes event gain vertically, shows dB during the gesture, magnetically returns to 0 dB within 0.75 dB, and resets on double-click. Gain changes, trimming and fades update through the shared project mutation path while dragging; moving commits at release. Undo/redo retain original source identity and recorded-file ownership. Moving or trimming an existing recording does not delete/rewrite its source.

OPR v21 persists source_offset_seconds, fade_in_seconds and fade_out_seconds with neutral legacy defaults. Event gain accepts 0..4 (about +12 dB), while the handle range is -60..+12.04 dB. These ranges, the 0.75 dB unity magnet, linear amplitude fade curves and mono/stereo compatibility interpretation are explicit OpenTrack implementation choices; their exact reference values/semantics remain unmeasured. Fades are continuous edits rather than quantized endpoints. Playback samples honor source trim, event end, fades and gain; the visible waveform uses the same envelope.

Startup investigation: eframe already creates the native window hidden until its first rendered frame. The app previously had no release GUI-subsystem attribute or explicit framebuffer clear. The release binary now uses the Windows GUI subsystem to avoid a console flash; main framebuffer/root background is explicitly cleared/painted; the existing main-window subclass paints native WM_ERASEBKGND black instead of allowing white erasure. The File/Edit strip uses the dark panel fill and white text in both system appearances; popups retain ordinary theme text. Actual startup flash elimination still requires live launch observation; do not claim measured resolution from compilation.

## Validation and review

117 workspace tests pass with local-asio (65 app, 40 audio, 12 core); standard WASAPI workspace check passes. Nine new tests cover event pointer behavior, full bounds/white handles, compatible/incompatible drops, Snap off, trim/Escape, fade/gain/unity, undo, legacy persistence and rendered samples. A full Project-grid regression verifies event dragging reaches another track without seeking the transport or scrolling the canvas. The test-only 2 Hz render-plan fixture was corrected to use a real output rate after EQ setup rejected an impossible filter range; source sample interpolation remains independently exercised.

OpenTrack was closed by the user before changes. No app instance was launched by the agent; EXAMPLE.opr untouched. Source/runtime acceptance are separate. Hardware playback, visual matching, edge-scroll hold behavior, multi-event operations and prolonged gesture performance remain unverified. This slice implements single-event editing only; multi-selection/copy/stretch/crossfade/editor dialogs are not claimed. Accepted Mixer layout is unchanged.

User review: launch and watch for white flash/menu style; load a disposable audio project; select/deselect an event and compare body/name band; drag horizontally under multiple grid settings and with Snap off; cross to matching tracks and reject incompatible widths; trim both ends, adjust both fades and gain in both directions; verify white handles, 0 dB snap, audible processing, Escape and Undo/Redo; save/reopen. Close after review. Stop for feedback before another milestone.

Release built with build.ps1 -Architecture x64 -LocalAsio. Artifact: builds/x64/OpenTrack.exe.

Identity: x64 local-asio EXE SHA-256 579AB98888B4A151B5EDA62E82F2615E05B90A41B94003C34EEAAB4B108B6262; crates/opentrack-app/src/audio_events.rs: 48B81735834FC7B6B6DEAF5EEC3F48E804B49A45BD57FE82F386911E33499C48; crates/opentrack-app/src/app.rs: F7413FEE4F4D9B3768D47AB5ECC60CB5DA8C0DF32F8766AA7BE34681A8DC11E8; crates/opentrack-app/src/main.rs: 0B80617CE97181D46666A34C5942982B319AFAF4990FEA7268A5CAB77F4F5942; crates/opentrack-app/src/native_keys.rs: 583B1340D2EAAB187C72F8ADEC7C89E3D5659D291C5D47D088F56606D1F9A0A5; crates/opentrack-app/src/theme.rs: 3CA86A8836531EB7E1D7642C6BC20F0BC0EE070F160318CC8E37475374502571; crates/opentrack-app/src/project_file.rs: 506A9F197750F2FF2C3320194871D038F0276F42DC5D13C95C192DE0D841B1C8; crates/opentrack-core/src/project.rs: 87A5B7078439DD8605D51BC3D0A56AC71BEF4BA536F071DEFF5C3D066E344784; crates/opentrack-audio/src/lib.rs: FE3F463F4A26ECFD8C999EF06DC7D057C4E57E1E34D6CB7F569F396FBE71D97A
 Publication was authorized on 2026-10-25 with the T019 cleanup/open correction checkpoint; runtime/visual acceptance is still pending. Prior checkpoint: a4a7773.
