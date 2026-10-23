# L1 reference capture: menus, Project window and Channel Settings

Date: 2026-10-22. Status: expanded reference capture accepted for continued evidence collection. This is an observation log, not a claim of feature or visual parity.

## Reference state and evidence

Reference: Cubase 5.1.1 Build 120, build date Nov. 23, 2009, running in the installed 32-bit program folder. Captures from this pass use an approximately 1906 x 1043 logical-pixel screenshot beginning at screen x=7. Display scaling and physical resolution remain unknown. The earlier user-preferred arrangement capture `04-preferred-approved-0.png` remains the geometry baseline; the later captures show the live session after exploration.

The open project is `Untitled1`, with no timeline events and two audio tracks: Audio 01 (stereo) and Audio 02 (mono). Audio 02 is selected and record-enabled; its current input is Left - Stereo In and output is Stereo Out. The Compressor shown in slot 1 was loaded by the user as an example. Slots 2-8 are empty. This is the current user-set project state, not a factory default.

Captures live outside the product repo at `../../../_analysis-tools/reports/workflow-L1/`. The initial menu sweep is numbered 10-43; Project, track, add-dialog and routing captures include 44-78. Each capture has a PNG image and, where available, an accessibility tree and SHA-256 manifest. The `-0.png` image is the full-screen context shot; numbered companions are supplementary crops of windows, menus, submenus or dialogs and are useful on their own. Their index is local to each capture, not a fixed global legend: `04-preferred-approved-1.png` is Transport and `-2.png` is Compressor, while `71-window-menu-1.png` is Compressor, `-2.png` is Transport and `-3.png` is the Window menu. For example, compare [preferred Transport](../../../_analysis-tools/reports/workflow-L1/04-preferred-approved-1.png), [preferred Compressor](../../../_analysis-tools/reports/workflow-L1/04-preferred-approved-2.png), [Window-menu Compressor](../../../_analysis-tools/reports/workflow-L1/71-window-menu-1.png), [Window-menu Transport](../../../_analysis-tools/reports/workflow-L1/71-window-menu-2.png) and [Window menu crop](../../../_analysis-tools/reports/workflow-L1/71-window-menu-3.png). Use the `-0` image for overall arrangement and the companion image that shows the specific detail being reviewed. Key full-screen images: [user-preferred layout](../../../_analysis-tools/reports/workflow-L1/04-preferred-approved-0.png), [Project window with track controls](../../../_analysis-tools/reports/workflow-L1/70-inspector-header-click-0.png), [Channel Settings with EQ band restored](../../../_analysis-tools/reports/workflow-L1/77-eq-band-one-restored-0.png), [review-ready layout](../../../_analysis-tools/reports/workflow-L1/78-review-ready-layout-0.png).

## Top-level menu sweep

Availability below is what appeared in this empty project. Commands disabled without an event or MIDI part may become available in a populated project; disabled does not mean unsupported.

