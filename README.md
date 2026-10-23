# OpenTrack-

OpenTrack- is a standalone, full Rust reimplementation of Cubase 5. The goal includes the application workflows and first-party Rust rewrites of its bundled instruments and effects. REA is the reverse-engineering tool used to observe the reference application and its files; OpenTrack- is the product. All implementation code and visual assets are original.

## Product goals

- Recreate Cubase 5 workflows, commands, project behavior, audio/MIDI functions, and window/workspace behavior as a native 64-bit Windows application.
- Rewrite the bundled Cubase 5 plug-ins in Rust and integrate them into OpenTrack-'s audio engine.
- Use REA and the local Operation Manual to gather evidence. Keep observations, inferences, and unknowns distinct.
- Investigate native Cubase project import and round-trip behavior. Do not claim compatibility before real project files and round trips have been verified.
- Keep first-party application, audio, compatibility, and plug-in code in Rust. Document any OS or external plug-in ABI boundaries.
- Use original code and fresh visual assets; reference the installed application without copying its program code or assets.

## Compatibility boundaries

OpenTrack- is a standalone application, not a plug-in or extension that runs inside Cubase. The bundled Cubase 5 plug-ins are rewrite targets: the original binaries are evidence, not substitutes to ship or load in the x64 application. Support for unrelated legacy 32-bit plug-ins is a separate compatibility question.

OpenTrack- currently saves its own versioned RWP projects. That format is not Cubase-compatible. Native CPR import and verified round-trip behavior remain investigation goals; the current prototype has no CPR importer.

## Repository layout

- crates/workstation-core: shared command, keymap, workspace, project, and CPR-inspection models.
- crates/workstation-audio: WAV decode/capture, audio-device I/O, channel EQ, and insert DSP.
- crates/cpr-inspect: read-only utility for CPR header and string reconnaissance.
- crates/workstation-app: OpenTrack- Windows desktop shell and application integration.
- docs: architecture, compatibility evidence, audio engine, plug-in rewrite inventory, native format, and reference command data.

The Cubase reference install and REA analysis tools remain outside this project directory. The saved bounded inventory is at ../_analysis-tools/reports/cubase-install.evidence.json.

## Current implementation

The prototype has an original vector-drawn arrange view, track creation and selection, a separate Mixer window with live track meters, WAV clip import/playback, and separate MediaBay and MIDI Editor windows. Arrange headers provide Mute, Solo, Solo Defeat, Record Enable, and Channel Settings controls. Ctrl-clicking Solo isolates a channel; Alt-clicking toggles Solo Defeat, which keeps it audible when other tracks are soloed. The Track menu adds Audio, Instrument, and MIDI tracks, renames the selected track, moves it up or down, and confirms removal; double-clicking a track name also opens rename. Channel Settings follows the selected track and exposes its instrument choice, four-band EQ, and ordered insert chain, including add, bypass, reorder, remove, and parameter controls. Window open state and geometry are saved with each project workspace; global workspace presets capture the main window only. These layouts are a first implementation pass and have not yet been measured against Cubase screenshots.

Arrange tracks can be resized individually by dragging a row's lower edge, or resized together by holding Ctrl during the drag. Controls remain available in compact rows. Alt+Down/Alt+Up and Ctrl+Up zoom track heights; Z/Ctrl+Down toggles the selected track's exclusive view. Track heights are stored per workspace with the project. Ctrl-click Solo isolates that channel; Alt-click Solo toggles Solo Defeat. Zoom sizing and exclusive behavior still need comparison against Cubase 5.

The app can record a selected mono/stereo input to synchronized WAV takes on every armed audio track using one shared input stream, show the live input peak while recording, optionally monitor that input through a bounded lock-free ring, stream sources too large for in-memory decoding, relink unavailable WAV sources from the File menu, and provide per-track four-band EQ, Stereo Delay, Compressor, and TestGenerator inserts. The Compressor is an OpenTrack- original stereo-linked peak design; Cubase parity has not been measured. MIDI and Instrument tracks support persisted MIDI parts, beat-based note and controller events with per-event MIDI channels, piano-roll editing, and four controller lanes. On Windows, WinMM MIDI input can record to multiple armed MIDI/Instrument tracks while all armed audio tracks record from the same selected input; MIDI Thru plays the starter oscillator through the default audio output. Instrument tracks can use a built-in sine, square, sawtooth, or triangle oscillator through the Rust instrument interface. The transport has editable beat-based cycle locators and loops playback at the right locator. These voices are OpenTrack- starter instruments, not Cubase rewrites. Project edits support bounded undo/redo with Ctrl+Z and Ctrl+Shift+Z (Ctrl+Y is an additional redo fallback). The keymap loads 152 command definitions transcribed from the reference Windows manual. The Key Commands window supports shortcut reassignment, conflict confirmation, and restoring defaults; assignments persist in application settings. Implemented shortcuts cover file open/save, transport and panel controls, track mute/solo/Solo Defeat/arm, tool selection, workspace keypad controls, Undo/Redo, and MIDI editor open/close. The Workspace Organizer saves the active layout as a global or project preset, copies global presets into a project, and creates workspaces from either preset list. It saves version 14 RWP projects, migrates versions 1 through 13, and stores project workspaces, project preset collections, and auto-instantiated global preset origins.

The CPR utility validates the observed outer header, reports ROOT/ARCH block framing, and decodes the observed length-prefixed ROOT labels. All 37 installed templates have been scanned; their ARCH payload records and semantics remain unmapped. TestGenerator is integrated as the first documented Cubase effect in the insert chain; its original Rust DSP and noise modes have not been compared with the Cubase output.

## Remaining work

This is an early prototype, not feature-complete Cubase 5 parity. Audio recording currently uses the selected mono/stereo input and fixed 32-bit float WAV; per-track audio input/output bus selection, distinct source routing, multichannel capture, cycle takes, external MIDI output, automation, a general plug-in ABI, CPR import/round trips, and parity-verified Cubase plug-in rewrites remain future work. MIDI input currently uses the Windows WinMM API and has not been checked with a physical device; input monitoring still needs latency and dropout acceptance on the target PC. Many catalogued Cubase commands also have no application action yet. The methodical feature sequence is in [the implementation roadmap](docs/implementation-roadmap.md); track and window evidence limits are in [the window and track fidelity notes](docs/window-and-track-fidelity.md), and recording and EQ evidence limits are in [the audio reference notes](docs/cubase5-audio-reference.md).

Build the Windows x64 application from this directory with:

    powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1

The current application is always `builds/x64/OpenTrack.exe`. Add `-Architecture all` to also refresh the experimental `builds/x86/OpenTrack.exe`. The execution-policy option applies only to this build-script process. Builds overwrite these stable paths; Cargo's intermediate files live in the ignored hidden `.build-cache` directory, also configured as Cargo's default target directory. All old review, verification, debug and baseline build folders have been removed.

OpenTrack follows the Windows app light/dark preference, including changes while running. Native window decorations follow the system, and UI text uses the installed Segoe UI font with an embedded fallback. Menus and custom track/editor surfaces use the active palette. The application's controls are drawn by egui; they are not native Win32 widgets.
