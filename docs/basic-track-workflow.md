# Historical basic-track workflow study

This document records an earlier workflow study. Its screenshot archive has been removed at the user's direction; it is not an active specification or acceptance gate. Current feature work follows [the delivery workflow](workflow/README.md) and the user's requirements.

## Target and capture environment

Reference: installed Cubase 5, `C:/Program Files (x86)/Steinberg/Cubase 5/Cubase5.exe`. Local Operation Manual is the documentation source. Observations must identify edition/build when established; monitor behavior differs between editions in the manual.

On 2026-10-22, Windows computer use successfully captured the reference for the first time in this investigation. The returned main-window capture was 1920 x 1032 logical pixels at screen origin (0,0). Display scaling and physical resolution are not established. The empty project was `Untitled1`; the captured workspace had a nonmaximized Project child window inside the application, a separate Transport pane, and later a Mixer child window. These are observed states, not assumed factory defaults. The user changed window layout between the first two observations.

Capture files are outside the product repository in `../_analysis-tools/reports/workflow-L1/`:

For each capture, `-0.png` is the full-screen context image. Numbered companions are separately captured windows or detail crops, including popup menus and dialogs, and should be read by their contents and manifest dimensions. The suffix meaning is capture-specific: in `04-preferred-approved`, `-1` is Transport and `-2` Compressor; in `71-window-menu`, `-1` is Compressor, `-2` Transport and `-3` the Window menu. Use the full-screen image for overall geometry, then consult the relevant companion for the detail.

- `01-empty-project-0.png`: empty Project child, application menus/toolbars, blank track area, timeline and bottom-positioned Transport.
- `01-empty-project-1.png`: bounded Transport capture; partly below the main capture.
- `02-mixer-transport-0.png`: Project child resized above a Mixer child; Transport floats across the application workspace.
- `02-mixer-transport-1.png`: bounded Transport capture.
- `02-mixer-transport-tree.txt`: accessibility tree identifying Project, Mixer and Transport roles.
- `03-preferred-layout-0.png` and companion captures: intermediate user arrangement; Channel Settings partly obscured by Project. Do not use as the geometry baseline.
- `04-preferred-approved-0.png`: unobstructed user-preferred arrangement captured at 2026-10-22T22:33:16.729Z; primary geometry reference.
- `04-preferred-approved-1.png`: Transport, 837 x 69 logical pixels at (552,687).
- `04-preferred-approved-2.png`: Compressor editor, 466 x 259 logical pixels at (1333,330).
- `04-preferred-approved-manifest.json` and `04-preferred-approved-tree.txt`: hashes, metadata and accessibility hierarchy for the preferred arrangement.

Capture geometry must be re-recorded after any user or agent layout change. Pixel measurements in a capture describe that specific state only.

## User-preferred layout observation

The user explicitly identified the arrangement as the layout they like by default. Preserve it as a preferred startup/workspace preset target; it is not established as a factory default. No reference track edits were performed by the agent in this capture pass.

In the primary `04` capture:

- Project occupies the upper workspace; Inspector is on its left, Track List next, timeline/event display to the right. Two Audio tracks are visible: Audio 01 stereo and Audio 02 mono. Audio 02 is selected and record-enabled; its displayed input is Left - Stereo In and output Stereo Out.
- Mixer occupies the lower workspace, with Stereo In, Audio 01 and Audio 02 strips at the left and Stereo Out at the right. Empty channel space fills the middle. Strip controls are compact and include an e button, monitor/record controls, routing, pan, fader and meters.
- Audio 02 Channel Settings is a compact overlapping workspace child. Inserts are an eight-slot left column, the EQ graph and four modules fill the center, eight sends form the next column, and the Audio 02 channel strip/I/O is at the right. Compressor occupies insert 1.
- Compressor has its own floating editor, separate from Channel Settings. Transport floats over the lower workspace. The accessibility tree reports Project/Mixer/Channel Settings inside the Workspace pane, with Compressor and Transport at the application level. This is hierarchy evidence, not a complete native window-style investigation.
- The displayed EQ frequencies are 100, 800, 2000 and 12000 Hz; displayed gains are zero and Q values are 7.0, 0.2, 0.2, 7.0. These are observed current-project values, not validated factory defaults or a basis to assume matching filter mathematics.

Approximate visual bounds from the 1920 x 1032 main capture (logical pixels; measured from visible borders, not OS bounds APIs):

| Region | x | y | Width | Height |
| --- | ---: | ---: | ---: | ---: |
| Project child | 6 | 44 | 1874 | 535 |
| Mixer child | 5 | 579 | 1881 | 449 |
| Channel Settings child | 382 | 145 | 902 | 458 |
| Inspector within Project | 14 | 145 | 159 | 424 |
| Track List within Project | 176 | 145 | 194 | 409 |

Use normalized placement and measured control density for design; confirm DPI and native bounds before hard-coding these pixel values. The required behavior is a movable/resizable workspace with overlapping editor windows and independent Transport/plug-in editors.

## Acceptance sequence and current gaps

