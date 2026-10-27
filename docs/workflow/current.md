# Current assignment - T052

Updated: 2026-10-27. Implementer: GPT-6.1 Sol. User review pending.

## T052 - Mixer meters, fader rail and numeric gain

Mono Mixer and Channel Settings tracks now draw one full-width meter lane;
stereo tracks retain independent L/R lanes and their separator. The shared
fader has a dark inset slot with shorter travel capped at 240 logical pixels.
Its thumb and gain mapping retain the existing range and fine-drag behavior.
Double-clicking the current dB readout opens an inline editor: Enter/focus loss
commits, Escape cancels, invalid input leaves gain unchanged, and -inf mutes.
Gain commits use the existing Track::gain and project history/persistence path.

Packet: `docs/workflow/tasks/T052-mixer-meter-fader-editing.md`. Ten targeted
checks, formatting and ledger validation pass. The x64 local-ASIO build is
staged at `builds/x64/OpenTrack.exe`, SHA-256
`7E7D3BF349F418ECD9528734CAD0FC58714655FF66B7570AD8923EFD498983D9`.
Live visual/audio review is pending.

## Prior assignment - T051

Updated: 2026-10-27. Implementer: GPT-6.1 Sol. User visual review pending.

## T051 - Button shapes and conventional letter controls

The latest README screenshots are the reference for the user's requested
button-shape review. Toolbar, track/Inspector letters, Mixer, Channel Settings,
Transport, panel close controls and standard egui buttons now share compact
rounded corners and restrained edges. The old Mixer bevel stripe is removed;
letter labels use fitted Segoe UI Semibold and the circular e label is centered.
Hover, pressed, disabled and keyboard focus treatments are shared. Existing hit
rectangles, control order, conventional letters and command behavior are retained.

Packet: `docs/workflow/tasks/T051-button-shape-polish.md`. Compile, formatting,
and 11 focused existing checks pass. The x64 local-ASIO build is staged at
`builds/x64/OpenTrack.exe`, SHA-256
`A19AA8B5A92AF419B9369F7D1FBAD2E5FBB6539953702329A1AF9412DF33A506`.
Live visual acceptance remains pending.

## Prior assignment - T050

Updated: 2026-10-26. Actual implementer: Codex. Independent review: pending.

## T050 - Tabler icon system and OpenTrack branding

The user approved continuing the toolbar prototype across the rest of the UI,
asked for slightly stronger icon contrast, and requested removal of the logo
beside File. The user supplied the new OpenTrack circle-and-dots mark, a
transparent GitHub banner, and dark-background splash artwork. The app icon and
README use the new SVG mark; the menu row has no extra logo. Startup displays
the dark splash artwork briefly and allows click-to-dismiss. The GitHub banner
is in `docs/images/opentrack-github-banner.svg` for the repository social image.
The native window/taskbar icon is rasterized at 256px for cleaner Windows
downsampling and uses a tighter SVG viewBox to make the mark fill more of the
icon. The transparent GitHub wordmark is displayed at 440px in a dark README
band. The README screenshots now show the supplied Project/VST and Sample
Editor/Pitch & Time views.

Tabler v3.49.0 (commit bbed884) is pinned and only required verified SVGs are
vendored, with MIT notice. The central renderer and resvg cache cover toolbar,
track controls, Mixer, Channel Settings, Project inspector, Sample Editor,
Transport, and split/draw/zoom cursor overlays. Existing layout, labels, DAW
letter controls, shortcuts, actions, colors and states are retained. Enabled
toolbar icons use near-black ink against light button faces; outline width is 1.75.

Current report: `docs/reviews/tabler-application-icon-inventory.md`, with the
full source inventory, mapping and screenshot limitation. The enlarged state
specimen is `docs/reviews/tabler-toolbar-specimen.svg`. Source validation and
the staged x64 local-ASIO build pass. Current executable: `builds/x64/OpenTrack.exe`,
x64 with local ASIO, SHA-256
`CA89FF580C3B9C9F7B2E04DE0AA0A699D02C2240C5136738F72D42CBF57B793B`. Live
desktop screenshot and cursor hotspot review await the user's screenshot
because this session cannot capture the Windows desktop. Audio and editing
behavior are unchanged.

## Prior assignment - T049

Updated: 2026-10-26. Actual implementer: OpenCode session (DeepSeek V4.1 Flash); no independent review.

## T044 continuation - native VST editor thread and live processing display

The installed Waves editor was reproduced as a white/stalled window on the main
UI thread, with controls that did not move. The cause was the native view sharing
the application's eframe/winit message and paint cadence. Native VST editors now
run on a dedicated STA thread with their own message pump, focus handling and
repaint requests. The view is created from the live audio effect's controller
(`DetachedEditor`) rather than a second plug-in instance, so the CLA-76 VU/GR
meters and processing displays follow the actual audio callback. Parameter edits
still use the effect's lock-free host queue, and track-wide insert bypass still
applies.