| Menu | Observed groups and notable commands | Evidence |
| --- | --- | --- |
| File | New/Open/Close/Save, Backup Project, Save as Template, Import/Export, Cleanup, Preferences, Key Commands, Quit. Import offers audio, CD, video, MIDI, track archive, tempo track, MusicXML and OMF. Export offers audio mixdown, selected tracks and tempo track; several other formats were disabled here. | `10-file-menu`, `11-file-import-submenu`, `12-file-export-submenu` |
| Edit | Undo/Redo and History; clip operations; Split at Cursor (Alt+X); Range, Select, Move, object and Duplicate/Repeat groups; Automation follows Events is checked; Auto Select Events under Cursor; track sizing; Zoom. Zoom shortcuts include H/G, Shift+F, Alt+S, Shift+E, Alt+Down, Ctrl+Up and Ctrl+Down. | `13-edit-menu` through `16-edit-zoom-submenu` |
| Project | Add Track, Duplicate/Remove Tracks, Track Folding, Pool (Ctrl+P), Markers (Ctrl+M), Tempo Track (Ctrl+T), Browser (Ctrl+B), Automation Panel, Beat Calculator, timecode, Notepad, Project Setup (Shift+S), Auto Fades. | `17-project-menu` through `20-project-track-folding` |
| Audio | Event-process commands (fade, gain, normalize, reverse, pitch/time stretch, resample and others), plug-in categories, Hitpoints, realtime processing and advanced processing. Process commands were disabled with no event selected. | `21-audio-menu` through `27-audio-advanced-submenu` |
| MIDI | Key/Score/Drum/List/In-place editors, quantize setup, Logical Presets, Drum Map, Insert Velocities, VST Expression, CC Automation and Reset. Editors and many event commands were disabled because this project has no MIDI part. | `28-midi-menu` through `31-midi-functions` |
| Scores | Score editor/layout, note and symbol editing, alignment, functions and advanced layout. These were disabled with no score content selected. | `32-scores-menu` through `35-scores-advanced-layout` |
| Media | Pool (Ctrl+P), MediaBay (F5), Loop Browser (F6), Sound Browser (F7), import, missing-file handling, conversion/conform, video-audio extraction, thumbnail cache, trash/removal, archive preparation, record folder, versions and insert/select/search actions. | `36-media-menu` |
| Transport | Transport Panel (F2), marker/event navigation, locator and playback commands, pre/post-roll, record-at-left-locator, Metronome (C), precount, synchronization and external sync. State-dependent actions were disabled. | `37-transport-menu` |
| Devices | Control Room, MIDI Device Manager/MMC, Mixer (F3) and Mixer 2/3, plug-in information, record-time/time displays, VST Connections (F4), VST Instruments (F11), performance (F12), Video Window (F8), Virtual Keyboard (Alt+K), Show Panel and Device Setup. | `38-devices-menu` |
| Window | Minimize/Maximize, Close All, Minimize All, Restore All, Workspaces and Windows. The open-window list showed Project, Transport, Mixer, Channel Settings and the Compressor editor. Workspace `Main` was active; its lock/new/organize actions were visible. | `39-window-menu` through `41-windows-dialog` |
| Help | Web/registration links, credits/copyright and About. About identifies Cubase 5.1.1 Build 120. | `42-help-menu`, `43-about-cubase` |

I did not invoke destructive cleanup, remove, close-all, record, or project/file commands during this sweep.

## Project window and track flow

The Project child has a compact toolbar across the top, an info line reading “No Object Selected,” a left Inspector, a Track List, a bar ruler and an empty event grid. The visible ruler spans bars 1-49. The toolbar includes tool-selection icons, snap/grid controls (the visible values are Bar and 1/16), and color controls. Exact behavior of every toolbar icon is not yet mapped. See [Project window in front](../../../_analysis-tools/reports/workflow-L1/68-project-window-front-0.png).

The Inspector follows the selected track. Its main track panel includes M/S/R/W controls, Record Enable, a monitor control, level/meter display, input and output selectors, a track-preset selector and collapsible Inserts, Equalizers, Sends, Channel, Notepad and Quick Controls sections. The Track List shows track number/type/name and compact controls. Expanding an Inspector section exposes that section's controls; the Channel section shows a small channel strip. Screenshots `47-inspector-eq-expanded` through `53-project-inspector-collapsed` document these states. The track context menu includes Add Track, Select All Events, Duplicate/Remove Tracks, Track Controls Settings, automation visibility commands, Disable Track, Set Record Folder, Auto Fades, and create/apply Track Preset. See [track context menu](../../../_analysis-tools/reports/workflow-L1/09-audio-track-context-0.png).

Observed interactions:

- Selecting Audio 01 or Audio 02 changes the Inspector target and Channel Settings title/strip. The original Audio 02 selection was restored.
- Mute lights on the selected track and its Mixer strip; Solo lights the selected track and shows the other track muted while Solo is active. Both controls were turned back off. Evidence: `54-audio02-muted`, `55-audio02-solo`.
- Record Enable is a toggle; its tooltip reads “Record Enable.” Turning it off clears the red arm state in the Project/Inspector and Mixer; turning it back on restores the original Audio 02 state. Transport Record was not started. Evidence: `73-audio02-record-disarmed`, `74-audio02-record-restored`.
- The monitor control is visible but was not activated. Its live input/output behavior remains unknown.
- The `e` control in the Inspector and Mixer opens/closes Channel Settings. The Mixer tooltip reads “Edit Audio Channel Settings.” The single shared Settings window follows the selected Audio track in these observations. Evidence: `56-channel-settings-open`, `57-mixer-edit-channel-settings-tooltip`.

Add Track dialogs were opened and cancelled, so no tracks were created:

