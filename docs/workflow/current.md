# Current assignment - T059

## Active goal T059 - editing, playback stability, menus, and loading polish

Work through these areas in small, independently reviewable steps:

1. MIDI editing: add the upper-left-pointing pencil cursor and Draw/Select
   tools; right-click opens the tool picker only over the note grid; drag a
   marquee to select groups of notes and move selected notes together. Then
   review runtime behavior and add any needed delete/copy/group-edit actions.
2. Audio stability: reproduce and measure playback pops during playback alone,
   track dB/fader adjustments, and MIDI controller changes; identify whether
   the glitches share a real-time audio cause before changing the engine.
3. Popup behavior: make outside-click dismissal consistent for menus and
   popups. The user chose outside-click only; no inactivity timeout.
4. Loading: report real project-load phases and investigate stalls from audio
   source and plugin initialization without regressing the existing loader.
5. First-render flashes: trace the white-box flash separately for app-owned
   windows and plugin views, then fix initialization/render readiness.

Current checkpoint: MIDI Draw/Select/Range controls, grid-only context menu,
marquee selection, and group move are implemented in source. Project inspector
gain/pan now update live, avoiding full playback-plan rebuilds for those
controls. Equalizer revisions now advance only when EQ values change, avoiding
redundant filter reconfiguration during fader drags. Selected-note velocity
changes now update atomic per-note targets and are smoothed over 5 ms for
already sounding notes, avoiding a full playback-plan rebuild on each slider
drag. A focused audio unit test passes. MIDI note structure and controller
drag gestures now defer plan reconstruction until pointer release, avoiding a
full rebuild on every changed frame. Main app utility/configuration windows now
close on outside clicks, and a focused egui interaction test covers the shared
close behavior. More specialized windows still need the same rule. Menus and
context popups already close on outside click. Popup preference is outside
click only, with no idle timeout. Playback impact has not been measured on the
user's audio setup. `cargo check --offline -p opentrack --features local-asio`,
the selected-velocity audio test, and the focused popup test pass. A
workspace-wide `--no-run` remains blocked by three standalone VST probe examples
using older window API signatures. Runtime visual/audio review remains pending
under the user's no-computer-use instruction.

Project loading reports project-read, source-discovery, per-source opening, and
plugin initialization stages; it initializes one VST3 insert per UI frame and
shows the plugin name/count. A slow individual plugin can still block its frame,
and final workspace/audio-plan setup remains on the UI thread. The dedicated
VST3 editor host stays hidden until its view is attached and saved bounds are
applied; first-paint behavior and app-owned window flashes need runtime review.

Project panel follow-up: inspector M/S buttons display inherited group state,
group headers reflect soloed descendants, and the inspector Monitor button now
toggles audio-track monitoring only when an input route is available. Two
unreferenced legacy RGBA logo assets were removed. Local-ASIO compilation
passes; runtime display/monitor review remains pending.

## Previous task T057 - recovery after accidental app.rs reset

## 2026-10-28 recovery after accidental app.rs reset

Actual model: GPT-6.1 Sol. Recovered Project/MIDI view functions from the
earlier session's recorded patches and shell edits, replayed in an isolated
recovery copy. Preserved the current project loader that the user confirmed
works. Restored Project sidebar/scrollbar placement, record-readout/time-zoom
lane, MIDI ruler/overview navigation, note previews, neutral gray controls,
and the controller divider. The current source and patch checkpoints are in
`recovery-20261028-204244/`; do not reset the working tree to recover a single
change.

The group waveform was suppressed by unrelated track solo or inherited mute.
Editing previews now retain child envelopes regardless of playback mute/solo;
the composite remains restricted to collapsed groups (children hidden).
The expression lane shares its actual resized height with the allocated
note/controller stack and reserves horizontal-scrollbar space. The recovered
note grid has its own click/drag response. Piano keys are painted in the fixed
gutter and note hit/draw regions exclude that gutter.

