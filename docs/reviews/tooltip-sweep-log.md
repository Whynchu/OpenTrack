# Reference tooltip sweep log

Status: Project-window pass captured through observation 90 (T4-90), including lower-area probes through T5-86; Mixer window pass captured in 97 t3 observations. Those passes contain 187 tooltip-sweep observations: 153 verified tooltips, 33 verified no-tooltip samples, and one ambiguous toolbar edge probe excluded from coverage. Additional Transport Panel and Channel Settings observations are recorded separately as t4-002–081. The Transport dropdown is already inventoried in L1; t4-001 is a redundant re-capture, not new coverage.

## Latest state (2026-10-22)

- The user confirmed that the menu labels File, Edit, Project, Audio, MIDI, Scores, Media, Transport, Devices, Window, and Help have no tooltips. Keep the menu-command inventory separate from tooltip coverage.
- Audio 01 and Audio 02 were created by the user before this session and are intentionally present as analysis/reference tracks. Do not treat them as the default project template.
- The ordered hover sweep has covered the Project toolbar; Audio 02 Inspector visible controls and collapsed section headers; Track List global controls; both visible audio-track control rows, names, and type labels; the Project info line, ruler, empty timeline lane, lower status/readout, scrollbar, and zoom sample points.
- Captures are saved under ../../../_analysis-tools/reports/workflow-L2/tooltips/ as full-screen -0.jpg images with JSON manifests. The current t4 tooltips were checked against the saved screenshots because the accessibility tree sometimes retained the previous tooltip after a move.
- The first power-symbol attempt at (25, 101) landed too far left and is excluded. The user had to move the cursor manually to trigger the tooltip. A clean retry from the blank grid to (30, 101) verified the actual cursor and showed Activate Project; that capture is the accepted P-1 result.
- Toolbar control T2-03 at x=80 remains an ambiguous edge probe with no tooltip and is excluded from coverage. Info line, timeline ruler, and an empty track lane were each sampled and showed no tooltip.
- A save-name collision between the Color Selector and preceding Snap to Zero Crossing captures was caught; Snap to Zero Crossing was re-hovered and re-saved as T2-35, and Color Selector is now saved separately as T2-36.
- The Mixer pass captured 97 full-screen -0.jpg images with JSON manifests. Mixer controls were hovered only; no buttons, menus, values, or routing were changed.
- L1 already contains the Transport dropdown inventory. t4-001 is a redundant full-screen re-capture and is not counted as new coverage.
- The separate floating Transport Panel was captured as t4-002 through t4-031. VST Audio Channel Settings for Audio 02 was captured as t4-032 through t4-081. The compressor remains the user-provided example in Insert 1; Inserts 2–8 and all eight Sends are empty, and the four-band EQ is flat. Only hover inputs were used on controls; a title-bar click raised the existing Channel Settings child. No transport, mute/solo/record, routing, fader, insert, send, or EQ state was changed.

## Initial MCP smoke check (resolved, 2026-10-22)

Codex runtime discovery succeeded: `hover_cubase` is registered. Two hover probes were rejected because Windows reported `WindowsTerminal.exe` foreground rather than `Cubase5.exe`; both responses confirmed the pointer was not moved. After the user foregrounded Cubase, the next probe reached the Cubase coordinate guard and returned `Target coordinate is outside the foreground Cubase window; pointer was not moved`. The old capture coordinate `(277, 246)` is therefore unsuitable for the current foreground window. The active Cubase window could be a smaller editor or have different geometry; neither explanation has been visually verified. Obtain a fresh screenshot and geometry before selecting another target.

The separate screenshot connection failed because its native server endpoint is absent. Read-only diagnostics confirmed that the named pipe configured by `SKY_CUA_NATIVE_PIPE_DIRECTORY` in the manual `mcp_servers.node_repl.env` registration does not exist and that zero `codex-computer-use` processes are running. The same pipe name was present in the previous day's config backup. JavaScript reset/reinitialization did not restore the server. The alternate CUA surface exposes only browser methods and no native apps. No matching recent Application crash event was found; why the native server stopped remains unknown. No config, app access policy, or native helper lifecycle was changed during diagnosis.

Recovery: use the desktop app's Plugins > Computer Use controls to enable the server and skill, then select Try now, as described in the [official Computer Use setup instructions](https://learn.chatgpt.com/docs/computer-use). If already enabled, restart the desktop app to attempt native-server initialization. Verify that the configured pipe exists and `sky.list_apps()` succeeds before resuming screenshots. If the app creates a different endpoint while the manual registration retains the old one, reconcile the registration against the app-provided connection rather than guessing another pipe name.

Recovery completed after the terminal/session restart. The new session listed Cubase, brought its main window forward, and captured the power-button hover. The prior missing-pipe and stale-process observations above are historical.

The screenshot path is separate from the hover helper: `sky.get_window_state()` captures the UI and accessibility text after each hover. The helper itself does not read tooltip text or capture screenshots. Keep the ordered sweep pending until the corrected helper moves the visible cursor and a screenshot confirms the tooltip.

## Capture rules

- Sweep visible UI regions from left to right, then top to bottom. Work by window when another window overlaps the current one.
- Hover each relevant icon, button, and field without clicking it; record complete tooltip text, including shortcut hints and line breaks when visible. Menu labels are not tooltip targets per the user's observation.
- Open menus to inventory their commands separately; expand a panel only when needed to reveal its contents. Dismiss menus without choosing a command.
- Record whether a tooltip appeared, its delay if noticeable, and any ambiguous/truncated text. Do not infer text from an icon or label.
- Use the reloaded `hover_cubase` tool to inject a mouse-move event and confirm its actual cursor coordinates. Check the current screenshot for the live tooltip when the accessibility tree appears stale; do not trust a repeated tooltip label after moving to another control. The helper is restricted to the foreground Cubase 5 process and never clicks, types, or scrolls.
- Save one full-screen context image per target as -0.jpg with a JSON manifest. Do not create -1/-2 crops for this tooltip sweep; the user says only the -0 images are relevant here. Existing separate-window Compressor and Transport references remain independent captures.
- Preserve the current Cubase project/window arrangement and the user-loaded Compressor example. Restore any panel expansion or selection change used only to expose a tooltip.

## Sweep order and coverage

Mark an item complete only after its visible targets have been hovered and logged. Add rows for repeated controls where the tooltip differs by track, channel, or state.

| Order | Surface | Left-to-right / top-to-bottom targets | Status |
| --- | --- | --- | --- |
| T1 | Application frame and menu bar | Menu-command inventory already captured in L1; top-level labels are not tooltip targets | Menu inventory complete; label tooltips N/A |
| T2 | Project top area | Power button; toolbar icons and fields; info line; ruler/navigation and grid/snap controls | First pass captured; x=80 edge probe unresolved and excluded |
| T3 | Project left area | Inspector header and controls; routing/preset fields; Inserts, Equalizers, Sends, Channel, Notepad, Quick Controls headers and expanded contents | Visible controls and collapsed headers captured; expanded contents pending |
| T4 | Track List | Track headers, row controls, track names/type indicators, per-track M/S/R/W/monitor/meter controls; both visible audio tracks | Global controls, Audio 01/02 names/type labels, and M/S/R/W/record/monitor captured; meters pending |
| T5 | Timeline and lower Project area | Ruler controls, event-grid controls, scroll/zoom controls and status/readout areas | First pass sampled through status, scrollbars, and zoom; no tooltip at sampled points |
| T6 | Mixer | Toolbar and every visible strip from left to right; routing, pan, M/S/R/W/monitor, channel-settings, fader, meter, and strip labels | First pass captured through t3-097; review at final landmark |
| T7 | Transport Panel | Separate floating panel controls, locators, displays, click/metronome, tempo, signature, sync, and markers | Captured through t4-031; dropdown inventory remains in L1; t4-001 is redundant |
| T8 | Channel Settings | Header/window controls; insert slots; EQ graph, band switches and numeric controls; send slots; channel strip and I/O controls | Captured through t4-081; screenshot-verified corrections applied |
| T9 | Compressor editor | Window controls, toolbar, every parameter/control and graph/meter element | Pending |
| T10 | Final pass | Revisit unresolved/ambiguous targets, verify captured text, restore the preferred layout, and take a full-screen review image | Pending |

For menu inventory, record the visible command text and any shortcut hints, then inspect revealed submenus in reading order. Disabled commands still count as visible inventory. Menu labels are excluded from the tooltip sweep based on the user's clarification.

## Observation ledger

