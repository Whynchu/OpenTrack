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

### Available today

- **Projects and arrangement:** a timeline for audio and MIDI tracks with transport and cycle playback, grid snapping (including snap to event edges and the cursor), project markers with next/previous navigation and a marker list, zoom commands, follow-playhead scrolling, event nudging, split-at-cursor, split loop, glue, range deletion, crossfades, MIDI part split/merge, and a bounded undo/redo with an undo-history window. New saves use native format version 27; earlier `.opr`, `.otp` and `.rwp` projects remain readable, and older projects load with safe defaults for newer fields. Older OpenTrack builds through version 26 cannot reopen a version 27 save.
- **Recording and media:** WAV import and playback, audio and MIDI recording, punch in/out driven by the locators, per-track input monitoring (Off / Auto while armed / On), loop recording that puts each cycle pass on its own take lane, a metronome with a per-beat accent pattern and a count-in, managed project folders, media consolidation into the project, missing-media relink, media-pool cleanup, multiple recovery points, and a MediaBay.
- **Mixing:** shared Mixer and Channel Settings controls, input gain and meters, a 32-band EQ with static and dynamic bands, spectrum and tonal-reference analysis, and an insert rack. Track-volume automation records from live control movement, plays back, and supports point editing; arrange context menus can reveal tracks with stored curves.
- **Plug-ins:** third-party VST3 effect hosting and native editor windows, alongside existing built-in processors. Compatibility varies by plug-in; universal VST3 support is not claimed.
- **Editors and workspace:** a MIDI editor with notes and controller lanes plus quantize (strength and swing) and transpose/velocity tools, a Sample Editor with on-demand Pitch & Time analysis and editing, a floating transport, editable key commands, and workspace presets.
- **Finishing:** stereo WAV mixdown, per-track stem export, bounce in place through a track's own chain, and Standard MIDI File export of the whole project or a selection.
- **Project tools:** project templates, a session health check, project statistics, a beat calculator, and a project notepad saved with the file.
- **Platform and diagnostics:** a callback-budget readout with a worst-block figure, project load measurement, and Windows job-object containment for scanner workers (implemented and compiled for Windows; runtime validation on Windows is pending).

### Highlights

- **32-band EQ with dynamic processing:** shape a channel with multiple filter types and independent dynamic controls for each band.
- **Built-in spectrum and tonal-reference analysis:** compare live audio with another channel or a captured or imported WAV reference.
- **Up to 24 insert and send slots per channel:** insert slots run available built-in processors and third-party VST3 effects, including native plug-in editor windows. Track and group sends have destination, level, pan, and pre/post-fader controls; the signal path passes offline tests and still needs live hardware review. Device-bus sends remain unavailable.
- **Integrated Pitch & Time:** analyze and edit audio in the Sample Editor, alongside the Project and Mixer workspace.
- **Windows audio choices:** use WASAPI, or enable the optional local ASIO adapter for a compatible installed driver.
- **Platform:** Windows x64 is the primary build target. Kubuntu Linux was part of development and has been built and verified.

### Built-in EQ and analysis

Channel Settings includes an EQ and analyzer for shaping and inspecting a channel:

- **Up to 32 EQ bands:** add and adjust bands on the graph, with frequency, gain and Q controls, individual band enable and solo, and an overall EQ bypass.
- **Filter choices:** constant-Q and proportional-Q bell filters, low- and high-pass filters, and several low- and high-shelf shapes.
- **Dynamic EQ:** independently enable dynamic processing on a band and adjust threshold, range, attack and release. The graph shows the allowed range and live gain movement.
- **Live spectrum:** view pre- or post-processing signal, compare another channel, and adjust the display range, tilt and FFT resolution. Peak hold and freeze help inspect the display.
- **Tonal reference:** capture a target from playback or import a WAV reference, then compare a longer-term tonal profile against the saved target. Targets are stored with the project.

These features are under active development; live listening, visual behavior and performance across audio devices still need further review.

### Still needed

- **Runtime and listening review:** recording, punch, monitoring, loop takes, the metronome click, stem export, bounce and mixdown need review with real audio and MIDI hardware. Offline and headless checks pass; device behavior and audible results are not accepted yet.
- **Finishing and interchange:** authored format-0/1 MIDI fixtures and independent export decoding pass; in-app review and comparison with DAW-produced MIDI files remain. Project interchange with other DAWs is still future work. CPR is currently a research and inspection effort, not an importer or round-trip format.
- **Mixing and routing:** device-bus sends and wider physical channel maps remain open. Track and group sends, typed routing and automation playback pass offline checks; live listening and real-time performance still need review.
- **Keyboard and interface completion:** 122 declared menu entries are still shown disabled, and 117 of the 152 keymap-bound commands have a working shortcut. The [accessibility and keyboard report](docs/workflow/accessibility-2026-11-02.md) lists the remaining shortcuts, the contrast measurements and the focus/label behavior. A manual keyboard-only pass is still pending.
- **Platform and feature breadth:** Windows x64 is the primary target. Kubuntu Linux was part of development and has been built and verified; Linux VST3 editor embedding is not implemented, and other Linux distributions, macOS and browser releases are not claimed. Video, surround and score editing are not available.
- **Command interfaces:** there is no supported command-line or MCP interface for controlling project edits.
- **Reliability and validation:** plug-in compatibility, project recovery, visual interaction and audible results need continued verification on real hardware.

OpenTrack aims to bring recording, editing, arrangement, mixing and finishing into one dependable Rust application. Future interoperability and first-party instruments and effects will be developed as separately scoped capabilities.

### Near-term gaps

Next work is user-side review of the recording, delivery and listening features above, followed by the CPR adapter once real `.cpr` fixtures are available, the remaining disabled menus and undispatched shortcuts, and a Linux VST3 editor path with a test plug-in. Windows x64 remains the primary target; Kubuntu Linux was part of development and has also been verified.

Progress and verification are tracked in [the current workflow state](docs/workflow/current.md), the [revision-2](docs/workflow/roadmap-32-steps-v2.md) and [revision-3](docs/workflow/roadmap-32-steps-v3.md) roadmaps, and the [accessibility report](docs/workflow/accessibility-2026-11-02.md).

## Build and run

The primary build target is **64-bit Windows**. Kubuntu Linux was part of development and has also been built and verified; other Linux distributions, macOS and browser builds are not current release claims. On Windows, install Rust with the MSVC toolchain and the Visual Studio C++ build tools, then run this from the repository root:

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