Focused tests pass: group envelope remains nonzero and unchanged with an
unrelated MIDI solo plus group mute; expression header is visibly inside the
viewport and divider size survives drag release. Existing test-only call
signatures were repaired to compile these checks. `cargo check --offline -p
opentrack --features local-asio` and whitespace validation pass. An ASIO-enabled
x64 package passed with `local_asio: true`, SHA-256
`8EAD799A049B2A305756CA5AE464525B5B155F8387DFC895F42DB1E90BE90245`.
The broader ruler suite reports 8 passed, 8 failed, 1 ignored, including stale
198 px header assumptions versus the current 264 px header and other fixed
pointer coordinates; it is not claimed passing. Desktop review remains pending under the user's
no-computer-use instruction; compilation/tests do not establish visual parity.

## T058 — Realtime performance and stability program (2026-10-28)

The user asked how to extract maximum performance, stability and quality from the
engine. A source audit found that playback cost scales with the *total* clip
count on a track rather than with audible clips, that the release build has no
LTO/codegen tuning and no FMA, that a late callback causes a larger render burst,
and that there is no render-side dropout counter and the callback has no
NaN/denormal guard and no fault containment. See
`docs/workflow/tasks/T058-realtime-performance-and-stability-program.md`.
The packet is phased: measurement, build/fill containment, block-first rendering,
renderer unification, fault containment, then quality. Status: ready for
architecture review (Astra/Sol); no implementation started, and no existing
engine behavior is changed by writing the packet.

### 2026-10-28 cross-editor navigation and scrollbar pass

- Project plain wheel input over the track-list/header scrolls tracks
  vertically; plain wheel over the timeline zooms time, and the modifier paths
  continue to map to horizontal pan and track-height scroll/zoom.
- The Project scroll area now ends beside a dedicated right-edge track-height
  zoom rail, so it no longer covers the vertical track scrollbar. Its bottom
  horizontal scrollbar ends before the time-zoom slider, leaving both controls
  side by side on the same lane.
- MIDI and Audio Editor now apply the same solid scrollbar style as Project.
  MIDI native wheel scrolling is disabled so shared modifier-aware wheel
  actions do not also move the horizontal ScrollArea. Audio overview viewport
  can be dragged directly to pan, matching MIDI overview behavior.
- MIDI viewport rail and zoom control extend alongside the controller lane;
  velocity dark fill applies to the slider track itself. MIDI ruler zoom/pan
  and part viewport updates from earlier passes remain in the working tree.
- Verification: `cargo fmt --all`, `git diff --check`, and `cargo check
  --offline -p opentrack` passed. `build.ps1 -Architecture x64` passed with
  local ASIO enabled. Packaged executable SHA-256:
  `A9E8DBAABCE422DF74567832D60CB2D4EEC5C22D43076AF4FE9A2A1455BB7982`.
- Runtime screenshot review confirmed the two Project zoom/scroll lanes do not
  overlap. With track-height zoom temporarily raised to create overflow, plain
  wheel over the left track headers moved the visible track rows while ruler
  positions stayed fixed; plain wheel over the timeline changed horizontal
  scale while track heights stayed fixed. Both view adjustments were restored.
  Audio overview dragging shifted the visible ruler range. MIDI overview-frame
  dragging shifted the visible bar range, and MIDI wheel zoom changed bar
  spacing without changing pitch rows; Fit Part restored the view. The
  modifier-wheel paths were source reviewed but not directly injected in this
  runtime pass. The reviewed selection was saved; OpenTrack remained open at
  that review checkpoint. The later divider/rail package and its current
  closed-app state are recorded below. Full T057 visual parity and user
  acceptance remain pending.

### 2026-10-28 MIDI divider and navigation rail follow-up

- The MIDI expression lane now has a press-origin resize gesture over its top
  14 px strip. Dragging that divider downward reduces the expression pane;
  resizing is gated away from controller-point edits. The lane is clamped to
  96–480 px with a 126 px default.
- Slider selection fills and custom zoom/pitch navigation thumbs use neutral
  gray. Shared solid scrollbars are 16 px wide, and the Project right-side
  track-scroll/zoom lane is 20 px wide with a 94 px zoom control, matching the
  Audio/MIDI editor rail size and placement.
- Formatting, `git diff --check`, `cargo check --offline -p opentrack
  --features local-asio`, and `cargo build --offline --release -p opentrack
  --features local-asio` passed. The packaged x64 build also passed via
  `build.ps1 -Architecture x64`; metadata confirms `local_asio: true` and the
  executable SHA-256 is
  `30247E9AC3766CB67269409CC90920DDD6899B8E1DC4F1A1FFB503D66CED1BB9`.
