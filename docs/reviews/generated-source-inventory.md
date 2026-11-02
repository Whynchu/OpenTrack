# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0x5d8c2e9d0729652e`; keymap CSV `0xa9f1eb00c81948c7`
- Scope: the project-open main menu and the empty-project shell. Context menus are
enumerated in the curated `docs/reviews/audit-menus.md` register; this generated file
does not claim to cover them.
- `available` means the row is built with literal `true`; it is still subject to the
menu being open. `state-dependent` means the availability is project/selection state.
`unavailable` means literal `false` or `disabled_menu_commands`. `informational` rows
are captions, not commands.
- Generated families are expanded from the simple `for` arrays in the builder. A
`<name>` path is a runtime-generated label (workspace/template/device family) and is
counted once, not per runtime instance.

## Project-open main menu

268 leaves: 37 available, 131 state-dependent, 95 unavailable, 5 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 13975 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 13986 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 13924 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 13935 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 13948 |
| Audio > Plug-ins > Distortion |  | unavailable | `false` | 13903 |
| Audio > Plug-ins > Dynamics |  | unavailable | `false` | 13903 |
| Audio > Plug-ins > EQ |  | unavailable | `false` | 13903 |
| Audio > Plug-ins > Filter |  | unavailable | `false` | 13903 |
| Audio > Plug-ins > Modulation |  | unavailable | `false` | 13903 |
| Audio > Plug-ins > Other |  | unavailable | `false` | 13903 |
| Audio > Plug-ins > Reverb |  | unavailable | `false` | 13903 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 13897 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 13846 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 13850 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 13858 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 13878 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 13874 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 13866 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 13889 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 13862 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 13854 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 13870 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 13893 |
| Audio > Realtime Processing > Add Plug-in |  | unavailable | `false` | 13963 |
| Audio > Realtime Processing > Direct Offline Processing |  | unavailable | `false` | 13963 |
| Audio > Realtime Processing > Remove Plug-in |  | unavailable | `false` | 13963 |
| Audio > Spectrum Analyzer |  | unavailable | `false` | 13916 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 13917 |
| Devices > Control Room Mixer |  | unavailable | `false` | 14585 |
| Devices > Control Room Overview |  | unavailable | `false` | 14585 |
| Devices > Device Setup... |  | available | `true` | 14556 |
| Devices > MIDI Device Manager |  | unavailable | `false` | 14585 |
| Devices > MMC Master |  | unavailable | `false` | 14585 |
| Devices > Mixer | F3 | available | `true` | 14594 |
| Devices > Plug-in Information |  | available | `true` | 14565 |
| Devices > Realtime Performance... | F12 | available | `true` | 14570 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 14560 |
| Devices > VST Instruments | F11 | available | `true` | 14574 |
| Devices > Video Window | F8 | unavailable | `false` | 14651 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 13553 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 13589 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 13411 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 13542 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 13284 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 13319 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 13273 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 13434 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 13438 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 13492 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 13577 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 13500 |
| Edit > Glue |  | state-dependent | `can_glue` | 13419 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 13526 |
| Edit > History... |  | unavailable | `false` | 13264 |
| Edit > Key Commands... |  | available | `true` | 13641 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 13534 |
| Edit > Macros |  | unavailable | `false` | 13639 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 13415 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13353 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 13505 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 13509 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 13336 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 13514 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 13522 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && self.audio_event_clipboard.is_some()` | 13288 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 13613 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 13608 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13378 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 13463 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 13467 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 13450 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 13455 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 13459 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 13258 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 13496 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 13473 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 13485 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 13481 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 13477 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 13398 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13340 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 13312 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13366 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 13252 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 13530 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 13538 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 13518 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 13633 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 13633 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 13633 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 13633 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 13633 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 13633 |
| File > Back up Project... |  | unavailable | `false` | 13144 |
| File > Cleanup... |  | unavailable | `false` | 13231 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 13120 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 13179 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13203 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 13209 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 13190 |
| File > Export > Tempo Track... |  | unavailable | `false` | 13202 |
| File > Import > Audio CD... |  | unavailable | `false` | 13157 |
| File > Import > Audio... |  | available | `true` | 13153 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13171 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 13167 |
| File > Import > OMF... |  | unavailable | `false` | 13157 |
| File > Import > Tempo Track... |  | unavailable | `false` | 13157 |
| File > Import > Track Archive... |  | unavailable | `false` | 13157 |
| File > Import > Video File... |  | unavailable | `false` | 13157 |
| File > Key Commands... |  | available | `true` | 13237 |
| File > New Library... |  | unavailable | `false` | 13114 |
| File > New Project... | Ctrl+N | available | `true` | 13110 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 13124 |
| File > Open Library... |  | unavailable | `false` | 13119 |
| File > Open... | Ctrl+O | available | `true` | 13115 |
| File > Page Setup... |  | unavailable | `false` | 13149 |
| File > Preferences... |  | available | `true` | 13233 |
| File > Print... |  | unavailable | `false` | 13150 |
| File > Quit | Ctrl+Q | available | `true` | 13245 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 13242 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 13230 |
| File > Revert |  | unavailable | `false` | 13147 |
| File > Save | Ctrl+S | available | `true` | 13136 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 13140 |
| File > Save Library... |  | unavailable | `false` | 13146 |
| File > Save as Template... |  | unavailable | `false` | 13145 |
| Help > About OpenTrack... |  | available | `true` | 14762 |
| Help > Credits and Copyrights |  | available | `true` | 14758 |
| Help > OpenTrack on the Web |  | unavailable | `false` | 14756 |
| Help > Registration |  | unavailable | `false` | 14757 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 14148 |
| MIDI > CC Automation Setup... |  | unavailable | `false` | 14072 |
| MIDI > Drum Map Setup... |  | unavailable | `false` | 14072 |
| MIDI > Input device |  | informational | `` | 14083 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14038 |
| MIDI > Logical Presets |  | unavailable | `false` | 14072 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 14141 |
| MIDI > No MIDI input devices found |  | informational | `` | 14122 |
| MIDI > Open Drum Editor |  | unavailable | `false` | 14022 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 13997 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 13993 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14010 |
| MIDI > Open Score Editor |  | unavailable | `false` | 14022 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 14030 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 14127 |
| MIDI > Reset |  | unavailable | `false` | 14072 |
| MIDI > VST Expression |  | unavailable | `false` | 14072 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 14344 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 14355 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 14377 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 14310 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 14295 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 14403 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 14321 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 14403 |
| Media > Import Audio CD... |  | unavailable | `false` | 14403 |
| Media > Import Medium... |  | available | `true` | 14286 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 14291 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 14418 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 14418 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 14418 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 14392 |
| Media > Open Loop Browser | F6 | available | `true` | 14275 |
| Media > Open MediaBay | F5 | available | `true` | 14270 |
| Media > Open Pool Window | Ctrl+P | unavailable | `false` | 14269 |
| Media > Open Sound Browser | F7 | available | `true` | 14280 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 14381 |
| Media > Reconstruct |  | unavailable | `false` | 14403 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 14440 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 14332 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 14299 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 14431 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 14435 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 14366 |
| Project > Add Track > Arranger |  | unavailable | `false` | 13667 |
| Project > Add Track > Audio |  | available | `true` | 13654 |
| Project > Add Track > Browse Sounds... |  | unavailable | `false` | 13667 |
| Project > Add Track > FX Channel |  | unavailable | `false` | 13667 |
| Project > Add Track > Folder |  | unavailable | `false` | 13667 |
| Project > Add Track > Group Channel |  | available | `true` | 13654 |
| Project > Add Track > Instrument |  | available | `true` | 13654 |
| Project > Add Track > MIDI |  | available | `true` | 13654 |
| Project > Add Track > Marker |  | unavailable | `false` | 13667 |
| Project > Add Track > Ruler |  | unavailable | `false` | 13667 |
| Project > Add Track > Signature |  | unavailable | `false` | 13667 |
| Project > Add Track > Tempo |  | unavailable | `false` | 13667 |
| Project > Add Track > Transpose |  | unavailable | `false` | 13667 |
| Project > Add Track > Video |  | unavailable | `false` | 13667 |
| Project > Auto Fades Settings... |  | available | `true` | 13795 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 13803 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 13784 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 13779 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 13748 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 13775 |
| Project > New from Template > <label> |  | available | `true` | 13742 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 13738 |
| Project > Notepad |  | state-dependent | `self.project_open` | 13791 |
| Project > Pool | Ctrl+P | available | `true` | 13713 |
| Project > Project Setup... | Shift+S | available | `true` | 13771 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 13760 |
| Project > Remove Empty Tracks |  | unavailable | `false` | 13703 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 13721 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 13717 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 13799 |
| Project > Tempo Track | Ctrl+T | unavailable | `false` | 13807 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 13706 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 13706 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 14257 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 14257 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 14257 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 14257 |
| Scores > Align Elements > Bottom |  | unavailable | `false` | 14197 |
| Scores > Align Elements > Distribute Horizontally |  | unavailable | `false` | 14197 |
| Scores > Align Elements > Distribute Vertically |  | unavailable | `false` | 14197 |
| Scores > Align Elements > Left |  | unavailable | `false` | 14197 |
| Scores > Align Elements > Right |  | unavailable | `false` | 14197 |
| Scores > Align Elements > Top |  | unavailable | `false` | 14197 |
| Scores > Auto Layout... |  | unavailable | `false` | 14255 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14171 |
| Scores > Convert to Grace Note |  | unavailable | `false` | 14183 |
| Scores > Flip |  | unavailable | `false` | 14183 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14240 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14227 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 14253 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14214 |
| Scores > Group/Ungroup Notes |  | unavailable | `false` | 14183 |
| Scores > Hide/Show |  | unavailable | `false` | 14183 |
| Scores > Insert Slur |  | unavailable | `false` | 14183 |
| Scores > Make Chord Symbols |  | unavailable | `false` | 14209 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 14209 |
| Scores > Open Layout... |  | unavailable | `false` | 14183 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14158 |
| Scores > Page Mode |  | unavailable | `false` | 14183 |
| Scores > Reset Layout... |  | unavailable | `false` | 14255 |
| Scores > Settings... |  | unavailable | `false` | 14183 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 14471 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 14471 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 14471 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 14471 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 14471 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 14471 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 14471 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 14471 |
| Transport > Metronome On | C | available | `true` | 14520 |
| Transport > Metronome Setup... |  | available | `true` | 14516 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 14471 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 14482 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 14476 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 14498 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 14493 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 14487 |
| Transport > Post-roll from Selection End |  | unavailable | `false` | 14503 |
| Transport > Post-roll from Selection Start |  | unavailable | `false` | 14503 |
| Transport > Pre-roll to Selection End |  | unavailable | `false` | 14503 |
| Transport > Pre-roll to Selection Start |  | unavailable | `false` | 14503 |
| Transport > Precount On |  | available | `true` | 14526 |
| Transport > Project Synchronization Setup... |  | unavailable | `false` | 14546 |
| Transport > Retrospective Record | Shift+Num * | unavailable | `false` | 14546 |
| Transport > Start Record at Left Locator |  | unavailable | `false` | 14514 |
| Transport > Transport Panel | F2 | available | `true` | 14453 |
| Transport > Use External Sync | T | unavailable | `false` | 14546 |
| Transport > Use Post-roll |  | unavailable | `false` | 14513 |
| Transport > Use Pre-roll |  | unavailable | `false` | 14512 |
| Window > Windows... |  | unavailable | `false` | 14683 |
| Window > Workspaces > <lock_label> |  | available | `true` | 14678 |
| Window > Workspaces > <name> |  | available | `*active` | 14656 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 14662 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 14809 |
| Devices > Device Setup... |  | available | `true` | 14812 |
| Devices > VST Connections... | F4 | unavailable | `false` | 14816 |
| Edit > Key Commands... |  | available | `true` | 14800 |
| Edit > Redo |  | unavailable | `false` | 14798 |
| Edit > Undo |  | unavailable | `false` | 14797 |
| File > New Project... | Ctrl+N | available | `true` | 14779 |
| File > Open... | Ctrl+O | available | `true` | 14783 |
| File > Quit | Alt+F4 | available | `true` | 14791 |
| File > Save | Ctrl+S | unavailable | `false` | 14788 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 14789 |
| Help > About OpenTrack... |  | available | `true` | 14824 |
| MIDI > No project is open |  | unavailable | `false` | 14820 |
| Media > No project is open |  | unavailable | `false` | 14820 |
| Project > No project is open |  | unavailable | `false` | 14806 |
| Scores > No project is open |  | unavailable | `false` | 14820 |
| Transport > No project is open |  | unavailable | `false` | 14820 |
| Window > No project is open |  | unavailable | `false` | 14820 |

No duplicate full paths.

## Bundled Windows reference keymap

152 distinct command IDs; 147 have a `dispatch_reference_command` arm; 5 have none.

A dispatch arm only proves the command reaches code in that function. It does not prove the
resulting action is complete, and it does not cover commands intercepted before dispatch.

| Command ID | Category | Label | Chords | Dispatch arm |
|---|---|---|---|---|
| `audio.adjust_fades_to_range` | Audio | Adjust Fades to Range | A | yes |
| `audio.auto_grid` | Audio | Auto Grid | Shift+Q | yes |
| `audio.crossfade` | Audio | Crossfade | X | yes |
| `audio.find_selected_in_pool` | Audio | Find Selected in Pool | Ctrl+F | yes |
| `automation.toggle_read_enable_all_tracks` | Automation | Toggle Read Enable All Tracks | Alt+R | yes |
| `automation.toggle_write_enable_all_tracks` | Automation | Toggle Write Enable All Tracks | Alt+W | yes |
| `devices.mixer` | Devices | Mixer | F3 | yes |
| `devices.video` | Devices | Video | F8 | no |
| `devices.virtual_keyboard` | Devices | Virtual Keyboard | Alt+K | yes |
| `devices.vst_connections` | Devices | VST Connections | F4 | yes |
| `devices.vst_instruments` | Devices | VST Instruments | F11 | yes |
| `devices.vst_performance` | Devices | VST Performance | F12 | yes |
| `edit.autoscroll` | Edit | Autoscroll | F | yes |
| `edit.copy` | Edit | Copy | Ctrl+C | yes |
| `edit.cut` | Edit | Cut | Ctrl+X | yes |
| `edit.cut_time` | Edit | Cut Time | Ctrl+Shift+X | yes |
| `edit.delete` | Edit | Delete | Backspace, Delete | yes |
| `edit.delete_time` | Edit | Delete Time | Shift+Backspace | yes |
| `edit.duplicate` | Edit | Duplicate | Ctrl+D | yes |
| `edit.edit_in_place` | Edit | Edit In-place | Ctrl+Shift+I | yes |
| `edit.group` | Edit | Group | Ctrl+G | yes |
| `edit.insert_silence` | Edit | Insert Silence | Ctrl+Shift+E | yes |
| `edit.left_selection_side_to_cursor` | Edit | Left Selection Side to Cursor | E | yes |
| `edit.lock` | Edit | Lock | Ctrl+Shift+L | yes |
| `edit.move_to_cursor` | Edit | Move to Cursor | Ctrl+L | yes |
| `edit.mute` | Edit | Mute | M | yes |
| `edit.mute_events` | Edit | Mute Events | Shift+M | yes |
| `edit.mute_unmute_objects` | Edit | Mute/Unmute Objects | Alt+M | yes |
| `edit.open_close_editor` | Edit | Open/Close Editor | Enter | yes |
| `edit.open_default_editor` | Edit | Open Default Editor | Ctrl+E | yes |
| `edit.open_score_editor` | Edit | Open Score Editor | Ctrl+R | no |
| `edit.paste` | Edit | Paste | Ctrl+V | yes |
| `edit.paste_at_origin` | Edit | Paste at Origin | Alt+V | yes |
| `edit.paste_time` | Edit | Paste Time | Ctrl+Shift+V | yes |
| `edit.record_enable` | Edit | Record Enable | R | yes |
| `edit.redo` | Edit | Redo | Ctrl+Shift+Z | yes |
| `edit.repeat` | Edit | Repeat | Ctrl+K | yes |
| `edit.right_selection_side_to_cursor` | Edit | Right Selection Side to Cursor | D | yes |
| `edit.select_all` | Edit | Select All | Ctrl+A | yes |
| `edit.select_none` | Edit | Select None | Ctrl+Shift+A | yes |
| `edit.snap_on_off` | Edit | Snap On/Off | J | yes |
| `edit.solo` | Edit | Solo | S | yes |
| `edit.split_at_cursor` | Edit | Split At Cursor | Alt+X | yes |
| `edit.split_range` | Edit | Split Range | Shift+X | yes |
| `edit.undo` | Edit | Undo | Ctrl+Z | yes |
| `edit.ungroup` | Edit | Ungroup | Ctrl+U | yes |
| `edit.unlock` | Edit | Unlock | Ctrl+Shift+U | yes |
| `edit.unmute_events` | Edit | Unmute Events | Shift+U | yes |
| `editors.show_hide_info_view` | Editors | Show/Hide Info view | Ctrl+I | yes |
| `editors.show_hide_inspector` | Editors | Show/Hide Inspector | Alt+I | yes |
| `editors.show_hide_overview` | Editors | Show/Hide Overview | Alt+O | yes |
| `file.close` | File | Close | Ctrl+W | yes |
| `file.new` | File | New | Ctrl+N | yes |
| `file.open` | File | Open | Ctrl+O | yes |
| `file.quit` | File | Quit | Ctrl+Q | yes |
| `file.save` | File | Save | Ctrl+S | yes |
| `file.save_as` | File | Save As | Ctrl+Shift+S | yes |
| `file.save_new_version` | File | Save New Version | Ctrl+Alt+S | yes |
| `media.open_loop_browser` | Media | Open Loop Browser | F6 | yes |
| `media.open_mediabay` | Media | Open MediaBay | F5 | yes |
| `media.open_sound_browser` | Media | Open Sound Browser | F7 | yes |
| `midi.quantize` | MIDI | Quantize | Q | yes |
| `navigate.add_down` | Navigate | Add Down | Shift+Down | yes |
| `navigate.add_left` | Navigate | Add Left | Shift+Left | yes |
| `navigate.add_right` | Navigate | Add Right | Shift+Right | yes |
| `navigate.add_up` | Navigate | Add Up | Shift+Up | yes |
| `navigate.bottom` | Navigate | Bottom | End | yes |
| `navigate.down` | Navigate | Down | Down | yes |
| `navigate.left` | Navigate | Left | Left | yes |
| `navigate.right` | Navigate | Right | Right | yes |
| `navigate.toggle_selection` | Navigate | Toggle Selection | Ctrl+Space | yes |
| `navigate.top` | Navigate | Top | Home | yes |
| `navigate.up` | Navigate | Up | Up | yes |
| `nudge.end_left` | Nudge | End Left | Alt+Shift+Left | yes |
| `nudge.end_right` | Nudge | End Right | Alt+Shift+Right | yes |
| `nudge.left` | Nudge | Left | Ctrl+Left | yes |
| `nudge.right` | Nudge | Right | Ctrl+Right | yes |
| `nudge.start_left` | Nudge | Start Left | Alt+Left | yes |
| `nudge.start_right` | Nudge | Start Right | Alt+Right | yes |
| `project.open_browser` | Project | Open Browser | Ctrl+B | yes |
| `project.open_close_pool` | Project | Open/Close Pool | Ctrl+P | yes |
| `project.open_markers` | Project | Open Markers | Ctrl+M | yes |
| `project.open_tempo_track_editor` | Project | Open Tempo Track Editor | Ctrl+T | no |
| `project.setup` | Project | Setup | Shift+S | yes |
| `project.show_hide_track_colors` | Project | Show/Hide Track Colors | Shift+C | yes |
| `tool.delete_tool` | Tool | Delete tool | 5 | yes |
| `tool.draw_tool` | Tool | Draw tool | 8 | yes |
| `tool.drumstick_tool` | Tool | Drumstick tool | 0 | yes |
| `tool.glue_tool` | Tool | Glue tool | 4 | yes |
| `tool.mute_tool` | Tool | Mute tool | 7 | yes |
| `tool.next_tool` | Tool | Next Tool | F10 | yes |
| `tool.play_tool` | Tool | Play tool | 9 | yes |
| `tool.previous_tool` | Tool | Previous Tool | F9 | yes |
| `tool.range_tool` | Tool | Range tool | 2 | yes |
| `tool.select_tool` | Tool | Select tool | 1 | yes |
| `tool.split_tool` | Tool | Split tool | 3 | yes |
| `tool.zoom_tool` | Tool | Zoom tool | 6 | yes |
| `transport.auto_punch_in` | Transport | Auto Punch In | I | yes |
| `transport.auto_punch_out` | Transport | Auto Punch Out | O | yes |
| `transport.cycle` | Transport | Cycle | Num / | yes |
| `transport.exchange_time_formats` | Transport | Exchange time formats | . | yes |
| `transport.fast_forward` | Transport | Fast Forward | Shift+Num + | yes |
| `transport.fast_rewind` | Transport | Fast Rewind | Shift+Num - | yes |
| `transport.forward` | Transport | Forward | Num - | yes |
| `transport.input_left_locator` | Transport | Input Left Locator | Shift+L | yes |
| `transport.input_position` | Transport | Input Position | Shift+P | yes |
| `transport.input_right_locator` | Transport | Input Right Locator | Shift+R | yes |
| `transport.input_tempo` | Transport | Input Tempo | Shift+T | yes |
| `transport.insert_marker` | Transport | Insert Marker | Insert | yes |
| `transport.locate_next_event` | Transport | Locate Next Event | N | yes |
| `transport.locate_next_marker` | Transport | Locate Next Marker | Shift+N | yes |
| `transport.locate_previous_event` | Transport | Locate Previous Event | B | yes |
| `transport.locate_previous_marker` | Transport | Locate Previous Marker | Shift+B | yes |
| `transport.locate_selection` | Transport | Locate Selection | L | yes |
| `transport.locators_to_selection` | Transport | Locators to Selection | P | yes |
| `transport.loop_selection` | Transport | Loop Selection | Shift+G | yes |
| `transport.metronome_on` | Transport | Metronome On | C | yes |
| `transport.nudge_cursor_left` | Transport | Nudge Cursor left | Ctrl+Num + | yes |
| `transport.nudge_cursor_right` | Transport | Nudge Cursor right | Ctrl+Num - | yes |
| `transport.panel_transport_panel` | Transport | Panel (Transport panel) | F2 | yes |
| `transport.play` | Transport | Play | Enter | yes |
| `transport.play_selection_range` | Transport | Play Selection Range | Alt+Space | yes |
| `transport.recall_cycle_marker_1_to_9` | Transport | Recall Cycle Marker 1 to 9 | Shift+Num 1, Shift+Num 2, Shift+Num 3, Shift+Num 4, Shift+Num 5, Shift+Num 6, Shift+Num 7, Shift+Num 8, Shift+Num 9 | yes |
| `transport.record` | Transport | Record | Num * | yes |
| `transport.retrospective_record` | Transport | Retrospective Record | Shift+Num * | no |
| `transport.return_to_zero` | Transport | Return to Zero | Num ,, Num . | yes |
| `transport.rewind` | Transport | Rewind | Num - | yes |
| `transport.set_left_locator` | Transport | Set Left Locator | Ctrl+Num 1 | yes |
| `transport.set_marker_1` | Transport | Set Marker 1 | Ctrl+1 | yes |
| `transport.set_marker_2` | Transport | Set Marker 2 | Ctrl+2 | yes |
| `transport.set_marker_3_to_9` | Transport | Set Marker 3 to 9 | Ctrl+3, Ctrl+4, Ctrl+5, Ctrl+6, Ctrl+7, Ctrl+8, Ctrl+9, Ctrl+Num 3, Ctrl+Num 4, Ctrl+Num 5, Ctrl+Num 6, Ctrl+Num 7, Ctrl+Num 8, Ctrl+Num 9 | yes |
| `transport.set_right_locator` | Transport | Set Right Locator | Ctrl+Num 2 | yes |
| `transport.start_stop` | Transport | Start/Stop | Space | yes |
| `transport.stop` | Transport | Stop | Num 0 | yes |
| `transport.to_left_locator` | Transport | To Left Locator | Num 1 | yes |
| `transport.to_marker_1` | Transport | To Marker 1 | Shift+1 | yes |
| `transport.to_marker_2` | Transport | To Marker 2 | Shift+2 | yes |
| `transport.to_marker_3_to_9` | Transport | To Marker 3 to 9 | Num 3, Num 4, Num 5, Num 6, Num 7, Num 8, Num 9, Shift+3, Shift+4, Shift+5, Shift+6, Shift+7, Shift+8, Shift+9 | yes |
| `transport.to_right_locator` | Transport | To Right Locator | Num 2 | yes |
| `transport.use_external_sync` | Transport | Use External Sync | T | no |
| `workspace.lock_unlock_active_workspace` | Workspace | Lock/Unlock Active Workspace | Alt+Num 0 | yes |
| `workspace.new` | Workspace | New | Ctrl+Num 0 | yes |
| `workspace.organize` | Workspace | Organize | W | yes |
| `workspace.workspace_1_to_9` | Workspace | Workspace 1 to 9 | Alt+Num 1, Alt+Num 2, Alt+Num 3, Alt+Num 4, Alt+Num 5, Alt+Num 6, Alt+Num 7, Alt+Num 8, Alt+Num 9 | yes |
| `zoom.zoom_full` | Zoom | Zoom Full | Shift+F | yes |
| `zoom.zoom_in` | Zoom | Zoom In | H | yes |
| `zoom.zoom_in_tracks` | Zoom | Zoom In Tracks | Alt+Down | yes |
| `zoom.zoom_out` | Zoom | Zoom Out | G | yes |
| `zoom.zoom_out_tracks` | Zoom | Zoom Out Tracks | Alt+Up, Ctrl+Up | yes |
| `zoom.zoom_to_event` | Zoom | Zoom to Event | Shift+E | yes |
| `zoom.zoom_to_selection` | Zoom | Zoom to Selection | Alt+S | yes |
| `zoom.zoom_tracks_exclusive` | Zoom | Zoom Tracks Exclusive | Ctrl+Down, Z | yes |

### Bound commands without a dispatch arm

- Video (`F8`, Devices) — `devices.video`
- Open Score Editor (`Ctrl+R`, Edit) — `edit.open_score_editor`
- Open Tempo Track Editor (`Ctrl+T`, Project) — `project.open_tempo_track_editor`
- Retrospective Record (`Shift+Num *`, Transport) — `transport.retrospective_record`
- Use External Sync (`T`, Transport) — `transport.use_external_sync`