| ID | Surface / target | Tooltip text (verbatim) | State / notes | Screenshot(s) | Result |
| --- | --- | --- | --- | --- | --- |
| P-1 | Project toolbar power button | `Activate Project` | Verified at actual (30,101); first (25,101) probe excluded as too far left. | `t2-001-project-power-0.jpg` | Verified |
| T2-02 | Constrain Delay Compensation | `Constrain Delay Compensation` | Verified. | `t2-002-project-toolbar-button-02-0.jpg` | Verified |
| T2-03 | Toolbar probe after Constrain Delay Compensation | No tooltip | Ambiguous edge probe at x=80; excluded from target coverage. | `t2-003-project-toolbar-button-03-0.jpg` | Unresolved / excluded |
| T2-04 | Show Inspector | `Show Inspector` | Verified. | `t2-004-project-show-inspector-0.jpg` | Verified |
| T2-05 | Show Event Info Line | `Show Event Info Line` | Verified. | `t2-005-project-show-event-info-line-0.jpg` | Verified |
| T2-06 | Show Overview | `Show Overview` | Verified. | `t2-006-project-show-overview-0.jpg` | Verified |
| T2-07 | Open Pool | `Open Pool` | Verified. | `t2-007-project-open-pool-0.jpg` | Verified |
| T2-08 | Open Mixer | `Open Mixer` | Verified. | `t2-008-project-open-mixer-0.jpg` | Verified |
| T2-09 | Show Automation Panel | `Show Automation Panel` | Verified. | `t2-009-project-show-automation-panel-0.jpg` | Verified |
| T2-10 | Global Automation Mode | `Global Automation Mode` | Verified; displayed mode Touch. | `t2-010-project-global-automation-mode-0.jpg` | Verified |
| T2-11 | Goto Previous Marker / Zero | `Goto Previous Marker / Zero` | Verified. | `t2-011-project-transport-previous-marker-zero-0.jpg` | Verified |
| T2-12 | Goto Next Marker | `Goto Next Marker` | Verified. | `t2-012-project-transport-next-marker-0.jpg` | Verified |
| T2-13 | Transport Cycle On/Off | `Transport Cycle On/Off` | Verified. | `t2-013-project-transport-cycle-0.jpg` | Verified |
| T2-14 | Transport Stop | `Transport Stop` | Verified. | `t2-014-project-transport-stop-0.jpg` | Verified |
| T2-15 | Transport Play | `Transport Play` | Verified. | `t2-015-project-transport-play-0.jpg` | Verified |
| T2-16 | Transport Record | `Transport Record` | Verified. | `t2-016-project-transport-record-0.jpg` | Verified |
| T2-17 | Object Selection | `Object Selection` | Verified. | `t2-017-project-object-selection-0.jpg` | Verified |
| T2-18 | Range Selection | `Range Selection` | Verified. | `t2-018-project-range-selection-0.jpg` | Verified |
| T2-19 | Split | `Split` | Verified. | `t2-019-project-split-tool-0.jpg` | Verified |
| T2-20 | Glue | `Glue` | Verified. | `t2-020-project-glue-tool-0.jpg` | Verified |
| T2-21 | Erase | `Erase` | Verified. | `t2-021-project-erase-tool-0.jpg` | Verified |
| T2-22 | Zoom | `Zoom` | Verified. | `t2-022-project-zoom-tool-0.jpg` | Verified |
| T2-23 | Mute tool | `Mute` | Verified. | `t2-023-project-mute-tool-0.jpg` | Verified |
| T2-24 | Time Warp | `Time Warp` | Verified. | `t2-024-project-time-warp-tool-0.jpg` | Verified |
| T2-25 | Draw | `Draw` | Verified. | `t2-025-project-draw-tool-0.jpg` | Verified |
| T2-26 | Line | `Line` | Verified. | `t2-026-project-line-tool-0.jpg` | Verified |
| T2-27 | Play tool | `Play` | Verified; distinct from Transport Play. | `t2-027-project-play-tool-0.jpg` | Verified |
| T2-28 | Color Tool | `Color Tool` | Verified. | `t2-028-project-color-tool-0.jpg` | Verified |
| T2-29 | Project Root Key | `Project Root Key` | Verified. | `t2-029-project-root-key-0.jpg` | Verified |
| T2-30 | Autoscroll | `Autoscroll` | Verified. | `t2-030-project-autoscroll-0.jpg` | Verified |
| T2-31 | Snap On/Off | `Snap On/Off` | Verified. | `t2-031-project-snap-toggle-0.jpg` | Verified |
| T2-32 | Snap Type | `Snap Type` | Verified. | `t2-032-project-snap-type-0.jpg` | Verified |
| T2-33 | Grid Type | `Grid Type` | Verified; displayed Bar. | `t2-033-project-grid-type-0.jpg` | Verified |
| T2-34 | Quantize Type | `Quantize Type` | Verified; displayed 1/16. | `t2-034-project-quantize-type-0.jpg` | Verified |
| T2-35 | Snap to Zero Crossing | `Snap to Zero Crossing` | Verified; capture restored after filename collision and retaken at x=1224. | `t2-035-project-snap-zero-crossing-0.jpg` | Verified |
| T2-36 | Color Selector | `Color Selector` | Verified at actual (1265,101). | `t2-036-project-color-selector-0.jpg` | Verified |
| T5-37 | Project info line (No Object Selected) | No tooltip | No tooltip after 2.5 s at actual (55,136). | `t2-037-project-info-line-0.jpg` | Verified absence |
| T5-38 | Timeline ruler near bar 2 | No tooltip | No tooltip after 2.5 s at actual (400,164). | `t2-038-project-ruler-0.jpg` | Verified absence |
| T3-39 | Audio 02 Inspector Edit Channel Settings | `Edit Channel Settings` | Verified. | `t2-039-inspector-edit-channel-settings-0.jpg` | Verified |
| T4-40 | Track list Track Controls Settings | `Track Controls Settings` | Verified. | `t2-040-track-list-controls-settings-0.jpg` | Verified |
| T4-41 | Track list In-place Editor toggle | `Toggle In-place Editor for Selected Tracks` | Verified. | `t2-041-track-list-in-place-editor-0.jpg` | Verified |
| T4-42 | Track list Unmute All | `Unmute All` | Verified. | `t2-042-track-list-unmute-all-0.jpg` | Verified |
| T4-43 | Track list Deactivate All Solo | `Deactivate All Solo` | Verified. | `t2-043-track-list-deactivate-all-solo-0.jpg` | Verified |
| T4-44 | Track list global automation read status | `All Automation to Read Status` | Verified. | `t2-044-track-list-all-automation-read-0.jpg` | Verified |
| T4-45 | Track list global automation write status | `All Automation to Write Status` | Verified. | `t2-045-track-list-all-automation-write-0.jpg` | Verified |
| T4-46 | Track list Show/Hide Track Colors | `Show/Hide Track Colors` | Verified. | `t2-046-track-list-show-hide-track-colors-0.jpg` | Verified |
| T4-47 | Audio 01 Mute | `Mute` | Verified. | `t2-047-audio01-track-mute-0.jpg` | Verified |
| T4-48 | Audio 01 Solo | `Solo State (Solo Defeat with ALT click)` | Verified; no click sent. | `t2-048-audio01-track-solo-0.jpg` | Verified |
| T4-49 | Audio 01 Read Enable | `Read Enable` | Verified. | `t2-049-audio01-read-enable-0.jpg` | Verified |
| T4-50 | Audio 01 Write Enable | `Write Enable` | Verified. | `t2-050-audio01-write-enable-0.jpg` | Verified |
| T4-51 | Audio 01 Record Enable | `Record Enable` | Verified. | `t2-051-audio01-record-enable-0.jpg` | Verified |
| T4-52 | Audio 01 Monitor | `Monitor` | Verified. | `t2-052-audio01-monitor-0.jpg` | Verified |
| T4-53 | Audio 02 Mute | `Mute` | Verified. | `t2-053-audio02-track-mute-0.jpg` | Verified |
| T4-54 | Audio 02 Solo | `Solo State (Solo Defeat with ALT click)` | Verified; no click sent. | `t2-054-audio02-track-solo-0.jpg` | Verified |
| T4-55 | Audio 02 Record Enable | `Record Enable` | Verified. | `t2-055-audio02-record-enable-0.jpg` | Verified |
| T4-56 | Audio 02 Monitor | `Monitor` | Verified. | `t2-056-audio02-monitor-0.jpg` | Verified |
| T4-57 | Audio 02 Read Enable | `Read Enable` | Verified. | `t2-057-audio02-read-enable-0.jpg` | Verified |
| T4-58 | Audio 02 Write Enable | `Write Enable` | Verified. | `t2-058-audio02-write-enable-0.jpg` | Verified |
| T5-59 | Empty Audio 01 timeline lane near bar 13 | No tooltip | No tooltip after 2.5 s at actual (800,190). | `t2-059-project-empty-track-lane-0.jpg` | Verified absence |
| T3-60 | Audio 02 Inspector Mute | `Mute` | Verified. | `t2-060-inspector-audio02-mute-0.jpg` | Verified |
| T3-61 | Audio 02 Inspector Solo | `Solo State (Solo Defeat with ALT click)` | Verified; no click sent. | `t2-061-inspector-audio02-solo-0.jpg` | Verified |
| T3-62 | Audio 02 Inspector Read Enable | `Read Enable` | Verified. | `t2-062-inspector-audio02-read-enable-0.jpg` | Verified |
| T3-63 | Audio 02 Inspector Write Enable | `Write Enable` | Verified. | `t2-063-inspector-audio02-write-enable-0.jpg` | Verified |
| T3-64 | Audio 02 Inspector Open Device Panels | `Open Device Panels` | Verified. | `t2-064-inspector-open-device-panels-0.jpg` | Verified |
| T3-65 | Audio 02 Inspector Auto Fades Settings | `Auto Fades Settings...` | Verified. | `t2-065-inspector-auto-fades-settings-0.jpg` | Verified |
| T3-66 | Audio 02 Inspector Enable Record | `Enable Record` | Verified; actual x=38 for requested x=39. | `t2-066-inspector-enable-record-0.jpg` | Verified |
| T3-67 | Audio 02 Inspector Timebase toggle | `Toggle Timebase between Musical and Linear` | Verified. | `t2-067-inspector-timebase-toggle-0.jpg` | Verified |
| T3-68 | Audio 02 Inspector Lock | `Lock` | Verified. | `t2-068-inspector-lock-0.jpg` | Verified |
| T3-69 | Audio 02 Inspector Lane Display Type | `Lane Display Type` | Verified. | `t2-069-inspector-lane-display-type-0.jpg` | Verified |
| T3-70 | Audio 02 Inspector Freeze Audio Channel | `Freeze Audio Channel` | Verified. | `t2-070-inspector-freeze-audio-channel-0.jpg` | Verified |
| T3-71 | Audio 02 Inspector Volume | `Volume` | Verified; hover only, no value change. | `t2-071-inspector-volume-0.jpg` | Verified |
| T3-72 | Audio 02 Inspector Pan | `Pan` | Verified; hover only, no value change. | `t2-072-inspector-pan-0.jpg` | Verified |
| T3-73 | Audio 02 Inspector Input Routing | `Input Routing (all selected with Shift + ALT + click)` | Verified; no click or routing change. | `t2-073-inspector-input-routing-0.jpg` | Verified |
| T3-74 | Audio 02 Inspector Output Routing | `Output Routing (all selected with Shift + ALT + click)` | Verified; no click or routing change. | `t2-074-inspector-output-routing-0.jpg` | Verified |
| T3-75 | Audio 02 Inspector Apply Track Preset | `Apply Track Preset` | Verified; no preset change. | `t2-075-inspector-apply-track-preset-0.jpg` | Verified |
| T3-76 | Audio 02 Inspector Inserts visibility | `Shows Active Inserts` | Verified; section not expanded. | `t2-076-inspector-shows-active-inserts-0.jpg` | Verified |
| T3-77 | Audio 02 Inspector Equalizers visibility | `Shows Active Equalizers` | Verified; section not expanded. | `t2-077-inspector-shows-active-equalizers-0.jpg` | Verified |
| T3-78 | Audio 02 Inspector Sends visibility | `Shows Active Sends` | Verified; section not expanded. | `t2-078-inspector-shows-active-sends-0.jpg` | Verified |
| T3-79 | Audio 02 Inspector Notepad | `Notepad is empty` | Verified; no note edit. | `t2-079-inspector-notepad-empty-0.jpg` | Verified |
| T3-80 | Audio 02 Inspector Quick Controls visibility | `Shows Active Quick Controls` | Verified; section not expanded. | `t2-080-inspector-shows-active-quick-controls-0.jpg` | Verified |
| T5-81 | Project lower status/readout area | No tooltip | No tooltip after 2.5 s at actual (260,570). | `t2-081-project-status-readout-0.jpg` | Verified absence |
| T5-82 | Project vertical scrollbar near the top | No tooltip | No tooltip after 2.5 s at actual (1868,200). | `t2-082-project-vertical-scrollbar-0.jpg` | Verified absence |
| T5-83 | Project horizontal scrollbar left end | No tooltip | No tooltip after 2.5 s at actual (383,570). | `t2-083-project-horizontal-scrollbar-0.jpg` | Verified absence |
| T5-84 | Project lower-right zoom slider | No tooltip | No tooltip after 2.5 s at actual (1840,570). | `t2-084-project-zoom-control-0.jpg` | Verified absence |
| T5-85 | Project lower-right zoom-out control area | No tooltip | No tooltip after 2.5 s at actual (1813,570); no zoom change. | `t2-085-project-zoom-out-control-0.jpg` | Verified absence |
| T5-86 | Project lower-right zoom-in control area | No tooltip | No tooltip after 2.5 s at actual (1863,570); no zoom change. | `t2-086-project-zoom-in-control-0.jpg` | Verified absence |
| T4-87 | Audio 01 track name label | No tooltip | No tooltip after 2.5 s at actual (283,187); no edit. | `t2-087-audio01-track-name-0.jpg` | Verified absence |
| T4-88 | Audio 01 stereo type label | No tooltip | No tooltip after 2.5 s at actual (231,202); no edit. | `t2-088-audio01-type-label-0.jpg` | Verified absence |
| T4-89 | Audio 02 track name label | No tooltip | No tooltip after 2.5 s at actual (283,227); no edit. | `t2-089-audio02-track-name-0.jpg` | Verified absence |
| T4-90 | Audio 02 mono type label | No tooltip | No tooltip after 2.5 s at actual (231,245); no edit. | `t2-090-audio02-type-label-0.jpg` | Verified absence |

 Event Info Line` | Verified. | `t2-005-project-show-event-info-line-0.jpg` | Verified |
| T2-06 | Show Overview | `Show Overview` | Verified. | `t2-006-project-show-overview-0.jpg` | Verified |
| T2-07 | Open Pool | `Open Pool` | Verified. | `t2-007-project-open-pool-0.jpg` | Verified |
| T2-08 | Open Mixer | `Open Mixer` | Verified. | `t2-008-project-open-mixer-0.jpg` | Verified |
| T2-09 | Show Automation Panel | `Show Automation Panel` | Verified. | `t2-009-project-show-automation-panel-0.jpg` | Verified |
| T2-10 | Global Automation Mode | `Global Automation Mode` | Verified; displayed mode Touch. | `t2-010-project-global-automation-mode-0.jpg` | Verified |
| T2-11 | Goto Previous Marker / Zero | `Goto Previous Marker / Zero` | Verified. | `t2-011-project-transport-previous-marker-zero-0.jpg` | Verified |
| T2-12 | Goto Next Marker | `Goto Next Marker` | Verified. | `t2-012-project-transport-next-marker-0.jpg` | Verified |
| T2-13 | Transport Cycle On/Off | `Transport Cycle On/Off` | Verified. | `t2-013-project-transport-cycle-0.jpg` | Verified |
| T2-14 | Transport Stop | `Transport Stop` | Verified. | `t2-014-project-transport-stop-0.jpg` | Verified |
| T2-15 | Transport Play | `Transport Play` | Verified. | `t2-015-project-transport-play-0.jpg` | Verified |
| T2-16 | Transport Record | `Transport Record` | Verified. | `t2-016-project-transport-record-0.jpg` | Verified |
| T2-17 | Object Selection | `Object Selection` | Verified. | `t2-017-project-object-selection-0.jpg` | Verified |
| T2-18 | Range Selection | `Range Selection` | Verified. | `t2-018-project-range-selection-0.jpg` | Verified |
| T2-19 | Split | `Split` | Verified. | `t2-019-project-split-tool-0.jpg` | Verified |
| T2-20 | Glue | `Glue` | Verified. | `t2-020-project-glue-tool-0.jpg` | Verified |
| T2-21 | Erase | `Erase` | Verified. | `t2-021-project-erase-tool-0.jpg` | Verified |
| T2-22 | Zoom | `Zoom` | Verified. | `t2-022-project-zoom-tool-0.jpg` | Verified |
| T2-23 | Mute tool | `Mute` | Verified. | `t2-023-project-mute-tool-0.jpg` | Verified |
| T2-24 | Time Warp | `Time Warp` | Verified. | `t2-024-project-time-warp-tool-0.jpg` | Verified |
| T2-25 | Draw | `Draw` | Verified. | `t2-025-project-draw-tool-0.jpg` | Verified |
| T2-26 | Line | `Line` | Verified. | `t2-026-project-line-tool-0.jpg` | Verified |
| T2-27 | Play tool | `Play` | Verified; distinct from Transport Play. | `t2-027-project-play-tool-0.jpg` | Verified |
| T2-28 | Color Tool | `Color Tool` | Verified. | `t2-028-project-color-tool-0.jpg` | Verified |
| T2-29 | Project Root Key | `Project Root Key` | Verified. | `t2-029-project-root-key-0.jpg` | Verified |
| T2-30 | Autoscroll | `Autoscroll` | Verified. | `t2-030-project-autoscroll-0.jpg` | Verified |
| T2-31 | Snap On/Off | `Snap On/Off` | Verified. | `t2-031-project-snap-toggle-0.jpg` | Verified |
| T2-32 | Snap Type | `Snap Type` | Verified. | `t2-032-project-snap-type-0.jpg` | Verified |
| T2-33 | Grid Type | `Grid Type` | Verified; displayed Bar. | `t2-033-project-grid-type-0.jpg` | Verified |
| T2-34 | Quantize Type | `Quantize Type` | Verified; displayed 1/16. | `t2-034-project-quantize-type-0.jpg` | Verified |
| T2-35 | Snap to Zero Crossing | `Snap to Zero Crossing` | Verified; capture restored after filename collision and retaken at x=1224. | `t2-035-project-snap-zero-crossing-0.jpg` | Verified |
| T2-36 | Color Selector | `Color Selector` | Verified at actual (1265,101). | `t2-036-project-color-selector-0.jpg` | Verified |
| T5-37 | Project info line (No Object Selected) | No tooltip | No tooltip after 2.5 s at actual (55,136). | `t2-037-project-info-line-0.jpg` | Verified absence |
| T5-38 | Timeline ruler near bar 2 | No tooltip | No tooltip after 2.5 s at actual (400,164). | `t2-038-project-ruler-0.jpg` | Verified absence |
| T3-39 | Audio 02 Inspector Edit Channel Settings | `Edit Channel Settings` | Verified. | `t2-039-inspector-edit-channel-settings-0.jpg` | Verified |
| T4-40 | Track list Track Controls Settings | `Track Controls Settings` | Verified. | `t2-040-track-list-controls-settings-0.jpg` | Verified |
| T4-41 | Track list In-place Editor toggle | `Toggle In-place Editor for Selected Tracks` | Verified. | `t2-041-track-list-in-place-editor-0.jpg` | Verified |
| T4-42 | Track list Unmute All | `Unmute All` | Verified. | `t2-042-track-list-unmute-all-0.jpg` | Verified |
| T4-43 | Track list Deactivate All Solo | `Deactivate All Solo` | Verified. | `t2-043-track-list-deactivate-all-solo-0.jpg` | Verified |
| T4-44 | Track list global automation read status | `All Automation to Read Status` | Verified. | `t2-044-track-list-all-automation-read-0.jpg` | Verified |
| T4-45 | Track list global automation write status | `All Automation to Write Status` | Verified. | `t2-045-track-list-all-automation-write-0.jpg` | Verified |
| T4-46 | Track list Show/Hide Track Colors | `Show/Hide Track Colors` | Verified. | `t2-046-track-list-show-hide-track-colors-0.jpg` | Verified |
| T4-47 | Audio 01 Mute | `Mute` | Verified. | `t2-047-audio01-track-mute-0.jpg` | Verified |
| T4-48 | Audio 01 Solo | `Solo State (Solo Defeat with ALT click)` | Verified; no click sent. | `t2-048-audio01-track-solo-0.jpg` | Verified |
| T4-49 | Audio 01 Read Enable | `Read Enable` | Verified. | `t2-049-audio01-read-enable-0.jpg` | Verified |
| T4-50 | Audio 01 Write Enable | `Write Enable` | Verified. | `t2-050-audio01-write-enable-0.jpg` | Verified |
| T4-51 | Audio 01 Record Enable | `Record Enable` | Verified. | `t2-051-audio01-record-enable-0.jpg` | Verified |
| T4-52 | Audio 01 Monitor | `Monitor` | Verified. | `t2-052-audio01-monitor-0.jpg` | Verified |
| T4-53 | Audio 02 Mute | `Mute` | Verified. | `t2-053-audio02-track-mute-0.jpg` | Verified |
| T4-54 | Audio 02 Solo | `Solo State (Solo Defeat with ALT click)` | Verified; no click sent. | `t2-054-audio02-track-solo-0.jpg` | Verified |
| T4-55 | Audio 02 Record Enable | `Record Enable` | Verified. | `t2-055-audio02-record-enable-0.jpg` | Verified |
| T4-56 | Audio 02 Monitor | `Monitor` | Verified. | `t2-056-audio02-monitor-0.jpg` | Verified |
| T4-57 | Audio 02 Read Enable | `Read Enable` | Verified. | `t2-057-audio02-read-enable-0.jpg` | Verified |
| T4-58 | Audio 02 Write Enable | `Write Enable` | Verified. | `t2-058-audio02-write-enable-0.jpg` | Verified |
| T5-59 | Empty Audio 01 timeline lane near bar 13 | No tooltip | No tooltip after 2.5 s at actual (800,190). | `t2-059-project-empty-track-lane-0.jpg` | Verified absence |
| T3-60 | Audio 02 Inspector Mute | `Mute` | Verified. | `t2-060-inspector-audio02-mute-0.jpg` | Verified |
| T3-61 | Audio 02 Inspector Solo | `Solo State (Solo Defeat with ALT click)` | Verified; no click sent. | `t2-061-inspector-audio02-solo-0.jpg` | Verified |
| T3-62 | Audio 02 Inspector Read Enable | `Read Enable` | Verified. | `t2-062-inspector-audio02-read-enable-0.jpg` | Verified |
| T3-63 | Audio 02 Inspector Write Enable | `Write Enable` | Verified. | `t2-063-inspector-audio02-write-enable-0.jpg` | Verified |
| T3-64 | Audio 02 Inspector Open Device Panels | `Open Device Panels` | Verified. | `t2-064-inspector-open-device-panels-0.jpg` | Verified |
| T3-65 | Audio 02 Inspector Auto Fades Settings | `Auto Fades Settings...` | Verified. | `t2-065-inspector-auto-fades-settings-0.jpg` | Verified |
| T3-66 | Audio 02 Inspector Enable Record | `Enable Record` | Verified; actual x=38 for requested x=39. | `t2-066-inspector-enable-record-0.jpg` | Verified |
| T3-67 | Audio 02 Inspector Timebase toggle | `Toggle Timebase between Musical and Linear` | Verified. | `t2-067-inspector-timebase-toggle-0.jpg` | Verified |
| T3-68 | Audio 02 Inspector Lock | `Lock` | Verified. | `t2-068-inspector-lock-0.jpg` | Verified |
| T3-69 | Audio 02 Inspector Lane Display Type | `Lane Display Type` | Verified. | `t2-069-inspector-lane-display-type-0.jpg` | Verified |
| T3-70 | Audio 02 Inspector Freeze Audio Channel | `Freeze Audio Channel` | Verified. | `t2-070-inspector-freeze-audio-channel-0.jpg` | Verified |
| T3-71 | Audio 02 Inspector Volume | `Volume` | Verified; hover only, no value change. | `t2-071-inspector-volume-0.jpg` | Verified |
| T3-72 | Audio 02 Inspector Pan | `Pan` | Verified; hover only, no value change. | `t2-072-inspector-pan-0.jpg` | Verified |
| T3-73 | Audio 02 Inspector Input Routing | `Input Routing (all selected with Shift + ALT + click)` | Verified; no click or routing change. | `t2-073-inspector-input-routing-0.jpg` | Verified |
| T3-74 | Audio 02 Inspector Output Routing | `Output Routing (all selected with Shift + ALT + click)` | Verified; no click or routing change. | `t2-074-inspector-output-routing-0.jpg` | Verified |
| T3-75 | Audio 02 Inspector Apply Track Preset | `Apply Track Preset` | Verified; no preset change. | `t2-075-inspector-apply-track-preset-0.jpg` | Verified |
| T3-76 | Audio 02 Inspector Inserts visibility | `Shows Active Inserts` | Verified; section not expanded. | `t2-076-inspector-shows-active-inserts-0.jpg` | Verified |
| T3-77 | Audio 02 Inspector Equalizers visibility | `Shows Active Equalizers` | Verified; section not expanded. | `t2-077-inspector-shows-active-equalizers-0.jpg` | Verified |
| T3-78 | Audio 02 Inspector Sends visibility | `Shows Active Sends` | Verified; section not expanded. | `t2-078-inspector-shows-active-sends-0.jpg` | Verified |
| T3-79 | Audio 02 Inspector Notepad | `Notepad is empty` | Verified; no note edit. | `t2-079-inspector-notepad-empty-0.jpg` | Verified |
| T3-80 | Audio 02 Inspector Quick Controls visibility | `Shows Active Quick Controls` | Verified; section not expanded. | `t2-080-inspector-shows-active-quick-controls-0.jpg` | Verified |

## Earlier tooltip notes now captured

These values were first noted during L1 exploration and are now represented in the ordered Project, Mixer, and Channel Settings captures below:

- Inspector/Mixer record control: `Record Enable` (`73-audio02-record-disarmed` capture).
- Mixer channel-settings control: `Edit Audio Channel Settings` (`57-mixer-edit-channel-settings-tooltip` capture).
- Channel-strip input gain: `Input Gain (use shift)` (Channel Settings/routing exploration).

## Review landmarks

1. Project-window first pass is captured through observation 90. Review the full-screen captures and unresolved x=80 toolbar probe before continuing to expanded Inspector contents and the remaining timeline details.
2. Mixer first pass is captured through t3-097. The separate Transport Panel is captured through t4-031, and VST Audio Channel Settings through t4-081. The L1 dropdown inventory is canonical; t4-001 is a redundant duplicate. Review this window-capture milestone before the Compressor editor pass.

Screenshots and manifests should be stored outside the product source under `../../../_analysis-tools/reports/workflow-L2/tooltips/`. Keep this document as the searchable verbatim text ledger; do not embed copied reference artwork into product assets.


## Mixer window pass (capture prefix t3; 2026-10-22)

Raised the Mixer child window in its existing desktop layout and preserved the visible workspace. The user clarified that Audio 01 and Audio 02 were created before this review as reference tracks, not default session tracks. Audio 02 has the example compressor in Insert 1; inserts are otherwise empty until explicitly loaded.

The visible strips are Stereo In, Audio 01 (stereo), Audio 02 (mono), and Stereo Out. Audio strips expose input phase/gain, routing, pan, channel level, fader, Mute/Solo/Listen, automation Read/Write, channel settings, Inserts/EQs/Sends state, Monitor, and Record Enable. Stereo In presents input controls, Mute/Listen, automation, and Edit Input Channel Settings; no Solo or Record Enable was visible there. Audio 02 Record Enable was already active/red and was left unchanged. Stereo Out exposes Edit Output Channel Settings; no tooltip appeared at the sampled Sends, Monitor, and Record Enable positions. Pan readouts, no-panner labels, and fader handles sampled here produced no tooltip.

The Mixer sidebar includes global Mute/Solo/Listen, Read/Write All Automation, Reset Mixer/Reset Channels, copy/paste channel settings, Show VST Connections, Hide Channels/Set Target Channels modes, channel-type visibility filters, and Reveal All Channels. Reset and all state-changing controls were hovered only; none were activated. Input/output routing tooltips advertise Shift + ALT + click for applying routes to selected channels. Input Gain knobs advertise Shift for finer control; Solo State advertises ALT-click for Solo Defeat. Inserts, EQs, and Sends state tooltips describe bypass by clicking and offer context-menu usage hints where present.

| Capture (-0.jpg) | Target | Tooltip | Notes |
|---|---|---|---|
| t3-001-mixer-window-raised-0.jpg | Mixer window raised in its existing layout | (none) | Raised the existing Mixer child window using its accessibility Raise action; no resize, move, click, key, or scroll. Audio 01 and Audio 02 are user-created analysis/reference tracks. |
| t3-002-mixer-hide-routing-0.jpg | Mixer left toolbar Hide Routing control | Hide Routing | Hover only; no click, key, or scroll. |
| t3-003-mixer-command-target-selected-only-0.jpg | Mixer left toolbar Command Target control | Command Target: Selected Only | Displayed mode Selected Only; hover only. |
| t3-004-mixer-audio01-input-routing-0.jpg | Mixer Audio 01 channel Input Routing field | Input Routing (all selected with Shift + ALT + click) | Hover only; no routing change. |
| t3-005-mixer-audio01-output-routing-0.jpg | Mixer Audio 01 channel Output Routing field | Output Routing (all selected with Shift + ALT + click) | Hover only; no routing change. |
| t3-006-mixer-audio02-input-routing-0.jpg | Mixer Audio 02 channel Input Routing field | Input Routing (all selected with Shift + ALT + click) | Hover only; no routing change. |
| t3-007-mixer-audio02-output-routing-0.jpg | Mixer Audio 02 channel Output Routing field | Output Routing (all selected with Shift + ALT + click) | Hover only; no routing change. |
| t3-008-mixer-audio01-type-label-0.jpg | Mixer Audio 01 stereo type label | (none) | No tooltip after 2.5 s; no click or edit. |
| t3-009-mixer-audio02-type-label-0.jpg | Mixer Audio 02 mono type label | (none) | No tooltip after 2.5 s; no click or edit. |
| t3-010-mixer-stereo-in-input-phase-0.jpg | Mixer Stereo In channel Input Phase control | Input Phase | Hover only; no phase change. |
| t3-011-mixer-stereo-in-input-gain-0.jpg | Mixer Stereo In channel Input Gain field | Input Gain | Hover only; no gain change. |
| t3-012-mixer-stereo-in-input-gain-shift-0.jpg | Mixer Stereo In channel Input Gain knob | Input Gain (use shift) | Tooltip advertises Shift for finer adjustment; hover only, no gain change. |
| t3-013-mixer-audio01-pan-no-tooltip-0.jpg | Mixer Audio 01 pan readout | (none) | No tooltip appeared on the pan value; hover only, no pan change. |
| t3-014-mixer-stereo-in-pan-label-no-tooltip-0.jpg | Mixer Stereo In no panner label | (none) | No tooltip appeared over the no panner label; hover only, no value change. |
| t3-015-mixer-stereo-in-volume-fader-no-tooltip-0.jpg | Mixer Stereo In volume fader handle | (none) | No tooltip appeared on the fader handle; hover only, no level change. |
| t3-016-mixer-stereo-in-mute-0.jpg | Mixer Stereo In Mute button | Mute | Hover only; no mute toggle. |
| t3-017-mixer-stereo-in-solo-row-no-tooltip-0.jpg | Mixer Stereo In position between Mute and Listen controls | (none) | No Solo button or tooltip is visible for Stereo In; hover only, no control change. |
| t3-018-mixer-stereo-in-listen-0.jpg | Mixer Stereo In Listen button | Listen | Hover only; listen state not toggled. |
| t3-019-mixer-stereo-in-read-automation-0.jpg | Mixer Stereo In Read Automation button | Read Automation | Hover only; automation read state unchanged. |
| t3-020-mixer-stereo-in-write-automation-0.jpg | Mixer Stereo In Write Automation button | Write Automation | Hover only; automation write state unchanged. |
| t3-021-mixer-stereo-in-channel-settings-0.jpg | Mixer Stereo In Edit Input Channel Settings button | Edit Input Channel Settings | Hover only; channel settings window not opened. |
| t3-022-mixer-audio01-mute-0.jpg | Mixer Audio 01 Mute button | Mute | Hover only; mute state unchanged. |
| t3-023-mixer-audio01-solo-0.jpg | Mixer Audio 01 Solo button | Solo State (Solo Defeat with ALT click) | Tooltip names Solo State and advertises ALT-click for Solo Defeat; hover only, no state change. |
| t3-024-mixer-audio01-listen-0.jpg | Mixer Audio 01 Listen button | Listen | Hover only; listen state unchanged. |
| t3-025-mixer-audio01-read-automation-0.jpg | Mixer Audio 01 Read Automation button | Read Automation | The R in this mid-strip row is Read Automation, not Record Enable; hover only. |
| t3-026-mixer-audio01-write-automation-0.jpg | Mixer Audio 01 Write Automation button | Write Automation | Hover only; automation write state unchanged. |
| t3-027-mixer-audio01-channel-settings-0.jpg | Mixer Audio 01 Edit Audio Channel Settings button | Edit Audio Channel Settings | Hover only; channel settings window not opened. |
| t3-028-mixer-audio01-inserts-state-0.jpg | Mixer Audio 01 Inserts State button | Inserts State (Bypass Inserts with click / Context-menu shows usage) | Inserts are empty on Audio 01; hovering exposes the bypass state/shortcut hint. No click was made. |
| t3-029-mixer-audio01-eq-state-0.jpg | Mixer Audio 01 EQs State button | EQs State (Bypass EQs with click) | No EQ bands are set on Audio 01; hovering shows EQ bypass behavior. No click was made. |
| t3-030-mixer-audio01-sends-state-0.jpg | Mixer Audio 01 Sends State button | Sends State (Bypass Sends with click / Context-menu shows usage) | No sends are configured on Audio 01; hovering shows bypass behavior. No click was made. |
| t3-031-mixer-audio01-monitor-0.jpg | Mixer Audio 01 Monitor button | Monitor | Hover only; monitor state unchanged. |
| t3-032-mixer-audio01-record-enable-0.jpg | Mixer Audio 01 Record Enable button | Record Enable | Hover only; the record-enable state was not changed. |
| t3-033-mixer-audio01-channel-level-0.jpg | Mixer Audio 01 Channel Level readout | Channel Level | Hover only; channel level unchanged. |
| t3-034-mixer-audio01-input-phase-0.jpg | Mixer Audio 01 Input Phase control | Input Phase | Hover only; input phase unchanged. |
| t3-035-mixer-audio01-input-gain-0.jpg | Mixer Audio 01 Input Gain value | Input Gain | Hover only; input gain unchanged. |
| t3-036-mixer-audio01-input-gain-shift-0.jpg | Mixer Audio 01 Input Gain knob | Input Gain (use shift) | Tooltip advertises Shift for finer adjustment; hover only, no gain change. |
| t3-037-mixer-audio02-input-phase-0.jpg | Mixer Audio 02 Input Phase control | Input Phase | Hover only; input phase unchanged. |
| t3-038-mixer-audio02-input-gain-0.jpg | Mixer Audio 02 Input Gain value | Input Gain | Hover only; input gain unchanged. |
| t3-039-mixer-audio02-input-gain-shift-0.jpg | Mixer Audio 02 Input Gain knob | Input Gain (use shift) | Tooltip advertises Shift for finer adjustment; hover only, no gain change. |
| t3-040-mixer-audio02-pan-no-tooltip-0.jpg | Mixer Audio 02 pan readout | (none) | No tooltip appeared on the pan value; hover only, no pan change. |
| t3-041-mixer-audio02-mute-0.jpg | Mixer Audio 02 Mute button | Mute | Hover only; no button or automation state change. |
| t3-042-mixer-audio02-solo-0.jpg | Mixer Audio 02 Solo button | Solo State (Solo Defeat with ALT click) | Hover only; no button or automation state change. |
| t3-043-mixer-audio02-listen-0.jpg | Mixer Audio 02 Listen button | Listen | Hover only; no button or automation state change. |
| t3-044-mixer-audio02-read-automation-0.jpg | Mixer Audio 02 Read Automation button | Read Automation | Hover only; no button or automation state change. |
| t3-045-mixer-audio02-write-automation-0.jpg | Mixer Audio 02 Write Automation button | Write Automation | Hover only; no button or automation state change. |
| t3-046-mixer-audio02-channel-settings-0.jpg | Mixer Audio 02 Edit Audio Channel Settings button | Edit Audio Channel Settings | Hover only; channel settings window not opened. |
| t3-047-mixer-audio02-inserts-state-0.jpg | Mixer Audio 02 Inserts State button | Inserts State (Bypass Inserts with click / Context-menu shows usage) | Audio 02 has the example compressor loaded; state was only hovered, not bypassed or opened. |
| t3-048-mixer-audio02-eq-state-0.jpg | Mixer Audio 02 EQs State button | EQs State (Bypass EQs with click) | No EQ bands are set on Audio 02; hover only, no bypass. |
| t3-049-mixer-audio02-sends-state-0.jpg | Mixer Audio 02 Sends State button | Sends State (Bypass Sends with click / Context-menu shows usage) | No sends are configured on Audio 02; hover only, no bypass. |
| t3-050-mixer-audio02-monitor-0.jpg | Mixer Audio 02 Monitor button | Monitor | Hover only; monitor state unchanged. |
| t3-051-mixer-audio02-record-enable-0.jpg | Mixer Audio 02 Record Enable button | Record Enable | Record Enable is visibly active/red in this reference state; hover only, no toggle. |
| t3-052-mixer-audio02-channel-level-0.jpg | Mixer Audio 02 Channel Level readout | Channel Level | Hover only; channel level unchanged. |
| t3-053-mixer-stereo-out-input-phase-0.jpg | Mixer Stereo Out Input Phase control | Input Phase | Hover only; input phase unchanged. |
| t3-054-mixer-stereo-out-input-gain-0.jpg | Mixer Stereo Out Input Gain value | Input Gain | Hover only; input gain unchanged. |
| t3-055-mixer-stereo-out-input-gain-shift-0.jpg | Mixer Stereo Out Input Gain knob | Input Gain (use shift) | Tooltip advertises Shift for finer adjustment; hover only, no gain change. |
| t3-056-mixer-stereo-out-no-panner-0.jpg | Mixer Stereo Out no panner label | (none) | No tooltip appeared over the no panner label; hover only. |
| t3-057-mixer-stereo-out-mute-0.jpg | Mixer Stereo Out Mute button | Mute | Hover only; mute state unchanged. |
| t3-058-mixer-stereo-out-solo-0.jpg | Mixer Stereo Out Solo button | Solo State (Solo Defeat with ALT click) | Hover only; no button or automation state change. |
| t3-059-mixer-stereo-out-listen-0.jpg | Mixer Stereo Out Listen button | Listen | Hover only; no button or automation state change. |
| t3-060-mixer-stereo-out-read-automation-0.jpg | Mixer Stereo Out Read Automation button | Read Automation | Hover only; no button or automation state change. |
| t3-061-mixer-stereo-out-write-automation-0.jpg | Mixer Stereo Out Write Automation button | Write Automation | Hover only; no button or automation state change. |
| t3-062-mixer-stereo-out-channel-settings-0.jpg | Mixer Stereo Out Edit Audio Channel Settings button | Edit Output Channel Settings | Hover only; no button or automation state change. |
| t3-063-mixer-stereo-out-inserts-state-0.jpg | Mixer Stereo Out Inserts State button | Inserts State (Bypass Inserts with click / Context-menu shows usage) | Hover only; output inserts state unchanged. |
| t3-064-mixer-stereo-out-eq-state-0.jpg | Mixer Stereo Out EQs State button | EQs State (Bypass EQs with click) | Hover only; output EQ state unchanged. |
| t3-065-mixer-stereo-out-sends-state-0.jpg | Mixer Stereo Out position for Sends State | (none) | No tooltip appeared at the Sends State position; hover only, no state change. |
| t3-066-mixer-stereo-out-no-monitor-control-0.jpg | Mixer Stereo Out position for Monitor control | (none) | No tooltip or Monitor control is visible in the output strip at the track Monitor position; hover only. |
| t3-067-mixer-stereo-out-no-record-enable-0.jpg | Mixer Stereo Out position for Record Enable control | (none) | No Record Enable button or tooltip is visible for Stereo Out; hover only. |
| t3-068-mixer-stereo-out-channel-level-0.jpg | Mixer Stereo Out Channel Level readout | Channel Level | Hover only; output level unchanged. |
| t3-069-mixer-stereo-out-volume-fader-no-tooltip-0.jpg | Mixer Stereo Out volume fader handle | (none) | No tooltip appeared on the output fader handle; hover only, no level change. |
| t3-070-mixer-sidebar-mute-all-0.jpg | Mixer sidebar Mute Active/Deactivate All Mute control | Mute Active/Deactivate All Mute | Hover only; global mute state unchanged. |
| t3-071-mixer-sidebar-solo-all-0.jpg | Mixer sidebar Solo Active/Deactivate All Solo control | Solo Active/Deactivate All Solo | Hover only; global solo state unchanged. |
| t3-072-mixer-sidebar-listen-all-0.jpg | Mixer sidebar Listen Active/Deactivate All Listen control | Listen Active/Deactivate All Listen | Hover only; global listen state unchanged. |
| t3-073-mixer-sidebar-read-all-automation-0.jpg | Mixer sidebar Read All Automation control | Read All Automation | Hover only; no automation playback state changed. |
| t3-074-mixer-sidebar-write-all-automation-0.jpg | Mixer sidebar Write All Automation control | Write All Automation | Hover only; no automation write state changed. |
| t3-075-mixer-sidebar-y844-no-tooltip-0.jpg | Mixer sidebar icon below Write All Automation | (none) | No tooltip appeared at this sidebar position; hover only, no state change. |
| t3-076-mixer-sidebar-reset-mixer-channels-0.jpg | Mixer sidebar Reset Mixer/Reset Channels control | Reset Mixer/Reset Channels | Destructive reset function identified by hover only; it was not activated. |
| t3-077-mixer-sidebar-y881-no-tooltip-0.jpg | Mixer sidebar icon below Reset Mixer | (none) | No tooltip appeared at this sidebar position; hover only, no state change. |
| t3-078-mixer-sidebar-copy-first-selected-channel-settings-0.jpg | Mixer sidebar Copy First Selected Channel's Settings | Copy First Selected Channel's Settings | Hover only; no settings were copied. |
| t3-079-mixer-sidebar-paste-settings-to-selected-channels-0.jpg | Mixer sidebar Paste Settings to Selected Channels | Paste Settings to Selected Channels | Hover only; settings were not pasted. |
| t3-080-mixer-sidebar-y935-no-tooltip-0.jpg | Mixer sidebar icon at y=935 | (none) | Hover only; no state change. |
| t3-081-mixer-sidebar-y953-no-tooltip-0.jpg | Mixer sidebar icon at y=953 | (none) | Hover only; no state change. |
| t3-082-mixer-sidebar-show-vst-connections-0.jpg | Mixer sidebar Show VST Connections | Show VST Connections | Hover only; VST Connections was not opened. |
| t3-083-mixer-sidebar-y989-no-tooltip-0.jpg | Mixer sidebar icon at y=989 | (none) | Hover only; no state change. |
| t3-084-mixer-sidebar-hide-channels-0.jpg | Mixer sidebar Hide Channels mode control | Hide Channels set to 'Can Hide' | Hover only; hide mode unchanged. |
| t3-085-mixer-sidebar-target-channels-can-hide-0.jpg | Mixer sidebar Set Target Channels mode control | Set Target Channels to 'Can Hide' | Hover only; target channel mode unchanged. |
| t3-086-mixer-sidebar-second-column-y785-no-tooltip-0.jpg | Mixer sidebar position below Set Target Channels | (none) | No tooltip appeared at this sidebar position; hover only, no state change. |
| t3-087-mixer-sidebar-hide-input-channels-0.jpg | Mixer sidebar Hide Input Channels control | Hide Input Channels | Hover only; channel visibility unchanged. |
| t3-088-mixer-sidebar-hide-audio-channels-0.jpg | Mixer sidebar Hide Audio Channels control | Hide Audio Channels | Hover only; channel visibility unchanged. |
| t3-089-mixer-sidebar-hide-group-channels-0.jpg | Mixer sidebar Hide Group Channels control | Hide Group Channels | Hover only; channel visibility unchanged. |
| t3-090-mixer-sidebar-hide-rewire-channels-0.jpg | Mixer sidebar Hide ReWire Channels control | Hide ReWire Channels | Hover only; channel visibility unchanged. |
| t3-091-mixer-sidebar-y881-0.jpg | Mixer sidebar icon at y=881 | Hide MIDI Channels | Hover only; channel visibility or mixer state unchanged. |
| t3-092-mixer-sidebar-y897-0.jpg | Mixer sidebar icon at y=897 | Hide MIDI Channels | Hover only; channel visibility or mixer state unchanged. |
| t3-093-mixer-sidebar-y916-0.jpg | Mixer sidebar icon at y=916 | Hide VST Instrument Channels | Hover only; channel visibility or mixer state unchanged. |
| t3-094-mixer-sidebar-y935-0.jpg | Mixer sidebar icon at y=935 | Hide FX Channels | Hover only; channel visibility or mixer state unchanged. |
| t3-095-mixer-sidebar-y953-0.jpg | Mixer sidebar icon at y=953 | Hide Output Channels | Hover only; channel visibility or mixer state unchanged. |
| t3-096-mixer-sidebar-y971-0.jpg | Mixer sidebar icon at y=971 | Reveal All Channels | Hover only; channel visibility or mixer state unchanged. |
| t3-097-mixer-sidebar-y989-no-tooltip-0.jpg | Mixer sidebar icon at y=989 | (none) | Hover only; channel visibility or mixer state unchanged. |

## Transport menu re-capture (t4-001; redundant; 2026-10-22)

The Transport dropdown was already inventoried in [L1](L1-menu-and-project-capture.md). This later capture duplicates that menu pass and is retained only as a visual cross-check; it is not new coverage. The separate floating Transport Panel is documented below.

Capture: [t4-001-transport-menu-full-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-001-transport-menu-full-0.jpg).

## Transport Panel and Channel Settings pass (capture prefix t4; 2026-10-22)

### Separate floating Transport Panel (t4-002–031)

This pass covers the independent Transport Panel, not the application Transport dropdown. The panel remained in its existing position. It showed the transport stopped at `1.1.1.0`, time `0:00:00:000`, tempo `120.000` in Tempo Track mode, 4/4, and Sync `INT.` / `Offline`. Captures cover recording modes, MIDI record quantize, navigation, Cycle, Play/Stop/Record, Click, tempo, time signature, sync, markers, locators, and position/time displays. No transport command was activated.

### VST Audio Channel Settings - Audio 02 (t4-032–081)

The overview shows 8 insert slots, 8 send slots, the four-band EQ, and the channel strip. The user's Compressor is only an example in Insert 1; Inserts 2–8 and all Sends are empty. The EQ graph is flat, with displayed band values: 100.0 Hz / 0.0 dB / Q 7.0; 800.0 Hz / 0.0 dB / Q 7.0; 2000.0 Hz / 0.0 dB / Q 0.2; and 12000 Hz / 0.0 dB / Q 7.0. The strip routes from Left - Stereo In to Stereo Out, is mono, has a centered pan display, and shows Channel Level 0.00. Record Enable was already active/red and remains unchanged.

The pass used hover input only for controls. No insert, send, EQ, routing, fader, mute, solo, listen, automation, or record state was changed. Screenshots were used to resolve several accessibility-tree labels that lagged behind the pointer; corrected targets and tooltip text are stored in the manifests.

| Capture | Target | Tooltip | Notes |
|---|---|---|---|
| [t4-002-transport-panel-overview-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-002-transport-panel-overview-0.jpg) | Floating Transport Panel in its existing Mixer workspace position | No tooltip | Full-screen overview; pointer was resting on the Normal record-mode control. No command or setting was activated. This is the Transport Panel, not the application Transport dropdown. |
| [t4-003-transport-panel-stop-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-003-transport-panel-stop-0.jpg) | Transport Panel Stop button | `Stop` | Hover only. The panel was not clicked; the transport remained stopped at 1.1.1.0 and 0:00:00:000. |
| [t4-004-transport-panel-close-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-004-transport-panel-close-0.jpg) | Transport Panel close control | No tooltip | Hover only; no transport control activated or value changed. |
| [t4-005-transport-panel-normal-mode-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-005-transport-panel-normal-mode-0.jpg) | Transport Panel record mode selector showing Normal | `Linear Record Mode` | Hover only; no transport control activated or value changed. |
| [t4-006-transport-panel-mix-midi-mode-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-006-transport-panel-mix-midi-mode-0.jpg) | Transport Panel Mix (MIDI) selector | `Cycle Record Mode` | Hover only; no transport control activated or value changed. |
| [t4-007-transport-panel-auto-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-007-transport-panel-auto-0.jpg) | Transport Panel AUTO control | `Automatic MIDI Record Quantize` | Hover only; no transport control activated or value changed. |
| [t4-008-transport-panel-auto-off-state-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-008-transport-panel-auto-off-state-0.jpg) | Transport Panel automation Off state | `Automatic MIDI Record Quantize` | Hover only; no transport control activated or value changed. |
| [t4-009-transport-panel-go-to-left-locator-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-009-transport-panel-go-to-left-locator-0.jpg) | Transport Panel go-to-left-locator button | `Goto Previous Marker / Zero` | Hover only. No transport command activated; project position and transport state left unchanged. |
| [t4-010-transport-panel-rewind-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-010-transport-panel-rewind-0.jpg) | Transport Panel rewind button | `Rewind` | Hover only. No transport command activated; project position and transport state left unchanged. |
| [t4-011-transport-panel-fast-forward-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-011-transport-panel-fast-forward-0.jpg) | Transport Panel fast-forward button | `Fast Forward` | Hover only. No transport command activated; project position and transport state left unchanged. |
| [t4-012-transport-panel-go-to-right-locator-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-012-transport-panel-go-to-right-locator-0.jpg) | Transport Panel go-to-right-locator button | `Goto Next Marker / Project End` | Hover only. No transport command activated; project position and transport state left unchanged. |
| [t4-013-transport-panel-cycle-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-013-transport-panel-cycle-0.jpg) | Transport Panel cycle button | `Cycle` | Hover only. No transport command activated; project position and transport state left unchanged. |
| [t4-014-transport-panel-play-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-014-transport-panel-play-0.jpg) | Transport Panel Play button | `Play` | Hover only. No transport command activated; project position and transport state left unchanged. |
| [t4-015-transport-panel-record-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-015-transport-panel-record-0.jpg) | Transport Panel Record button | `Record` | Hover only. No transport command activated; project position and transport state left unchanged. |
| [t4-016-transport-panel-click-toggle-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-016-transport-panel-click-toggle-0.jpg) | Transport Panel Click/metronome toggle | `Metronome/Click (use CTRL to open metronome settings)` | Hover only; no metronome, tempo, meter, or sync setting changed. |
| [t4-017-transport-panel-tempo-mode-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-017-transport-panel-tempo-mode-0.jpg) | Transport Panel tempo mode selector | `Activate Tempo Track (use CTRL to open project tempo track editor)` | Hover only; no metronome, tempo, meter, or sync setting changed. |
| [t4-018-transport-panel-tempo-value-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-018-transport-panel-tempo-value-0.jpg) | Transport Panel tempo value | `Current Tempo Display (display only because project's tempo track is active)` | Hover only; no metronome, tempo, meter, or sync setting changed. |
| [t4-019-transport-panel-time-signature-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-019-transport-panel-time-signature-0.jpg) | Transport Panel time signature display | `First Time Signature Value (always changes first time signature in project's tempo track)` | Hover only; no metronome, tempo, meter, or sync setting changed. |
| [t4-020-transport-panel-sync-mode-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-020-transport-panel-sync-mode-0.jpg) | Transport Panel sync mode selector | `Activate External Sync (use CTRL to open synchronisation settings)` | Hover only; no metronome, tempo, meter, or sync setting changed. |
| [t4-021-transport-panel-sync-status-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-021-transport-panel-sync-status-0.jpg) | Transport Panel sync status | `Current Synchronisation State` | Hover only; no metronome, tempo, meter, or sync setting changed. |
| [t4-022-transport-panel-show-markers-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-022-transport-panel-show-markers-0.jpg) | Transport Panel SHOW control | `Open Markers` | Hover only; no marker, locator, or position value changed. |
| [t4-023-transport-panel-marker-selector-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-023-transport-panel-marker-selector-0.jpg) | Transport Panel MARKER selector | No tooltip | Hover only; no marker, locator, or position value changed. |
| [t4-024-transport-panel-marker-slot-one-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-024-transport-panel-marker-slot-one-0.jpg) | Transport Panel marker slot 1 | `Jump to Marker 1 (use ALT to set)` | Hover only; no marker, locator, or position value changed. |
| [t4-025-transport-panel-left-locator-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-025-transport-panel-left-locator-0.jpg) | Transport Panel left locator field | `Left Locator Position` | Hover only; no marker, locator, or position value changed. |
| [t4-026-transport-panel-right-locator-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-026-transport-panel-right-locator-0.jpg) | Transport Panel right locator field | `Right Locator Position` | Hover only; no marker, locator, or position value changed. |
| [t4-027-transport-panel-position-display-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-027-transport-panel-position-display-0.jpg) | Transport Panel bar/beat position display | `Primary Time Display` | Hover only; no marker, locator, or position value changed. |
| [t4-028-transport-panel-time-display-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-028-transport-panel-time-display-0.jpg) | Transport Panel time display | No tooltip | Hover only; no marker, locator, or position value changed. |
| [t4-029-transport-panel-rewind-upper-edge-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-029-transport-panel-rewind-upper-edge-0.jpg) | Transport Panel rewind control (sample at upper edge) | `Rewind` | Hover only. The sampled point landed on Rewind; no transport command activated and the project position remained unchanged. |
| [t4-030-transport-panel-drag-handle-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-030-transport-panel-drag-handle-0.jpg) | Transport Panel left drag-handle area | No tooltip | Hover only; no drag or control activation. |
| [t4-031-transport-panel-right-meter-area-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-031-transport-panel-right-meter-area-0.jpg) | Transport Panel right-side meter area | No tooltip | Hover only; no drag or control activation. |
| [t4-032-vst-audio-channel-settings-audio02-overview-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-032-vst-audio-channel-settings-audio02-overview-0.jpg) | VST Audio Channel Settings window for Audio 02 | No tooltip | Full-screen overview of the existing Channel Settings layout. Audio 02 has the user-provided Compressor example in Insert 1, empty remaining insert and send slots, a flat four-band EQ, and a channel strip; no value or processor state changed. |
| [t4-033-channel-settings-inserts-header-button-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-033-channel-settings-inserts-header-button-0.jpg) | Channel Settings Inserts header button | `Shows Active Inserts` | Hover only; no insert, bypass, or plugin state changed. |
| [t4-034-channel-settings-insert1-bypass-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-034-channel-settings-insert1-bypass-0.jpg) | Channel Settings Insert 1 bypass control | `Deactivate Insert 1` | Hover only; no insert, bypass, or plugin state changed. |
| [t4-035-channel-settings-insert1-type-selector-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-035-channel-settings-insert1-type-selector-0.jpg) | Channel Settings Insert 1 type selector | `Select Insert Type 1` | Visible screenshot confirms Select Insert Type 1; the accessibility tree label was stale. Hover only; Compressor untouched. |
| [t4-036-channel-settings-insert1-slot-control-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-036-channel-settings-insert1-slot-control-0.jpg) | Channel Settings Insert 1 right-side slot control | `Select Insert Type 1` | Hover only; no insert, bypass, or plugin state changed. |
| [t4-037-channel-settings-insert2-activate-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-037-channel-settings-insert2-activate-0.jpg) | Channel Settings Insert 2 activation control | `Activate Insert 2` | Visible screenshot confirms Activate Insert 2; Insert 2 remains empty and inactive. |
| [t4-038-channel-settings-insert2-expand-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-038-channel-settings-insert2-expand-0.jpg) | Channel Settings empty Insert 2 expand control | `Activate Insert 2` | Hover only; no insert, bypass, or plugin state changed. |
| [t4-039-channel-settings-insert8-bypass-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-039-channel-settings-insert8-bypass-0.jpg) | Channel Settings empty Insert 8 bypass control | `Activate Insert 8` | Hover only; no insert, bypass, or plugin state changed. |
| [t4-040-channel-settings-insert8-expand-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-040-channel-settings-insert8-expand-0.jpg) | Channel Settings empty Insert 8 expand control | `Activate Insert 8` | Hover only; no insert, bypass, or plugin state changed. |
| [t4-041-channel-settings-equalizers-header-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-041-channel-settings-equalizers-header-0.jpg) | Channel Settings Equalizers section header | No tooltip | Hover only; EQ enable state and all band values unchanged. |
| [t4-042-channel-settings-eq-main-enable-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-042-channel-settings-eq-main-enable-0.jpg) | Channel Settings main Equalizers enable control | No tooltip | Hover only; EQ enable state and all band values unchanged. |
| [t4-043-channel-settings-eq-main-right-control-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-043-channel-settings-eq-main-right-control-0.jpg) | Channel Settings Equalizers right-side control | No tooltip | Hover only; EQ enable state and all band values unchanged. |
| [t4-044-channel-settings-eq1-band-enable-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-044-channel-settings-eq1-band-enable-0.jpg) | Channel Settings EQ Band 1 active control | `EQ Band 1 Active` | Visible screenshot confirms EQ Band 1 Active. Hover only; EQ remains unchanged. |
| [t4-045-channel-settings-eq1-band-label-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-045-channel-settings-eq1-band-label-0.jpg) | Channel Settings EQ Band 1 label | `EQ Band 1 Active` | Hover only; EQ enable state and all band values unchanged. |
| [t4-046-channel-settings-eq1-gain-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-046-channel-settings-eq1-gain-0.jpg) | Channel Settings EQ Band 1 Gain value | `EQ Band 1 Gain` | Hover only; EQ enable state and all band values unchanged. |
| [t4-047-channel-settings-eq1-frequency-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-047-channel-settings-eq1-frequency-0.jpg) | Channel Settings EQ Band 1 Frequency value | `EQ Band 1 Freq` | Initial accessibility tree repeated the previous Gain label; the visible hover tooltip was verified as Band 1 Freq in t4-049. |
| [t4-048-channel-settings-eq1-q-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-048-channel-settings-eq1-q-0.jpg) | Channel Settings EQ Band 1 Q value | `EQ Band 1 Q` | Hover only; EQ enable state and all band values unchanged. |
| [t4-049-channel-settings-eq1-frequency-verified-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-049-channel-settings-eq1-frequency-verified-0.jpg) | Channel Settings EQ Band 1 Frequency control | `EQ Band 1 Freq` | Hover only; value remains 100.0 Hz and no EQ parameter changed. |
| [t4-050-channel-settings-eq2-gain-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-050-channel-settings-eq2-gain-0.jpg) | Channel Settings EQ Band 2 Gain control | `EQ Band 2 Gain` | Hover only; EQ Band 2/3 enable state and values remain unchanged. |
| [t4-051-channel-settings-eq2-frequency-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-051-channel-settings-eq2-frequency-0.jpg) | Channel Settings EQ Band 2 Frequency control | `EQ Band 2 Freq` | Hover only; EQ Band 2/3 enable state and values remain unchanged. |
| [t4-052-channel-settings-eq2-q-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-052-channel-settings-eq2-q-0.jpg) | Channel Settings EQ Band 2 Q control | `EQ Band 2 Q` | Initial accessibility tree repeated the prior Freq label; the visible hover tooltip was verified as Band 2 Q in t4-056. |
| [t4-053-channel-settings-eq3-gain-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-053-channel-settings-eq3-gain-0.jpg) | Channel Settings EQ Band 3 Gain control | `EQ Band 3 Gain` | Hover only; EQ Band 2/3 enable state and values remain unchanged. |
| [t4-054-channel-settings-eq3-frequency-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-054-channel-settings-eq3-frequency-0.jpg) | Channel Settings EQ Band 3 frequency control | `EQ Band 3 Freq` | Visible screenshot confirms Band 3 Freq; earlier tree label was stale. Displayed value 2000.0 Hz remains unchanged. |
| [t4-055-channel-settings-eq3-q-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-055-channel-settings-eq3-q-0.jpg) | Channel Settings EQ Band 3 Q control | `EQ Band 3 Q` | Hover only; EQ Band 2/3 enable state and values remain unchanged. |
| [t4-056-channel-settings-eq2-q-verified-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-056-channel-settings-eq2-q-verified-0.jpg) | Channel Settings EQ Band 2 Q control | `EQ Band 2 Q` | Visible tooltip confirms Band 2 Q; no EQ parameter changed. |
| [t4-057-vst-audio-channel-settings-eq-band3-freq-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-057-vst-audio-channel-settings-eq-band3-freq-0.jpg) | EQ Band 3 frequency | `EQ Band 3 Freq` | Hover-only verification; screenshot retained because the accessibility tooltip may lag. EQ value left unchanged. |
| [t4-058-vst-audio-channel-settings-eq-band4-gain-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-058-vst-audio-channel-settings-eq-band4-gain-0.jpg) | EQ Band 4 gain | `EQ Band 4 Gain` | Hover only; four-band EQ remains flat and unchanged. |
| [t4-059-vst-audio-channel-settings-eq-band4-freq-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-059-vst-audio-channel-settings-eq-band4-freq-0.jpg) | EQ Band 4 frequency | `EQ Band 4 Freq` | Hover only; displayed value 12000 Hz remains unchanged. |
| [t4-060-vst-audio-channel-settings-eq-band4-q-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-060-vst-audio-channel-settings-eq-band4-q-0.jpg) | EQ Band 4 Q control | `EQ Band 4 Q` | Visible screenshot confirms Band 4 Q; earlier accessibility tree label lagged behind the pointer. Value unchanged. |
| [t4-061-vst-audio-channel-settings-sends-header-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-061-vst-audio-channel-settings-sends-header-0.jpg) | Sends section header | No tooltip | Hover only; Sends section is expanded and its eight slots are empty. |
| [t4-062-vst-audio-channel-settings-send1-bypass-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-062-vst-audio-channel-settings-send1-bypass-0.jpg) | Send 1 bypass/enable control | `Activate Send 1` | Hover only; Send 1 is empty and not activated. |
| [t4-063-vst-audio-channel-settings-send8-control-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-063-vst-audio-channel-settings-send8-control-0.jpg) | Send 8 activation control | `Activate Send 8` | Hover only; Send 8 is empty and not activated. |
| [t4-064-vst-audio-channel-settings-send8-slot-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-064-vst-audio-channel-settings-send8-slot-0.jpg) | Send 8 empty slot control | No tooltip | No tooltip visible at the cursor. Accessibility tree repeated the previous Activate Send 8 tooltip; the Send 8 slot remains empty and unchanged. |
| [t4-065-vst-audio-channel-settings-input-routing-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-065-vst-audio-channel-settings-input-routing-0.jpg) | Channel input routing control | `Input Routing (all selected with Shift + ALT + click)` | Hover only. Audio 02 input remains Left - Stereo In. |
| [t4-066-vst-audio-channel-settings-output-routing-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-066-vst-audio-channel-settings-output-routing-0.jpg) | Channel output routing control | `Output Routing (all selected with Shift + ALT + click)` | Visible screenshot confirms Output Routing (all selected with Shift + ALT + click); tree repeated the previous Input Routing label. Output remains Stereo Out. |
| [t4-067-vst-audio-channel-settings-input-phase-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-067-vst-audio-channel-settings-input-phase-0.jpg) | Input Phase control | `Input Phase` | Visible screenshot confirms Input Phase. Hover only; input phase unchanged. |
| [t4-068-vst-audio-channel-settings-input-gain-value-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-068-vst-audio-channel-settings-input-gain-value-0.jpg) | Input Gain value control | `Input Gain` | Visible screenshot confirms Input Gain; this coordinate is the input gain value, not pan. Value unchanged. |
| [t4-069-vst-audio-channel-settings-input-gain-knob-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-069-vst-audio-channel-settings-input-gain-knob-0.jpg) | Input Gain knob | `Input Gain (use shift)` | Visible screenshot confirms Input Gain (use shift); this coordinate is the input gain knob, not pan. Hover only. |
| [t4-070-vst-audio-channel-settings-pan-strip-no-tooltip-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-070-vst-audio-channel-settings-pan-strip-no-tooltip-0.jpg) | Channel strip horizontal pan-area probe | No tooltip | No tooltip visible at the cursor; the accessibility tree repeated Input Gain (use shift) from the prior probe. Pan display remains centered. |
| [t4-071-vst-audio-channel-settings-mute-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-071-vst-audio-channel-settings-mute-0.jpg) | Channel Mute button | `Mute` | Visible screenshot confirms Mute; the accessibility tree returned no tooltip text. Hover only; state unchanged. |
| [t4-072-vst-audio-channel-settings-solo-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-072-vst-audio-channel-settings-solo-0.jpg) | Channel Solo button | `Solo State (Solo Defeat with ALT click)` | Visible screenshot confirms Solo State (Solo Defeat with ALT click); the accessibility tree still showed Mute. Hover only; state unchanged. |
| [t4-073-vst-audio-channel-settings-listen-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-073-vst-audio-channel-settings-listen-0.jpg) | Channel Listen button | `Listen` | Visible screenshot confirms Listen; the accessibility tree still showed the previous Solo tooltip. Hover only; state unchanged. |
| [t4-074-vst-audio-channel-settings-automation-read-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-074-vst-audio-channel-settings-automation-read-0.jpg) | Automation Read button | `Read Automation` | Visible screenshot confirms Read Automation; tree retained the previous Listen tooltip. Hover only; state unchanged. |
| [t4-075-vst-audio-channel-settings-automation-write-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-075-vst-audio-channel-settings-automation-write-0.jpg) | Automation Write button | `Write Automation` | Visible screenshot confirms Write Automation; tree retained the previous Read Automation tooltip. Hover only; state unchanged. |
| [t4-076-vst-audio-channel-settings-record-enable-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-076-vst-audio-channel-settings-record-enable-0.jpg) | Channel Record Enable button | `Record Enable` | Hover only. Audio 02 Record Enable was already on and remains on. |
| [t4-077-vst-audio-channel-settings-channel-fader-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-077-vst-audio-channel-settings-channel-fader-0.jpg) | Channel volume fader | No tooltip | No tooltip visible at the fader handle; accessibility tree retained Record Enable from the prior probe. Volume remains 0.00. |
| [t4-078-vst-audio-channel-settings-fader-value-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-078-vst-audio-channel-settings-fader-value-0.jpg) | Channel fader value display | `Channel Level` | Hover only; volume value remains 0.00. |
| [t4-079-vst-audio-channel-settings-eq-band2-active-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-079-vst-audio-channel-settings-eq-band2-active-0.jpg) | EQ Band 2 active control | `EQ Band 2 Active` | Visible screenshot confirms Band 2 Active; accessibility tree retained Channel Level from the prior probe. Hover only; EQ unchanged. |
| [t4-080-vst-audio-channel-settings-eq-band3-active-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-080-vst-audio-channel-settings-eq-band3-active-0.jpg) | EQ Band 3 active control | `EQ Band 3 Active` | Hover only; EQ remains flat and unchanged. |
| [t4-081-vst-audio-channel-settings-eq-band4-active-0.jpg](../../../_analysis-tools/reports/workflow-L2/tooltips/t4-081-vst-audio-channel-settings-eq-band4-active-0.jpg) | EQ Band 4 active control | `EQ Band 4 Active` | Hover only; EQ remains flat and unchanged. |
