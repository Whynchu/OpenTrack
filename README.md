<table align="center" width="100%" bgcolor="#292929">
  <tr><td align="center">
    <img src="docs/images/opentrack-github-banner.svg" alt="OpenTrack" width="440">
  </td></tr>
</table>

# OpenTrack

OpenTrack is a standalone open source DAW built in Rust. Its goal is to provide a dependable, expressive environment for recording, editing, arranging, mixing, and finishing music.

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

OpenTrack is an early prototype under active development. Its features are still being integrated and reviewed, and some visible controls are incomplete. The following describes the current development version; availability does not imply that every workflow has completed runtime or hardware validation.

### Available today

- **Projects and arrangement:** a timeline for audio and MIDI tracks, transport and cycle playback, grid snapping, editing, and bounded undo/redo. New projects use the `.opr` format; earlier `.otp` and `.rwp` projects remain readable.
- **Recording and media:** WAV import and playback, audio and MIDI recording, basic track input/output selection and software monitoring, managed project folders, media relinking, and a MediaBay.
- **Mixing:** shared Mixer and Channel Settings controls, input gain and meters, a 32-band EQ with static and dynamic bands, spectrum and tonal-reference analysis, and an insert rack.
- **Plug-ins:** third-party VST3 effect hosting and native editor windows, alongside existing built-in processors. Compatibility varies by plug-in; universal VST3 support is not claimed.
- **Editors and workspace:** a MIDI editor with notes and controller lanes, a Sample Editor with on-demand Pitch & Time analysis and editing, a floating transport, editable key commands, and workspace presets.
- **Audio and MIDI capture:** record audio and MIDI input into a project; real-device behavior still needs broad hardware validation.

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

- **Finishing and interchange:** initial stereo WAV mixdown and Standard MIDI File note/controller exchange are implemented in source. Authored format-0/1 fixtures and independent export decoding pass; in-app review and comparison with DAW-produced MIDI files remain. Project interchange with other DAWs is still future work. CPR is currently a research and inspection effort, not an importer or round-trip format.
- **Mixing and recording:** automation playback and writing, device-bus sends, general typed bus routing, cycle takes, and broader device channel mapping. Track and group send processing is implemented and covered by offline tests; live listening and real-time performance still need review.
- **Platform and feature breadth:** Windows x64 is the primary target. Kubuntu Linux was part of development and has been built and verified; other Linux distributions, macOS and browser releases are not claimed. Video, surround and score editing are not available.
- **Command interfaces:** there is no supported command-line or MCP interface for controlling project edits.
- **Interface completion:** many menus, preference pages, dialogs and controls are disabled or partial. The ongoing [UI inventory](docs/reviews/ui-implementation-inventory.md) tracks source findings and outstanding review.
- **Reliability and validation:** recording, monitoring and low-latency behavior need further testing with real audio and MIDI hardware. Plug-in compatibility, project recovery, visual interaction and audible results need continued verification.

OpenTrack aims to bring recording, editing, arrangement, mixing and finishing into one dependable Rust application. Future interoperability and first-party instruments and effects will be developed as separately scoped capabilities.

### Near-term gaps

Next work prioritizes in-app validation of audio mixdown, MIDI file exchange, and track/group sends, followed by automation, broader audio-device routing, and recording reliability. Send processing passes offline checks; live listening and device-bus sends remain open. Windows x64 remains the primary target; Kubuntu Linux was part of development and has also been verified.

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