A standalone CLA-76 probe with a continuous test tone showed the meter moving and
the controls responding. The installed mono/stereo audio checks pass, as do the
hosted-audio and dedicated-editor regressions. The user reported that the
controls work in the application; final in-app meter and save/reopen review
remains pending.

Logo follow-up: the user-supplied reel mark is now stored as
`docs/images/opentrack-logo-source.png`. White and black variants are used in the
application header and README, and a matching taskbar icon is generated from the
same artwork.

Final candidate: x64 local-ASIO `84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`.
The full app suite has 133 passes, 8 ignored opt-in tests and six pre-existing
pitch-editing failures owned by the separate pitch track. VST3, scanner,
plug-in-catalog, hosted-audio and installed CLA-76 checks pass. The app was not
launched by this session after the final build.

## T049 - Project scissors, Sample Editor guide, Mixer selection, and VST review

The Project's lower-waveform cut zone was removed; users cut only after choosing
the Scissors tool. The Sample Editor keeps the scissor cursor at the real pointer
and draws its snapped guide through the ruler and waveform or Pitch & Time grid.
Clicking anywhere on an ordinary Mixer track strip selects the shared Project
track. Native VST editor windows now run on a dedicated STA thread, use the live
audio instance's controller, and report a matching processing failure while
passing dry audio through.

Source inspection confirms the hosted VST insert path is included in each track's
playback plan and reaches the configured output route. Installed CLA-76 mono and
stereo processing checks produce finite output with the track path intact. The
reported Waves blank editor, stalled input and dead meter were addressed by the
dedicated editor thread and live-controller view; a continuous-tone probe showed
the meter moving and controls responding. Final in-app meter and save/reopen
review remain pending. The supplied reel mark is now the white/black application
and repository logo.

The x64 local-ASIO release build completed successfully (SHA-256:
`84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`). The app
was not launched by this session after that build. Formatting and whitespace
checks pass; VST3, scanner, catalog and hosted-audio checks pass. Six pitch tests
remain failing in the separate in-progress pitch track. Runtime Project, Mixer,
final Waves meter display, z-order, and audible routing acceptance remain
pending.

## Product direction — feature-led open-source DAW and project interchange

OpenTrack development is organized around user workflows and concrete DAW features, with the running application as the primary design surface. User-provided screenshots are optional inputs for specific tasks and are not archived by default. Project interoperability remains a product capability: implement format- and version-specific adapters, map supported features, preserve unknown project data as unchanged opaque stubs where possible, and report unsupported content.

## Product direction — effects and plug-ins (2026-10-26)

The user clarified that new effects should be VST plug-ins rather than built-in OpenTrack processors. The native OpenTrack Compressor was removed from insert pickers and audio processing. Existing projects preserve its serialized insert as an inactive legacy slot labeled unavailable; users can replace it with an external VST3 effect or remove it. External VST3 compressors are unaffected. Future first-party effects and instruments should be designed with the user and built as separately loadable VST3 plug-ins. This direction does not change the active Project/transport review gate.

## T048 - Transport viewport focus and pinning

The user requested an OpenTrack icon on VST Audio Channel Settings, transport shortcuts in focused child windows, and a right-click Always on Top toggle for the compact Transport that keeps it above Channel Settings. T048 moves Transport into a separate borderless native viewport, routes child-window keys through egui, sets the app icon on native windows, and stores the pin and bounds in app preferences. The x64 local-ASIO release build passed (SHA-256 `A91EA6C01C8652A0C9AA1987C9109CE8D1CA1AC4D55957DF6397890989327461`). Visual, focus, stacking, and restart review remain pending; see [T048](tasks/T048-transport-window-branding-focus-and-pinning.md).

T047's pitch edge handles and Project take lanes remain pending the separate user review documented in [T047](tasks/T047-pitch-edge-handles-and-take-lanes.md). T042 likewise remains awaiting the user's listening review as documented below. Neither review is resolved by T048.

## Current review - T042 Batch 2

Batch 1 T041 was accepted by the user. After listening to the first T042 iteration, the user asked for pitch targets to blend toward adjacent notes inside the notes themselves, with a maximum reach of one sixteenth note, and reported bubbly/choppy sound when shortening notes.

The renderer now shares an event-wide source-pulse map and continuous output clock across note cuts. It reacquires real waveform peaks at voiced onsets, adapts overlap windows to source harmonic content and time expansion/compression, and uses the same tempo-aware pitch contour in playback and the Sample Editor. Edited and adjacent unedited notes ease toward a shared boundary pitch for up to one sixteenth-note on each side.

