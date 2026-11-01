# OpenTrack UI implementation inventory

Status: source inventory v1, 2026-11-01. Read-only audit; existing application changes were preserved. Runtime behavior has not been inspected in this pass. “Disabled” is source-confirmed; “planned” is confirmed only as a reference page or requirement and may overlap existing behavior. Re-audit before assigning implementation.

## Summary

There is no single exhaustive list generated from the running application. This inventory consolidates the explicit source and documentation evidence found so far. It is a starting backlog rather than proof that every visible control works or fails. A control can be partly implemented even if a broader menu item is disabled.

### Source-confirmed unavailable controls and sections

| Location | Disabled item(s) / gap | Evidence |
| --- | --- | --- |
| File > Import | Audio CD, Video File, MIDI File, Track Archive, Tempo Track, MusicXML, OMF and other entries in the same disabled block | `crates/opentrack-app/src/app.rs`, `disabled_menu_commands` near 7714 |
| File > Export | Audio Mixdown, Selected Tracks, Tempo Track | `app.rs`, near 7728 |
| File > Edit | Paste at Origin, Split at Cursor/Loop, selection/range actions, duplicate/repeat/fill-loop, move-to locators, lock/mute/group and other entries in disabled blocks | `app.rs`, near 7807–7892; some adjacent operations are live |
| Project menu/context menu | Arranger, FX Channel, Folder, Marker, Ruler, Signature, Tempo, Transpose and Video tracks; Remove Empty Tracks; several window/panel actions; automation bulk actions; track preset/settings actions | `app.rs` near 7930–7979; `project_controls.rs` lines 150–265 |
| Audio menu | Process actions (fade, envelope, gain, merge, normalize, DC offset, etc.), audio plug-in/effect categories, hitpoint actions, realtime/offline plug-ins, and advanced analysis entries | `app.rs` near 8027–8083; categories without a registered effect show “No implemented effects” near 17574 |
| MIDI menu/editor | Score/Drum/List/In-Place editor actions, quantize setup, logical presets, drum map, note utilities and advanced layout | `app.rs` near 8098–8241; MIDI editor includes “Split at Selection” disabled near 15231 |
| Media menu | Pool window/import/export/reconstruct/convert functions, media search, and track creation from media | `app.rs` near 8274–8306; some file relinking and current MediaBay functions exist |
| Transport menu | Locator/event/marker navigation variants, pre/post-roll, sync and retrospective record | `app.rs` near 8331–8368; transport panel has active controls elsewhere |
| Devices menu | Control Room, MIDI device manager and MMC master | `app.rs` near 8397 |
| Window menu | Additional Mixer windows, VST instruments/performance, video window and other legacy windows | `app.rs` near 8411; some intentionally out of current product scope |
| Preferences | All reference pages except VST Metering show “Planned”; preset Store/Rename/Delete disabled; hardware Direct Monitoring mapping disabled | `crates/opentrack-app/src/preferences.rs` lines 201–226 |
| Channel Settings layout | Insert Routing, Studio Sends, Studio Sends Routing are marked unimplemented | `app.rs` ChannelSettingsViewItem::implemented near 247; labels displayed in `app.rs`/`channel_settings.rs` |
| Project track context | Show Used Automation, Show All Used Automation, Track Controls Settings, Disable Track, Set Record Folder, Auto Fades Settings, track preset actions; various track types | `crates/opentrack-app/src/project_controls.rs` lines 150–265 |
| Audio configuration | All/expand/collapse and add/remove buttons in device-channel map are disabled | `crates/opentrack-app/src/audio_configuration.rs` near 703–713; determine whether capability is intentionally deferred |

## Preference page backlog

The [existing 32-page reference inventory](../reviews/preferences-reference-inventory.md) lists observed controls, dependency notes and exclusions. Source shows only VST Metering is functional as a preference page. Keep this table as a summary; the linked source contains the detailed control lists.

| Family | Pages or behavior recorded as planned | Source / reference |
| --- | --- | --- |
| Appearance | General palette, meters, work-area/editor grids | PREF-02–04 |
| Editing | Selection, overlaps, lock, zoom, drag delay, automation-follow policy | PREF-05 |
| Editing > Audio | Stretch, import/shared-clip policy, event gestures, hitpoints, zero-crossing | PREF-06 |
| Editing > Controls | Value box, knob and slider interaction modes | PREF-07 |
| Editing > MIDI | Controller selection, legato and split policies | PREF-08 |
| Editing > Project & Mixer | Selection synchronization, track color/scroll/folding/enlargement policies | PREF-09 |
| Editing > Tools | Tool modifiers, contextual toolbox and crosshair preferences | PREF-10–11 |
| Event Display | Event names, backgrounds, audio waveform/fade and MIDI display policies | PREF-13–16 |
| General | Auto-save/backups, tips, undo limit, startup policy | PREF-18 |
| MIDI | Thru/chase/filtering and MIDI file import/export | PREF-19–21 |
| MediaBay | Result limits, viewer editing, scan policy/extensions | PREF-22 |
| Record | Audio pre-roll/cycle recording and MIDI retrospective/replace policies | PREF-24–25 |
| Transport | Locate/zoom/stop/punch/wind and audible scrub policies | PREF-26–27 |
| VST | Send defaults, automation/mute/monitoring policies, plug-in behavior, Control Room | PREF-28–31 |
| VariAudio | Warnings associated with pitch editing | PREF-32 |
| Excluded by direction | Video and Scores preference pages | PREF-12, PREF-17; Scores excluded in task docs |