| ID | Action and reference expectation | Reference evidence | Current implementation / gap |
| --- | --- | --- | --- |
| BT01 | File > New Project; choose template/Empty, then project folder; Project Setup is preference-dependent | Manual p.34 | Direct blank reset in `app.rs` `reset_project`; no template/folder flow. Reference sequence not yet captured |
| BT02 | Project > Add Track or Track List context menu; select type/count; Audio configuration chosen on creation; insert below selected track | Manual pp.38,70 | Track menu/toolbar adds one track; `Project::add_track` appends. Count/configuration dialog and reference placement missing |
| BT03 | Select row; Ctrl adds/removes noncontiguous selection, Shift selects range | Manual p.39 | Single `selected_track` index. Multi-selection absent |
| BT04 | Double-click name to rename; drag track row to reorder; drag lower border to resize, Ctrl-resize all | Manual pp.35-36,39 | Rename dialog, menu reorder and resize implemented. Drag reorder absent; exact name editing flow unverified |
| BT05 | Inspector follows selected track; toolbar toggles it; Input Routing selects configured input bus | Manual pp.70-71 | Inspector absent. Global selected input only; no per-track bus routing |
| BT06 | Operate Mute, Solo, Record Enable and monitor controls in their reference locations | Manual pp.67,72,119; edition-specific monitoring details require capture | M/S/R and Solo Defeat implemented. No per-track monitor button; global input monitoring attached to recording. Visual/behavior comparison pending |
| BT07 | `e` from Track List/Inspector/Mixer opens Audio Channel Settings; channel synchronization follows preference; Alt+e can open multiple windows | Manual pp.122-123 | One shared native Channel Settings viewport follows selected track. Mixer strip lacks direct e entry. Multiple settings instances and sync preference absent |
| BT08 | Channel Settings has eight insert slots, four EQ modules plus curve, eight sends and duplicate channel strip/I/O panel by default; panels customizable | Manual pp.122-125 | Original vertical settings form with four-band EQ and variable insert chain. Curve, sends, reference structure and slot layout missing |
| BT09 | Arm Audio/MIDI tracks, record at cursor, stop, play takes; compare monitor and meter behavior | Manual pp.67-72 | Shared audio input fans to separate armed-track WAVs; MIDI capture exists. Device/timing acceptance pending; no per-track source routing |
| BT10 | Save/reopen project and confirm content/channel state | Native acceptance requirement; CPR is a separate investigation | RWP v15 serialization exists. Two-track live reopen and new track pan/delay values confirmed; full L2/L4 round-trip remains pending. No CPR importer |
| BT11 | Move/resize/open/close windows, adjust zoom/heights; switch unlocked/locked workspaces and recall state | Manual pp.469-470 | Workspace snapshots exist. Current dock/native viewport model differs from observed reference child-window workspace; runtime comparison pending |

Source audit locations (as of L0): `app.rs` reset 1251, track menus 3702, Channel Settings viewport 4097, toolbar 4336, track controls 5155, settings form 6315; `project.rs` add 304 and move 380. Use symbols/search after edits because line numbers move.

## First UI scope proposed for L2

Reproduce the application/Project window hierarchy, compact toolbar and info/ruler regions, Inspector/Track List/event-display arrangement, and basic track creation/selection/reorder/control flow. Use original colors/icons. Match the reference organization and interaction before adding new feature categories. Decide whether the current egui dock/native viewport architecture can reproduce the observed child-window workspace; record the decision before the UI rebuild.

## Measurements still required

The expanded menu/Project/Channel Settings pass now records the top-level menu sweep, Audio/MIDI/Instrument add dialogs, Inspector sections, M/S/R toggles, Channel Settings structure and visible routing choices. The user accepted this capture and requested an ordered tooltip survey; see [L1 menu and Project capture](reviews/L1-menu-and-project-capture.md), the [tooltip sweep log](reviews/tooltip-sweep-log.md), and captures in `../../_analysis-tools/reports/workflow-L1/`.

Still required: New Project/template/folder flow; track rename, drag reorder, row resize/folding; monitor behavior; populated Audio/MIDI event workflows; actual recording/playback and signal behavior; save/reopen; and workspace recall. Record focus, reuse versus new-window behavior, shortcuts, bounds and scale. Audio timing and signal checks are separate from visual capture.

## Review result

L1 was accepted and the user requested the L2 implementation. Project-window, Inspector, track-list, and Add Track source changes are present, with live screenshots and comparisons captured through `node_repl` and `@oai/sky`. Track creation, record enable, Project-child drag, two-track RWP reopen and new track pan/delay save/reopen have been exercised; this does not establish full L2 or audio parity. See the [M2 review report](../../_analysis-tools/reports/opentrack-comparisons/M2-review-report.md) for current captures, remaining layout/behavior gaps and build status. The [remote review workflow](reviews/remote-review-workflow.md) records the working desktop hook. Use the saved Cubase evidence for this checkpoint.


## Project file naming (2026-10-23)

New saves use **OpenTrack Project (`.opr`)**. The Open dialog also accepts existing `.otp` projects and legacy `.rwp` projects. Saving one of those older formats offers a new OPR path and preserves the original. The internal serialization identifier and version remain compatible; no project file contents are rewritten by changing the filename extension.
