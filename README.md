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

## What we are building

- **A complete creative workflow:** audio and MIDI recording, timeline editing, arrangement, mixing, and export in one application.
- **Responsive, predictable editing:** accurate transport controls, grid snapping, loop ranges, smooth navigation, and dependable undo and redo.
- **A capable mixing environment:** track routing, monitoring, automation, channel processing, and support for third-party instruments and effects.
- **Projects you can trust:** reliable saving, media management, recovery, and consistent recall of your work and workspace.
- **A flexible desktop workspace:** clear controls, useful shortcuts, configurable layouts, and a polished interface that keeps everyday actions close at hand.
- **Interoperable projects:** import and export supported DAW project formats while retaining unsupported metadata where the format allows.
- **An extensible foundation:** a Rust application and audio engine that can grow through community contributions and independently designed instruments and effects.

Development prioritizes complete, usable musical workflows. Functionality, reliability, and interface polish are reviewed together as the application grows.

## Development status

OpenTrack is an early prototype under active development. The current application includes a Project timeline with audio and MIDI editing, a Mixer with shared channel strips, Channel Settings with a 32-band EQ (static and dynamic bands), live spectrum and tonal-reference analysis, an insert rack with built-in processors and third-party VST3 hosting, native plug-in editor windows, a MIDI editor with part and controller lanes, a Sample Editor with on-demand Pitch & Time analysis, a MediaBay, a floating transport, bounded project undo/redo, editable key commands and workspace presets, and native project saving.

These pieces are still being integrated and reviewed. Third-party VST3 hosting has documented compatibility limits and is not universal support. Automation, typed buses and routing, Send DSP, cycle takes and CPR project interchange remain development goals. Visible controls may still be incomplete. Recording, monitoring and low-latency behavior still require validation with real audio hardware.

New projects use the `.opr` format. Earlier `.otp` and `.rwp` project files remain readable.

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