## Windows

Window inventory from source title construction is not yet matched against every menu command or lifecycle. Existing window implementations include Project/main shell, Mixer, Channel Settings, Transport, MediaBay, MIDI Editor, Preferences, Device Setup, Project Setup, Plug-in Information, Key Commands, Assign Shortcut, Workspace Organizer, metronome setup, tempo/time signature, audio file operation, project-rate confirmation, VST editors, Event Colors and track dialogs. The presence of a window does not certify every action inside it. Some appear conditionally or transiently.

Potential missing/disabled window destinations include VST Connections, Control Room views, MIDI Device Manager, VST instruments/performance, Video Window, additional Mixer instances, Browser/Pool variants and MediaBay subviews. Several are legacy reference commands; product decisions are needed before treating every one as a required feature.

## Shared implementation risks

- The keyboard command catalog registers many reference commands, and dispatch has a generic fallback. A registered command is not automatically a live menu action; compare each ID with handler behavior and menu enabled state.
- `app.rs` is roughly 19,761 lines and includes broad application wiring. Feature work that touches its dispatch/state areas requires a single integration owner.
- Sends have stored/UI state but send DSP is explicitly absent (`mixer.rs` around 1203); routing controls need an engine contract before implementation.
- Automation, cycle takes, MIDI file interchange, Control Room and CPR project interchange are broader workflow gaps documented elsewhere. They require task-specific requirements and evidence, not one-off control patches.
- Disabled items may be intentionally unavailable. Remove or label stale legacy items only after product-scope review.

## Next audit steps

1. Enumerate all built-in command IDs; map them to actual dispatch branches, menus and UI actions. Mark implemented, partial, unavailable or intentionally deferred.
2. Capture every menu/window path in the running app and check each disabled or suspicious control against the source. Keep runtime findings separate from source status.
3. Inspect active dropdowns for choices that do not change the expected project/device/DSP state. Source grep alone cannot establish runtime dropdown completeness.
4. For each confirmed gap, link the user outcome, owning state, engine consumer, persistence, undo/redo, failure modes, file ownership and verification into a task packet.
5. Prioritize small independent slices and share `app.rs`/audio graph ownership through one integrator.

## Detailed window and selection-control findings (Luna Low source passes)

The first inventory grouped some menus, but the audit now has these concrete window/control findings. These are source-confirmed; no desktop runtime pass was performed.

| Window/control | Source status | Finding |
| --- | --- | --- |
| Device Setup > VST Connections | Partial | Input/Output mapping views exist. Group/FX, External FX, External Instruments and Studio tabs report unavailable. All, Add Bus, and preset controls are disabled. `crates/opentrack-app/src/audio_configuration.rs:649,694-724`. |
| Add Track dialog | Partial | Mono/Stereo selection changes new audio/group track channel count. Speaker choice is disabled and fixed to M; instrument selector is disabled with “No VST Instrument”; Browse Presets has no connected behavior. `crates/opentrack-app/src/app.rs:10210-10406`. |
| Preferences | Partial | VST Metering is live. Other retained pages show Planned; Hardware Direct Monitoring mapping and preset Store/Rename/Delete are disabled. `crates/opentrack-app/src/preferences.rs:175-226`. |
| Channel Settings | Partial | Inserts/EQ/Fader and normal-track Sends UI exist. Insert Routing and Studio Sends variants are marked unimplemented. Send audio processing has no consumer. `app.rs:196-253`, `channel_settings.rs:2809-2819`, `mixer.rs:1203`. |
| MIDI Editor | Partial | Position grid, note length and controller lane selection are connected to editing state. Toolbar Snap checkbox is a local value with no state consumer; Split at Selection is disabled. `app.rs:15211-15231,15494-15504`. |
| Media/browser windows | Shared destination | MediaBay, Open Loop Browser and Open Sound Browser route to category indices but render the same Media Pool viewport; no separate browser windows were found. `app.rs:8253-8267,9704+`. |
| VST insert picker | Partial catalog | Several named Delay/Dynamics/EQ entries are disabled; empty groups say “No implemented effects.” Some built-ins and discovered third-party VST3 choices work. `app.rs:17484-17574`. |
| Disabled Devices/Window destinations | No destination in current menu | Control Room Mixer/Overview, MIDI Device Manager, MMC Master, Mixer 2/3, VST Instruments/Performance, Video Window, Virtual Keyboard, Show Panel, and Windows… are disabled. `app.rs:8397-8456`. Scope decisions are needed for legacy-specific panels. |
| Audio device channel map | Partial | All, expand/collapse and add/remove map buttons are disabled. `audio_configuration.rs:703-713`. |