- Audio: count 1 and configuration Mono (displayed as `M`); evidence [Audio dialog](../../../_analysis-tools/reports/workflow-L1/07-add-audio-dialog-0.png).
- Instrument: count 1, VST selector defaulting to “No VST Instrument,” a Browse Presets option, and this install's visible choices Groove Agent ONE and Synth; evidence [Instrument dialog](../../../_analysis-tools/reports/workflow-L1/61-add-instrument-dialog-0.png) and [instrument list](../../../_analysis-tools/reports/workflow-L1/62-instrument-dropdown-3.png).
- MIDI: count 1 and Browse Presets; evidence [MIDI dialog](../../../_analysis-tools/reports/workflow-L1/63-add-midi-dialog-0.png).

The Instrument list is only an observation of this install's available content. It does not establish default instruments for other machines or licenses.

## Channel Settings and Mixer surface

Channel Settings is a separate workspace child opened from track/Mixer `e` controls. The layout has eight insert slots at left, a central EQ graph with four band modules, eight send slots, and a selected-channel strip at right. The separate Compressor editor remains a distinct floating window. The live workspace list and open-window dialog record Project, Transport, Mixer, Channel Settings and Compressor as five windows.

- **Inserts:** slot 1 contains the user's Compressor example; slots 2-8 are empty. The empty-slot chooser exposes No Effect and plug-in categories; no new plug-in was loaded in this pass. Earlier evidence: `08-empty-insert-chooser` and current state in `72-channel-settings-front`.
- **EQ:** four modules (`eq 1`-`eq 4`) display 100, 800, 2000 and 12000 Hz, gains 0.0 dB, and Q values 7.0, 0.2, 0.2 and 7.0 in this project. The graph is flat. These are observed current values, not validated factory defaults. Toggling band 1 turns its enable control green and shows a `Low Shelf I` label/node at 100 Hz; toggling it back restores the original display. Evidence: [band 1 enabled](../../../_analysis-tools/reports/workflow-L1/76-eq-band-one-toggle-0.png), [band 1 restored](../../../_analysis-tools/reports/workflow-L1/77-eq-band-one-restored-0.png).
- **Sends:** eight slots are shown and currently have no bus. The chooser offers No Bus and Outputs; Outputs expands to Stereo Out, Left and Right. No send was assigned. Evidence: `58-send-slot-no-bus`, `59-send-outputs-submenu`.
- **Routing:** Audio 02's input selector offers No Bus and Stereo In; Stereo In expands to Left and Right. Its current Left - Stereo In routing was left intact. Output offers No Bus, Stereo Out (checked), Left and Right. The channel strip shows mono, pan/input controls, M/S/L/R/W controls, a meter and fader. The input gain tooltip says “Input Gain (use shift).” Evidence: [input buses](../../../_analysis-tools/reports/workflow-L1/65-channel-input-menu-0.png), [Stereo In channels](../../../_analysis-tools/reports/workflow-L1/66-channel-input-sources-0.png), [output buses](../../../_analysis-tools/reports/workflow-L1/67-channel-output-menu-0.png).
- **Mixer:** the preferred capture shows Stereo In, Audio 01, Audio 02 and Stereo Out strips, compact control buttons, routing, pan, meters and faders. Track Mute/Solo and Channel Settings access were checked; no fader, pan or routing value was changed.

## State changes and remaining unknowns

No audio or MIDI event, track, effect, send, bus assignment, or file was created. Mute, Solo, Record Enable and EQ band 1 were returned to their captured starting states. Add Track dialogs and routing menus were dismissed without a selection. The Inspector track-control panel was reopened to match the preferred capture. The user-loaded Compressor remains in insert 1 as it was at the start of this pass.

L1 still needs its user review and has open evidence gaps: New Project/template/folder flow; track drag reorder and row resize/folding; actual monitor behavior; track input/output choices beyond the visible buses; populated Audio/MIDI events; audio/MIDI recording and playback; save/reopen; and workspace recall. The Mixer and Channel Settings parameter controls need deeper value-change/persistence mapping. Full product behavior is far beyond this menu and empty-project survey.

## Review checkpoint

The user accepted this reference capture as a sound basis and requested a systematic tooltip sweep before deeper implementation. Follow [the tooltip sweep log](tooltip-sweep-log.md) in its stated order and preserve the captured project state. Once that sweep and review are complete, continue with the L2 track sequence (add, select, rename, reorder/resize, controls) before substantial L2 UI changes. The product code has not been changed during this capture pass.