The 2026-10-25 audit found a hard wet-to-dry switch at the inner endpoint of an unedited neighbor's rendered edge. The fix adds a 5 ms endpoint crossfade, shortens the audio handoff to 5 ms while keeping the long pitch ramps, and makes one-unchanged-note pairs resolve to the unchanged edge pitch. Sample-continuity tests cover both orientations. Headless synthetic and read-only Vocal 01 measurements now pass; user listening is still pending.

This checkpoint also adds semitone-row note colors, track-color cohesion for meters/EQ/spectrum with blue default Stereo In/Out, and a dedicated note-drag audition voice that works while the transport is stopped without moving the playhead.

## Build and checks

- x64 release build with local ASIO succeeded; WASAPI remains available. Batch 3, VST3 and the native-editor thread work are included.
- Executable SHA-256: `84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`.
- App suite: 133 passed, 8 ignored opt-in tests, six pre-existing pitch-editing failures. VST3 suite, 6 hosted-audio checks, scanner/catalog tests and installed CLA-76 mono/stereo audio checks pass.
- Native CLA-76 editor test passes on the dedicated STA thread. A continuous-tone probe showed the meter moving and controls responding.
- Headless production-renderer suite: 6 passed; 1 explicit-source test ignored in the ordinary run. Read-only Vocal 01 matrix passed separately. Details and artifacts are in T042.
- OpenTrack was checked closed before the final build and was not launched afterward. Final desktop, device and user-listening review remain pending.

## User review checkpoint - pending

The user will review pitch quality with their own audio. The current build also includes note colors, track-color cohesion (including blue default Stereo In/Out and the Mixer color picker), note-drag audition, and Batch 3.

Batch 3 implementation is complete in source: toolbar Scissors cuts, snap-aware guides, Ctrl+wheel horizontal pan, rectangular Mixer fader caps, and shared Mixer/Project track selection. T049 removes the selected-track cut band, extends the Sample Editor cut guide through its edit area, and makes ordinary Mixer track-strip clicks select the same Project track. The x64 build succeeded; live interaction, Waves graphics, audio routing, native pin behavior, and visual review remain pending.

## VST3 lane — native editor and effect compatibility corrections

2026-10-26 user follow-up rejects generic sliders as the default plugin UI and
reports Waves insert crashes. Insert add/click now opens the plugin's native
editor. Fixed Waves mono bus coercion, factory release after module unload on
initialization failure, native editor close/replacement lifetime, frame sizing,
and iZotope's no-op processing notification. Native editor edits mark projects
modified. Fresh installed-folder sweep: 25/25 effects pass native editor lifecycle,
overlapping instances, processing and state restore, including 19/19 Waves
classes. Retrologue, Superior Drummer 3 and sforzando remain unsupported instruments;
this is not universal hosting or production sandboxing.

The later dedicated-thread editor work uses the live audio effect's controller so
controls and VU/GR meters follow processing. App suite: 133 passed, 8 opt-in
probes ignored, six pre-existing pitch failures. VST3/catalog/scanner and six
hosted-audio regressions pass. x64 local-ASIO release SHA-256:
`84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`. Final in-app
meter and release end-to-end acceptance remain pending. See
[T044](tasks/T044-vst3-native-editors-and-compatibility.md) for reproduction,
per-class sweep command, evidence and remaining limits. T043 remains historical.

## Cross-platform VST3 foundation — T045/T046

The scanner and insert picker share native VST3 bundle resolution for Windows,
macOS and Linux layouts. Scanner defaults include standard macOS/Linux user and
system folders, and the architecture precheck recognizes PE, ELF and Mach-O
images, including universal Mach-O slices. T046 adds required Linux
`ModuleEntry`/`ModuleExit` and macOS `bundleEntry`/`bundleExit` loading, explicit
Linux X11/Wayland and portal dependencies, and a hosted compile matrix for
Windows x64, Linux x64/ARM64 and macOS x64/ARM64. The matrix is prepared but has
not run. Locally, Windows x64 workspace all-target compilation passes, and the
scanner/catalog/VST3 test code compiles for Windows x86; no test binaries were
run. This workspace has no macOS/Linux Rust targets installed, so their code
still needs the hosted compile check. Native editor parent surfaces and event
loops, audio/MIDI device paths, and plug-in runtime validation remain open. See
[T045](tasks/T045-cross-platform-vst3-discovery.md) and
[T046](tasks/T046-cross-platform-vst3-runtime-foundation.md).

## Preserved direction

Keep pitch analysis/editing/processing in Rust, preserve original audio and the accepted Mixer, and maintain the gated sequence: Batch 1 accepted; Batch 2 listening review now; Batch 3 Project scissors/wheel behavior and Mixer fader/selection only after Batch 2 acceptance. Scope remains `transport-v16-pitch-time-editing-workflows`; implementation, feature verification, and user acceptance remain separate statuses. See [the historical feature report](metrics.md) and [T042](tasks/T042-vocal-quality-and-note-transitions.md).