### Dropdown/selection controls checked

Most current selectors inspected have direct consumers: WASAPI/ASIO mode, audio input/output device, driver rate/buffer, project recording rate/encoding, MIDI input, Analyzer comparison/FFT/tilt/range, Sample Editor timebase/grid/quantize, and MIDI editor grid/length/controller lane. These are source-wired and not classified as stubs based on code reading alone. Physical device behavior and actual runtime results remain unverified.

Exceptions found: Add Track speaker/instrument choices above; MIDI toolbar Snap checkbox above; unavailable VST Connections bus types; disabled selection presets in Preferences. Disabled items while a worker runs or before folders/devices are configured are state-gated and not counted as missing features.

## Workflow implementation and stale-document checks

| Workflow | Current finding | Evidence and documentation correction |
| --- | --- | --- |
| Automation | Track automation read/write flags and arrange reveal lane exist, but no persistent automation event/curve model or engine playback/write path was found. | `opentrack-core/src/project.rs` Track fields; `app.rs` automation lane; `docs/workflow/tasks/T020-automation-and-channel-gain.md:9,15`. Treat lane layout as implemented and automation behavior as absent. |
| Sends | Destination/rack UI and stored route metadata exist, but no audible send DSP and no complete persistent per-send enabled/level state. | `app.rs` Track send fields; `channel_settings.rs`; `mixer.rs:1203`; T015/T020. Older statements that the sends section itself is absent are stale for ordinary track editors. |
| MIDI arrangement | Moving/resizing MIDI parts appears implemented in current app source, despite older docs. | `app.rs:13978-14026,14175`; `docs/midi-editor.md:40` should be corrected. Physical MIDI output ports remain absent; current live input path/hardware has separate validation limits. |
| Audio recording | WAV recording, take paths, undo/redo/discard recovery and waveform preview exist. Automatic recovery after abnormal termination is absent; hardware timing/reliability remains unverified. | `opentrack-audio/src/recording.rs`, `opentrack-app/src/recording_undo.rs`, `docs/workflow/tasks/T016-recording-undo-and-waveforms.md:11`; older T011 capture description may be stale. |
| Audio export | No audio mixdown/export path surfaced in app menus/source; File > Export entries are disabled. | `app.rs:7727-7738`; requires a separate rendering/export acceptance packet. |
| MIDI files | MIDI notes/controller events persist and edit in OPR, but MIDI File import/export actions and file policy are absent. | Preferences inventory PREF-20; File/Media menu disabled ranges; `docs/midi-editor.md`. |
| CPR interoperability | `cpr-inspect` reads outer framing and selected ROOT labels; no track/media/mixer decoding, importer, exporter or round trip. | `crates/opentrack-core/src/cpr.rs`, `docs/compatibility-notes.md:63-64`; do not describe inspection as project interchange. |
| Native project/media | OPR save/open is native project support, not cross-DAW interchange. Media is referenced by paths; imports are copied and relinking exists; media is not embedded. | `docs/native-project-format.md:22`; moving projects can require relinking. |

These checks expose a structural command gap: `CommandCatalog` is built from keyboard reference CSV rows, while menus are created directly in `app.rs`. There is no stable menu-item-to-command ID registry, so command catalog completeness cannot prove menu completeness. A full command audit must extract menu items, shortcut IDs and dispatch arms as separate inventories, then map semantics explicitly.

## Documentation cleanup candidates

- Update `docs/midi-editor.md:40`: current app has MIDI part move/resize handling; preserve distinctions for physical MIDI output and hardware validation.
- Keep `docs/native-project-format.md:22` automation statement specific: automation persistence/playback/write remains unimplemented, though automation reveal/layout controls exist.
- Update older T011 recording claims against current recording path and hardware evidence; do not assume old task packet means current behavior.
- Distinguish current Sends UI/routing configuration from missing send DSP and full per-send state.
- Keep CPR inspection utility and CPR interchange claims separate.

## Remaining full-audit gates

1. Enumerate every generated/conditional menu item and each enabled handler, including empty-project state and context menus; map to actual action/state/engine consumer.
2. Inspect all other windows not covered by the main pass (transport, mixer, Channel Settings, MediaBay, Project Setup, Device Setup, Plug-in Information, Key Commands, Workspace Organizer and modal workflows) for controls whose appearance is real but behavior is partial.
3. Review all ComboBox/Selectable/Popup controls beyond the first pass, including project color/event palettes, insert choices, routing, plugin menus and contextual track/event controls.
4. Run the application and exercise suspicious items where runtime UI inspection is available; record visible, enabled, disabled, persisted and audible results separately.
5. Convert confirmed missing workflows to user-outcome packets with dependencies and file ownership before implementation.