- The user requested no further computer use, so the current build was not
  opened for runtime interaction or screenshot review. The earlier response-
  based divider drag did not resize; this press-origin rework and the revised
  slider/scrollbar colors and widths remain runtime-unverified. T057 remains in
  rework pending that check and the user's broader visual review.

## T057 — MIDI editor framing, note preview and arrangement parity (2026-10-27)

The user asked for the MIDI Editor to use the Audio Editor's overall UI
structure while retaining the piano-roll look, with shorter default notes and a
snapped ghost note on hover that is committed by click. Notes should use the
track highlight color. MIDI parts in Project should share audio-event selection
visuals and applicable behavior, without audio gain/fade controls. See
`docs/workflow/tasks/T057-midi-editor-and-arrangement-parity.md`.

### 2026-10-28 note-preview readability follow-up

Project MIDI note previews now preserve their scaled beat duration instead of
forcing short notes to overlap at arrangement zoom. Both mini previews scale
pitch positions to the notes' actual pitch range. Built x64 with local ASIO
enabled, SHA-256
`1C9199C91C892C9A2E3031652D7CC94A15843989EE56BBF7CA69CC721A15239F`.
Reopened the saved four-note `EXAMPLE.opr` in the rebuilt app. The Transport
Panel was closed for the review capture; four note blocks remain visible in
the arrangement clip and piano roll. T057 visual parity and user acceptance
remain pending. The app is left open for user review.

### 2026-10-28 overview navigator follow-up

The user identified that the MIDI visible-range box crossed the inspector's
track header and did not have a clear overview role. The MIDI overview now has
a dedicated labeled lane: its note map and interactive viewport frame start at
the piano-roll beat-grid edge, leaving the inspector and keyboard gutter
outside the navigator. `cargo fmt --all --check` and the x64 release build
passed with local ASIO enabled. Packaged executable SHA-256:
`7B2D90C27B3FBB66F81BCDC5787906909E16ABD6536F1E55C9D01C5C77AD9C8B`.
A fresh `EXAMPLE.opr` screenshot was captured at 1920×1033. The viewport frame
no longer crosses the inspector header; in this saved view it spans nearly the
full overview width because the piano roll is fit to the part. OpenTrack was
closed after capture. This targeted rework does not complete T057 parity or
user acceptance.

### 2026-10-28 MIDI map and ruler follow-up

The user clarified that the overview must show the whole MIDI part at a glance,
with a viewport frame for the currently visible piano-roll section, and a
separate ruler below it. The viewport now maps visible piano-roll beats to the
full part overview; clicking jumps and dragging the frame pans. The piano-roll
ruler uses the transport's musical position formatting, seeks on click, and
draws the shared transport playhead. Project arrangement note previews now use
part-local note positions offset from the part's project start and clip to the
part bounds. Formatting, `cargo check --offline -p opentrack`, and the x64
release build passed (local ASIO enabled); executable SHA-256:
`61C5D7F80A521B1BB0D4E223439383318C3FFDD502DD2A64A7EE9C5A274D0D9F`. Review
then exposed that empty-cell clicks only cleared selection; note creation was
drag-only. Empty-cell clicks now add a snapped note at the saved insert length,
while click-drag still sets its duration. Formatting, `cargo check --offline -p
opentrack`, and x64 release build passed again with local ASIO; current
executable SHA-256: `F5A6B4DBE4B28820141B5F828A835179975909029045BFDC0DB2ADC8344038FB`.
In the rebuilt app, click-created four notes and confirmed the Notes field
incremented to four; saved `EXAMPLE.opr` and closed OpenTrack. The four-note
MIDI Editor screenshot was captured, but Project-window note preview at this
zoom remains too small to confirm visually. T057 remains in rework pending
timeline comparison and user review.

### 2026-10-28 readability follow-up

The user identified weak semitone borders and a controller title colliding
with the piano keyboard. The shared piano renderer now draws pixel-aligned
one-physical-pixel separators after the key fills, across both the keyboard and
pitch rows. Key borders use stronger contrast. The controller title now sits
in a raised strip above its plot, to the right of the keyboard. Formatting and
the x64 build passed with local ASIO enabled; a fresh runtime screenshot was
captured. This targeted polish does not complete the full T057 parity review;
user acceptance remains pending. OpenTrack was closed after review.

