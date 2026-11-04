<table align="center" width="100%" bgcolor="#292929">
  <tr><td align="center">
    <img src="docs/images/opentrack-github-banner.svg" alt="OpenTrack" width="440">
  </td></tr>
</table>

# OpenTrack

OpenTrack is a standalone open source DAW built in Rust. Its goal is to provide a dependable, expressive environment for recording, editing, arranging, mixing, and finishing music.

**License:** [Apache License 2.0](LICENSE).

![OpenTrack full workspace showing the Project, Mixer, Audio Editor, VST Channel Settings, and effect during playback](docs/images/opentrack-full-workspace.png)

*OpenTrack's full workspace during playback, with the Project, Audio Editor, Mixer, Channel Settings, and VST effect visible together.*

<details>
<summary>More workspace views</summary>

![Project timeline and Mixer with Vocal, Guitar, Rhythm, Lead, Instrument, and MIDI tracks](docs/images/opentrack-mixer-and-project.png)

*Project timeline and Mixer with Channel Settings open.*

![Audio Editor showing the Vocal track's pitch and time view with Channel Settings open](docs/images/opentrack-audio-editor.png)

*Audio Editor and Channel Settings for the Vocal track.*

</details>

## Development status

OpenTrack is an early prototype under active development. Its features are still being integrated and reviewed, and some visible controls are incomplete. The following describes the current development version; availability does not imply that every workflow has completed runtime or hardware validation. The status below is refreshed with every delivered slice.

### What works today

- **Arranging:** audio, MIDI, instrument, FX-return and group tracks plus organizational folders; a Tempo Track editor for the project tempo and time signature; transport and cycle playback, including play-from and play-until selection/marker commands; bar/beat/quantize snapping plus snapping to event edges or the cursor; zoom commands; follow-playhead scrolling; an optional secondary ruler row; an optional dedicated marker lane; markers with next/previous navigation and a marker list; editable named regions with locate, rename, resize, delete and locator/hitpoint creation; arranger sections and a playback-only sequence preview; event nudge, edge nudge, split at cursor or at both locators, glue, range delete and crossfades; MIDI part split and merge; Remove Empty Tracks cleans up media lanes with no events, parts or automation; 16 event color slots with six built-in palettes - Default, Muted Earth, Cool Studio, High Contrast, Pastel and Grayscale; applying a palette assigns each track a slot by track id, Default Color is the standard grey, and any slot can be edited in Event Colors; bounded undo/redo with an undo-history window.
- **Recording:** audio and MIDI capture; Retrospective Record turns the last 60 seconds of MIDI input into a part; punch in/out driven by the locators; per-track input monitoring (Off, Auto while armed, On); each loop pass recorded to its own take lane; a metronome with a per-beat accent pattern and a count-in.
- **Media:** imported WAVs copied into the project; media pool; a project-relative record folder for new pool files; consolidation into the project folder; missing-media relink; unused and missing pool cleanup; several timestamped recovery points; Audio CD import through a CUE sheet, one event per track.
- **Mixing:** Mixer and Channel Settings; input gain and metering; 32-band static and dynamic EQ; live spectrum and tonal-reference analysis; insert rack; up to 24 sends per channel with typed routing, including FX return channels that can receive sends from several tracks. Add FX Channel creates one or more returns with an optional first VST3 effect; FX returns have no record or waveform-edit controls. Folder rows are organizational and can nest tracks and folders; folding a folder hides its subtree without stopping playback or recording.
- **Automation:** track, Group, and FX-return volume/pan/mute curves run in live and offline audio; Stereo In/Out buses also read volume/pan/mute against the project clock. Enabled send level/pan curves apply through routed live/offline audio, and Write captures channel fader, pan, mute, send and hosted VST3 parameter gestures. Plug-in parameter lanes keep stable insert and parameter IDs and schedule bounded sample-offset points. Desktop/device listening and a named real plug-in parameter run remain pending.
- **Plug-ins:** third-party VST3 effect and instrument hosting. Multi-output instruments expose negotiated main and auxiliary buses as owner-locked child channels under the MIDI-owning Instrument track; child channels retain independent processing and routing, and their sum passes through the Instrument parent. Effect inserts keep the main-bus path. Insert editors open from the rack; selected Instrument tracks also have a dedicated **V** editor button in the Project Inspector. The user confirmed MT Power Drum Kit loads and makes sound on Linux; its multi-output playback and desktop presentation still need acceptance on that machine. Compatibility varies by plug-in; universal VST3 support is not claimed.
- **Editing tools:** MIDI editor with note and controller lanes, including a configurable single CC lane (any controller 0–127) whose choice is saved with the project; quantize with strength and swing, transpose and velocity tools; Sample Editor with on-demand Pitch & Time analysis and editing.
- **Delivery:** stereo WAV mixdown (always the locator range), per-track stem export, bounce in place through a track's own chain, Standard MIDI File export of the whole project or a selection, and an OpenTrack Track Archive v1 directory package with manifest, hierarchy, routing reports and copied media; importing it runs in the background with progress, cancellation and stale-result rejection. Direct Offline Processing bakes an ordered chain of built-in processors per event.
- **Project tools:** templates, session health check, project statistics, beat calculator, and a notepad saved with the project.
- **Menus and windows:** open menus block pointer input to the Project underneath, only one top-level menu is shown at a time, and moving left or right across the menu bar switches the open menu until the pointer leaves the bar and its popup. Score Settings and Score Layout open from the Scores menu even when the Score view is closed and no longer disable the project behind them.
- **Platform and format:** Windows x64 and Linux x86_64 are both target platforms. Windows builds use WASAPI or the optional local ASIO adapter; Linux builds use ALSA through CPAL. The native project format is OPR (`.opr`, currently version 48); earlier `.opr`, `.otp` and `.rwp` files remain readable, and older builds refuse a newer file instead of stripping fields they do not know.

### What still needs work

- **Real-device review:** recording, punch, monitoring, loop takes, the metronome click, stem export, bounce and mixdown pass offline and headless checks but still need listening and hardware validation. Those results are not claimed yet.
- **Automation coverage:** Track/Group/FX volume, pan and mute; send level/pan; input/output bus channel controls; and hosted VST3 parameter lanes have source implementation and focused headless checks. Plug-in lanes use persistent insert IDs and target-specific parameter IDs. Detailed verification for the full set, real plug-in parameter behavior, desktop review and listening remain pending.
- **Routing:** device-bus sends and wider physical channel maps are open. Track and group sends and typed routing pass offline checks and still need live review.
- **Arranger:** Project > Add Track > Arranger opens the section/sequence editor; Apply is one undo step, Preview Arranger is playback-only, and the offline transformer renders exact repeated sections. Audible preview, rapid-stop/seek behavior and export of the sequence still need runtime review.
- **Interchange:** CPR/OMF compatibility waits until the feature set is complete (user decision, 2026-11-02); no adapter is planned before then. OpenTrack Track Archive v1 export/import is implemented with a background import; desktop round-trip review remains. Other DAW formats are future work. Authored MIDI fixtures pass; comparison with DAW-produced MIDI files remains.
- **Interface completion:** the project-open main menu has 17 rows still shown disabled (measured from source in the [generated inventory](docs/reviews/generated-source-inventory.md)); 151 of 152 keymap-bound commands have a dispatch arm. The last undispatched command (external sync) names a feature that does not exist yet and is unavailable in its menu. The [accessibility and keyboard report](docs/workflow/accessibility-2026-11-02.md) lists the gaps; a manual keyboard-only pass is pending.
- **New in revision 5, pending desktop review:** Tempo Track editor and tempo-file exchange, Retrospective Record, Direct Offline Processing, Audio CD import, and the background archive import have automated coverage only.
- **Linux audio scheduling:** the packaged Linux build requests RTKit-backed real-time scheduling for eligible ALSA streams. Audio Setup keeps both direct ALSA hardware endpoints and format-converting routes where available; direct hardware can receive RTKit priority, while server-backed and converting routes may remain at normal priority. The DSP readout shows the actual output policy and counts promotion denials separately from xruns. Opt-in diagnostics record Linux callback interval and render time without clock reads when disabled. Five-second silent checks at 48 kHz / 512 frames compared direct HDMI ALSA at RR priority 10 (11,079 us interval p95; 72 us render p95) with PipeWire at normal priority (10,806 us interval p95; 67 us render p95); both reported zero xruns. Muted synthetic load checks with eight tracks through 12 EQ bands each reached 60% worst callback load on direct HDMI at RR 10 and 65% through PipeWire at normal priority; both had zero xruns and RTKit denials. Full saved-project and audible performance review remain open.
- **Linux plug-ins:** native VST3 editors use the X11/XWayland embed path and are owned by the OpenTrack main window before being shown. They use the OpenTrack window identity and icon and request focus through the window manager. DISTRHO MVerb (DPF VST3 1.7) was exercised on XWayland for paint, keyboard focus, pointer parameter editing, resize and close/reopen. Linux host tests also exercise VST3 run-loop FD and timer dispatch. Native Wayland-only embedding, review of updated ownership/stacking behavior, and broader plug-in compatibility remain open.
- **Not available:** video, surround, score editing, macOS and browser builds, and a command-line or MCP interface. Plug-in compatibility and recovery behavior need continued verification.

### Highlights

- **32-band EQ with dynamic processing:** shape a channel with multiple filter types and independent dynamic controls for each band.
- **Built-in spectrum and tonal-reference analysis:** compare live audio with another channel or a captured or imported WAV reference.
- **Up to 24 insert and send slots per channel:** insert slots run available built-in processors and third-party VST3 effects, including native plug-in editor windows. Track, group and FX-return sends have destination, level, pan, and pre/post-fader controls; the signal path passes offline tests and still needs live hardware review. Device-bus sends remain unavailable.
- **Integrated Pitch & Time:** analyze and edit audio in the Sample Editor, alongside the Project and Mixer workspace.
- **Audio backends:** Windows offers WASAPI or the optional local ASIO adapter for a compatible installed driver; Linux uses ALSA through CPAL.
- **Platform:** Windows x64 and Linux x86_64 are both target platforms, built from the same Rust code. Linux packages request RTKit scheduling for eligible ALSA streams and support native VST3 editor windows under X11/XWayland; one third-party editor has passed a native lifecycle check. Native Wayland-only plug-in editors and broader compatibility remain open.

### Built-in EQ and analysis

Channel Settings includes an EQ and analyzer for shaping and inspecting a channel:

- **Up to 32 EQ bands:** add and adjust bands on the graph, with frequency, gain and Q controls, individual band enable and solo, and an overall EQ bypass.
- **Filter choices:** constant-Q and proportional-Q bell filters, low- and high-pass filters, and several low- and high-shelf shapes.
- **Dynamic EQ:** independently enable dynamic processing on a band and adjust threshold, range, attack and release. The graph shows the allowed range and live gain movement.
- **Live spectrum:** view pre- or post-processing signal, compare another channel, and adjust the display range, tilt and FFT resolution. Peak hold and freeze help inspect the display.
- **Tonal reference:** capture a target from playback or import a WAV reference, then compare a longer-term tonal profile against the saved target. Targets are stored with the project.

These features are under active development; live listening, visual behavior and performance across audio devices still need further review.

### Near-term priorities

User-side review of the recording, delivery and listening features above; the CPR adapter once real `.cpr` files are available; the remaining disabled menus and undispatched shortcuts; further Linux VST3 compatibility review and loaded-project audio performance. Windows x64 and Linux x86_64 are both target platforms, not a primary/secondary split.

Progress and verification live in [the current workflow state](docs/workflow/current.md), [the revision-4 delivery handoff](docs/workflow/handoffs/revision-4-delivery.md), the [revision-2](docs/workflow/roadmap-32-steps-v2.md) and [revision-3](docs/workflow/roadmap-32-steps-v3.md) roadmaps, and the [accessibility report](docs/workflow/accessibility-2026-11-02.md).

## Build and run

Windows x64 and Linux x86_64 are the target platforms, built from this repository. Other Linux distributions, macOS and browser builds are not current release claims. On Linux, install Rust, ALSA and D-Bus development headers (`libasound2-dev` and `libdbus-1-dev` on Debian/Ubuntu), then run `./build.sh`; it enables CPAL's RTKit integration for eligible ALSA streams. RTKit can deny promotion according to the session policy; OpenTrack continues playback and reports denials separately. The Linux editor path requires X11 or XWayland; native Wayland-only VST3 embedding remains unsupported. On Windows, install Rust with the MSVC toolchain and the Visual Studio C++ build tools, then run this from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64
```

Launch `builds/x64/OpenTrack.exe`. Close any running OpenTrack instance before rebuilding. The execution-policy option applies only to this build process.

Add `-LocalAsio` to include the optional local ASIO adapter alongside WASAPI. Subsequent builds retain the previous backend selection for that architecture, recorded in `builds/x64/build-info.json`. Use `-LocalAsio:$false` to explicitly build with WASAPI only.

## Contributing

Contributions that make real musical workflows more reliable and useful are welcome. Useful work includes reproducing bugs, improving editing and audio behavior, reviewing accessibility and interaction, testing with audio and MIDI hardware, and refining the interface.

Keep changes focused, describe the expected behavior, and include meaningful verification. Distinguish working features from planned functionality so users and contributors can follow progress accurately.

## License

OpenTrack's original application and workspace source are licensed under Apache-2.0. See [LICENSE](LICENSE). Third-party dependencies, bundled artwork and the optional ASIO component retain their own licenses; see [third-party notices](docs/third-party-notices.md) and the notices beside bundled assets.
