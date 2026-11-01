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

### Still needed

- **Finishing and interchange:** audio mixdown/export, MIDI file import/export, and project interchange with other DAWs. CPR is currently a research and inspection effort, not an importer or round-trip format.
- **Mixing and recording:** automation playback and writing, audible send processing, general typed bus routing, cycle takes, and broader device channel mapping. Some related controls or layout elements are visible before their processing is complete.
- **Interface completion:** many menus, preference pages, dialogs and controls are disabled or partial. The ongoing [UI inventory](docs/reviews/ui-implementation-inventory.md) tracks source findings and outstanding review.
- **Reliability and validation:** recording, monitoring and low-latency behavior need further testing with real audio and MIDI hardware. Plug-in compatibility, project recovery, visual interaction and audible results need continued verification.

OpenTrack aims to bring recording, editing, arrangement, mixing and finishing into one dependable Rust application. Future interoperability and first-party instruments and effects will be developed as separately scoped capabilities.

## Build and run

The primary target is **64-bit Windows**. Install Rust with the MSVC toolchain and the Visual Studio C++ build tools, then run this from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64
```

Launch `builds/x64/OpenTrack.exe`. Close any running OpenTrack instance before rebuilding. The execution-policy option applies only to this build process.

Add `-LocalAsio` to include the optional local ASIO adapter alongside WASAPI. Subsequent builds retain the previous backend selection for that architecture, recorded in `builds/x64/build-info.json`. Use `-LocalAsio:$false` to explicitly build with WASAPI only.

## Contributing

Contributions that make real musical workflows more reliable and useful are welcome. Useful work includes reproducing bugs, improving editing and audio behavior, reviewing accessibility and interaction, testing with audio and MIDI hardware, and refining the interface.

Keep changes focused, describe the expected behavior, and include meaningful verification. Distinguish working features from planned functionality so users and contributors can follow progress accurately.