The 2026-10-28 side-by-side screenshots reject the prior claim of visual parity.
The MIDI editor still differs substantially in toolbar and field presentation,
inspector/header styling, keyboard rendering and note naming, and pitch
navigation. The expression/controller lane is below and aligned with the note
grid. MIDI note drawing inside the MIDI Editor does not require
Pencil: primary left-click on an empty cell creates a note and primary drag can
create its duration. Other mouse buttons must not create or edit notes, and
middle-button drag pans only. Pencil remains required for drawing MIDI parts in
the Project arrangement. T057 is in rework, not awaiting acceptance. The only
intended structural differences from Audio Editor are no audio waveform and the
expression/controller lane below the MIDI note area. See the visual gap table
and input contract in
`docs/workflow/tasks/T057-midi-editor-and-arrangement-parity.md`.

## Prior assignment — T056

## T056 — Plug-in window memory and MIDI arrangement/editor workflow (2026-10-27)

The user confirmed Gateway meters look good and requested persistent VST Audio
Channel Settings and VST3 editor positions, VST3 editors above OpenTrack's
Project and lower-zone panels, pencil-drawn MIDI parts on MIDI/Instrument
tracks, and a MIDI Editor tab alongside Mixer and Audio Editor. Existing
Channel Settings bounds already flow through `.opr` workspace snapshots;
verify close/reopen behavior and preserve the feature. Existing MIDI parts,
note editing and a floating MIDI Editor exist, but Project pencil creation and
the docked MIDI Editor tab are missing. See
`docs/workflow/tasks/T056-plugin-window-memory-midi-arrangement-editor.md`.
Implementation uses snapped drag duration (click makes one grid unit); drawing
leaves the part unselected and the current lower-zone tab unchanged, while
double-click selects the part and opens the MIDI Editor. Native VST editor
bounds use project-workspace snapshots with owner-relative z-order. Runtime and
user acceptance remain pending.

## Prior assignment — T055

## T055 — Splash isolation, VST editor animation, and loop pencil cursor (2026-10-27)

The user confirmed that fader/EQ response is substantially improved and that
Ozone can load on Stereo Out. Follow-up concerns: startup splash must appear
alone before the main window, Ozone editor animation should continue without
pointer movement, and the Loop Draw pencil must point upper-left with its tip
at the cursor hotspot. See
`docs/workflow/tasks/T055-startup-splash-vst-editor-animation-and-draw-cursor.md`.
The user rejected the first splash approach because it opened the application
small instead of maximized, and reported that forced redraw did not fix Ozone.
Discard that approach. Rework startup to use a standalone 600×327 splash while
the maximized main window remains hidden, then reveal the main window after
initialization and a two-second minimum. Fix the VST editor message pump to
dispatch thread-level timer messages, a likely source of idle animation stalls.
The requested Ozone behavior is specifically live EQ and level-meter movement
during audio playback without pointer movement. Keep the cursor-only upper-left
pencil change. The user clarified that the README SVG is authoritative, the
background must remain grey but darker, and the app window must not load until
the splash is finished. Startup now shows the exact SVG artwork over a uniform
dark grey background, then creates the maximized main window after the splash
closes. Runtime startup appearance and Ozone playback animation remain
unverified.

The latest follow-up also addresses the remaining display/build concerns:
Gateway now receives processor-produced VST3 parameter changes on the host
control thread, editor windows request regular repaints, the executable has a
multi-size Windows icon resource, and the release script keeps build metadata
out of `builds/x64` while replacing stale package contents. Main and floating
windows are created hidden and shown after their first UI frame to reduce the
startup flash. `builds/x64` was rebuilt; the installed Gateway smoke test loads
the plug-in and processes audio successfully. The live Gateway meter behavior
and cold-start appearance still need a GUI check because the Sky computer-use
service is unavailable in this session.

## Sample Editor piano key labels and MIDI pencil (2026-10-27)

The Pitch & Time keyboard now names every key, including black keys, using
sharp names with octaves (`C#4`). The keyboard widened from 38 to 64 px so a
black key's name sits inside the black key; labels hide as a set when semitone
rows are under 12 px. Pitch-note corner handles now draw and hit-test inside
their note at every row height. The existing MIDI Editor window gains pencil
semantics: the Draw tool click- or drag-draws notes with a live preview, while
the Select tool no longer adds notes from empty cells. Source and automated
checks pass and the x64 build was rebuilt; live visual review in OpenTrack
remains pending. Whether the MIDI Editor should become a lower-zone tab beside
Audio Editor instead of a separate window is an open user decision.

## Prior assignment — T054

## T054 — Realtime control performance, Stereo Out VST3 and startup splash (2026-10-27)

The user accepted the T053 group editing tools and drag behavior as good to go.
Waveform edge cases are deferred to a separate user review and do not block
editing-tool acceptance. T054 addresses audio static/slowdown during fader and
EQ manipulation, VST3 inserts on Stereo Out, and the supplied startup splash
reference (approximately 600×327 px, centered, with a two-second minimum
display while initialization proceeds). See
`docs/workflow/tasks/T054-realtime-controls-output-vst-and-splash.md`.
The user reports substantially improved fader/EQ response and successfully
loaded Ozone on Stereo Out. Live plug-in processing and persistence remain to
be checked.
Investigation found that Channel Settings changes rebuild the complete playback
plan; each plan reconstructs per-track channel processors, including EQ filter
state, and may crossfade/render both plans. VST scanning, instance mapping and
audio hosted inserts currently enumerate project tracks only, while Stereo Out
is a separate bus. Current work should avoid rebuilding the full playback path
for every continuous control drag and must extend hosted processing to the
existing output-bus identity without disturbing normal track/group inserts.

## Accepted prior assignment — T053

Group editing tools, including live ghost/candidate drag preview and stable
cursor, are user-accepted. Group waveform edge-case study is deferred to a
future user review. The rest of T053 history is retained below.

---

## T053 implementation and build history

## T053 — Group audio tracks (2026-10-27)

Implementation and build are complete; runtime/user review is pending in
`docs/workflow/tasks/T053-group-audio-tracks.md`.
The latest drag-preview follow-up preserves the existing composite styling,
shows the original group segment as a translucent ghost, renders the candidate
segment live from its dragged child clips, and keeps the standard arrow cursor
while hovering/dragging group segments. The updated x64 local-ASIO build is
`builds/x64/OpenTrack.exe`, SHA-256
`9DB7002CCEC6C07C8B75EBDEA46A9C9AC4EEE5D5ACF207E58C66A6E71A9797FB`.
Live drag and cursor behavior still need user review.
The user confirmed that drag-and-drop grouping nests the selected track. The
existing lane-button slot uses the supplied stack-and-G SVG to collapse/expand
group children; descendant M/S indicators reflect inherited group state. Group
creation offers mono/stereo configuration; moving a group carries descendants,
and placing a track between group members assigns it to that group. The Project
track-header area is one third wider.

Collapsed groups render a normalized mono or stereo composite. Group-row
editing now supports synchronized split/cut/delete/copy/paste/move/trim and the
shared fade/gain gestures, retaining child tracks and applying common timeline
offsets. Group tracks are now actual playback buses: children with automatic
routing feed their group, nested buses process before their parents, and the
group output follows its own channel processing to Stereo Out. Automatic
routing returns children to Stereo Out when removed; explicit non-default
routes survive group membership changes. Route displays resolve group names.
The lower-zone Sample Editor tab is renamed Audio Editor. The latest user
review clarified that a collapsed group is one continuous virtual event from
the earliest child start to latest child end. Only explicit group cuts and
boundaries shared by every populated child track divide it into selectable
pieces. Unshared child clip edges remain hidden; selecting/moving a piece
operates on child clips in that range. Collapsing no longer highlights child
recordings. Project meters use the track's mono/stereo channel count. VST
Channel Settings resolves group routes to names and offers valid group outputs.

`cargo fmt --all` and `cargo check -p opentrack` pass. The x64 local-ASIO
release build passes with existing dead-code warnings. Executable:
`builds/x64/OpenTrack.exe`, SHA-256
`899B47F3C873CDFA1A78DBBBB2E7DFF8B696AA96C3AEE0368E1EDC6E2BED1B59`;
`build-info.json` confirms `local_asio: true`. Runtime review of grouped edit
gestures, routing/audio, and the updated labels is pending user acceptance.

## Prior assignment — T052

Updated: 2026-10-27. Implementer: GPT-6 Luna. Implementation incomplete;
runtime and user review pending.

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
