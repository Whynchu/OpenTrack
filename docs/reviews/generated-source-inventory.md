# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0x66a3c6725d4d17cb`; keymap CSV `0xa9f1eb00c81948c7`
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

271 leaves: 38 available, 138 state-dependent, 87 unavailable, 8 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14122 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 14133 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14071 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14082 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14095 |
| Audio > Plug-ins > Distortion |  | unavailable | `false` | 14050 |
| Audio > Plug-ins > Dynamics |  | unavailable | `false` | 14050 |
| Audio > Plug-ins > EQ |  | unavailable | `false` | 14050 |
| Audio > Plug-ins > Filter |  | unavailable | `false` | 14050 |
| Audio > Plug-ins > Modulation |  | unavailable | `false` | 14050 |
| Audio > Plug-ins > Other |  | unavailable | `false` | 14050 |
| Audio > Plug-ins > Reverb |  | unavailable | `false` | 14050 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 14044 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 13993 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 13997 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 14005 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 14025 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 14021 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 14013 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 14036 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 14009 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 14001 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 14017 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 14040 |
| Audio > Realtime Processing > Add Plug-in |  | unavailable | `false` | 14110 |
| Audio > Realtime Processing > Direct Offline Processing |  | unavailable | `false` | 14110 |
| Audio > Realtime Processing > Remove Plug-in |  | unavailable | `false` | 14110 |
| Audio > Spectrum Analyzer |  | unavailable | `false` | 14063 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 14064 |
| Devices > Control Room Mixer |  | unavailable | `false` | 14795 |
| Devices > Control Room Overview |  | unavailable | `false` | 14795 |
| Devices > Device Setup... |  | available | `true` | 14766 |
| Devices > MIDI Device Manager |  | unavailable | `false` | 14795 |
| Devices > MMC Master |  | unavailable | `false` | 14795 |
| Devices > Mixer | F3 | available | `true` | 14804 |
| Devices > Plug-in Information |  | available | `true` | 14775 |
| Devices > Realtime Performance... | F12 | available | `true` | 14780 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 14770 |
| Devices > VST Instruments | F11 | available | `true` | 14784 |
| Devices > Video Window | F8 | unavailable | `false` | 14861 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 13700 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 13736 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 13558 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 13689 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 13431 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 13466 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 13420 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 13581 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 13585 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 13639 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 13724 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 13647 |
| Edit > Glue |  | state-dependent | `can_glue` | 13566 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 13673 |
| Edit > History... |  | unavailable | `false` | 13411 |
| Edit > Key Commands... |  | available | `true` | 13788 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 13681 |
| Edit > Macros |  | unavailable | `false` | 13786 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 13562 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13500 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 13652 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 13656 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 13483 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 13661 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 13669 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && self.audio_event_clipboard.is_some()` | 13435 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 13760 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 13755 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13525 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 13610 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 13614 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 13597 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 13602 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 13606 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 13405 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 13643 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 13620 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 13632 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 13628 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 13624 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 13545 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13487 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 13459 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13513 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 13399 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 13677 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 13685 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 13665 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 13780 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 13780 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 13780 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 13780 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 13780 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 13780 |
| File > Back up Project... |  | unavailable | `false` | 13291 |
| File > Cleanup... |  | unavailable | `false` | 13378 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 13267 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 13326 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13350 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 13356 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 13337 |
| File > Export > Tempo Track... |  | unavailable | `false` | 13349 |
| File > Import > Audio CD... |  | unavailable | `false` | 13304 |
| File > Import > Audio... |  | available | `true` | 13300 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13318 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 13314 |
| File > Import > OMF... |  | unavailable | `false` | 13304 |
| File > Import > Tempo Track... |  | unavailable | `false` | 13304 |
| File > Import > Track Archive... |  | unavailable | `false` | 13304 |
| File > Import > Video File... |  | unavailable | `false` | 13304 |
| File > Key Commands... |  | available | `true` | 13384 |
| File > New Library... |  | unavailable | `false` | 13261 |
| File > New Project... | Ctrl+N | available | `true` | 13257 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 13271 |
| File > Open Library... |  | unavailable | `false` | 13266 |
| File > Open... | Ctrl+O | available | `true` | 13262 |
| File > Page Setup... |  | unavailable | `false` | 13296 |
| File > Preferences... |  | available | `true` | 13380 |
| File > Print... |  | unavailable | `false` | 13297 |
| File > Quit | Ctrl+Q | available | `true` | 13392 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 13389 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 13377 |
| File > Revert |  | unavailable | `false` | 13294 |
| File > Save | Ctrl+S | available | `true` | 13283 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 13287 |
| File > Save Library... |  | unavailable | `false` | 13293 |
| File > Save as Template... |  | unavailable | `false` | 13292 |
| Help > About OpenTrack... |  | available | `true` | 14972 |
| Help > Credits and Copyrights |  | available | `true` | 14968 |
| Help > OpenTrack on the Web |  | unavailable | `false` | 14966 |
| Help > Registration |  | unavailable | `false` | 14967 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 14309 |
| MIDI > CC Automation Setup... |  | unavailable | `false` | 14217 |
| MIDI > Drum Map Setup... |  | state-dependent | `self.project_open` | 14235 |
| MIDI > Input device |  | informational | `` | 14244 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14183 |
| MIDI > Logical Presets |  | state-dependent | `self.data.project.selected_midi_part_id.is_some()` | 14224 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 14302 |
| MIDI > No MIDI input devices found |  | informational | `` | 14283 |
| MIDI > Open Drum Editor |  | state-dependent | `midi_editor_available` | 14170 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14144 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 14140 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14157 |
| MIDI > Open Score Editor |  | unavailable | `false` | 14169 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 14175 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 14288 |
| MIDI > Reset |  | available | `true` | 14239 |
| MIDI > VST Expression |  | unavailable | `false` | 14217 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 14505 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 14516 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 14538 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 14471 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 14456 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 14564 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 14482 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 14564 |
| Media > Import Audio CD... |  | unavailable | `false` | 14564 |
| Media > Import Medium... |  | available | `true` | 14447 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 14452 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 14579 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 14579 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 14579 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 14553 |
| Media > Open Loop Browser | F6 | available | `true` | 14436 |
| Media > Open MediaBay | F5 | available | `true` | 14431 |
| Media > Open Pool Window | Ctrl+P | unavailable | `false` | 14430 |
| Media > Open Sound Browser | F7 | available | `true` | 14441 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 14542 |
| Media > Reconstruct |  | unavailable | `false` | 14564 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 14601 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 14493 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 14460 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 14592 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 14596 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 14527 |
| Project > Add Track > Arranger |  | unavailable | `false` | 13814 |
| Project > Add Track > Audio |  | available | `true` | 13801 |
| Project > Add Track > Browse Sounds... |  | unavailable | `false` | 13814 |
| Project > Add Track > FX Channel |  | unavailable | `false` | 13814 |
| Project > Add Track > Folder |  | unavailable | `false` | 13814 |
| Project > Add Track > Group Channel |  | available | `true` | 13801 |
| Project > Add Track > Instrument |  | available | `true` | 13801 |
| Project > Add Track > MIDI |  | available | `true` | 13801 |
| Project > Add Track > Marker |  | unavailable | `false` | 13814 |
| Project > Add Track > Ruler |  | unavailable | `false` | 13814 |
| Project > Add Track > Signature |  | unavailable | `false` | 13814 |
| Project > Add Track > Tempo |  | unavailable | `false` | 13814 |
| Project > Add Track > Transpose |  | unavailable | `false` | 13814 |
| Project > Add Track > Video |  | unavailable | `false` | 13814 |
| Project > Auto Fades Settings... |  | available | `true` | 13942 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 13950 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 13931 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 13926 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 13895 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 13922 |
| Project > New from Template > <label> |  | available | `true` | 13889 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 13885 |
| Project > Notepad |  | state-dependent | `self.project_open` | 13938 |
| Project > Pool | Ctrl+P | available | `true` | 13860 |
| Project > Project Setup... | Shift+S | available | `true` | 13918 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 13907 |
| Project > Remove Empty Tracks |  | unavailable | `false` | 13850 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 13868 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 13864 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 13946 |
| Project > Tempo Track | Ctrl+T | unavailable | `false` | 13954 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 13853 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 13853 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 14418 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 14418 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 14418 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 14418 |
| Scores > Align Elements > Bottom |  | unavailable | `false` | 14358 |
| Scores > Align Elements > Distribute Horizontally |  | unavailable | `false` | 14358 |
| Scores > Align Elements > Distribute Vertically |  | unavailable | `false` | 14358 |
| Scores > Align Elements > Left |  | unavailable | `false` | 14358 |
| Scores > Align Elements > Right |  | unavailable | `false` | 14358 |
| Scores > Align Elements > Top |  | unavailable | `false` | 14358 |
| Scores > Auto Layout... |  | unavailable | `false` | 14416 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14332 |
| Scores > Convert to Grace Note |  | unavailable | `false` | 14344 |
| Scores > Flip |  | unavailable | `false` | 14344 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14401 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14388 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 14414 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14375 |
| Scores > Group/Ungroup Notes |  | unavailable | `false` | 14344 |
| Scores > Hide/Show |  | unavailable | `false` | 14344 |
| Scores > Insert Slur |  | unavailable | `false` | 14344 |
| Scores > Make Chord Symbols |  | unavailable | `false` | 14370 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 14370 |
| Scores > Open Layout... |  | unavailable | `false` | 14344 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14319 |
| Scores > Page Mode |  | unavailable | `false` | 14344 |
| Scores > Reset Layout... |  | unavailable | `false` | 14416 |
| Scores > Settings... |  | unavailable | `false` | 14344 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 14632 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 14632 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 14632 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 14632 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 14632 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 14632 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 14632 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 14632 |
| Transport > Metronome On | C | available | `true` | 14730 |
| Transport > Metronome Setup... |  | available | `true` | 14726 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 14632 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 14643 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 14637 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 14659 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 14654 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 14648 |
| Transport > Post-roll from Selection End |  | state-dependent | `can_roll` | 14669 |
| Transport > Post-roll from Selection Start |  | state-dependent | `can_roll` | 14665 |
| Transport > Pre-roll to Selection End |  | state-dependent | `can_roll` | 14677 |
| Transport > Pre-roll to Selection Start |  | state-dependent | `can_roll` | 14673 |
| Transport > Precount On |  | available | `true` | 14736 |
| Transport > Project Synchronization Setup... |  | unavailable | `false` | 14756 |
| Transport > Retrospective Record | Shift+Num * | unavailable | `false` | 14756 |
| Transport > Roll Lengths... > Post-roll (beats, 0-64) |  | informational | `` | 14698 |
| Transport > Roll Lengths... > Pre-roll (beats, 0-64) |  | informational | `` | 14682 |
| Transport > Roll Lengths... > The selection is the selected audio events plus the selected MIDI part. |  | informational | `` | 14714 |
| Transport > Start Record at Left Locator |  | unavailable | `false` | 14724 |
| Transport > Transport Panel | F2 | available | `true` | 14614 |
| Transport > Use External Sync | T | unavailable | `false` | 14756 |
| Transport > Use Post-roll |  | unavailable | `false` | 14723 |
| Transport > Use Pre-roll |  | unavailable | `false` | 14722 |
| Window > Windows... |  | unavailable | `false` | 14893 |
| Window > Workspaces > <lock_label> |  | available | `true` | 14888 |
| Window > Workspaces > <name> |  | available | `*active` | 14866 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 14872 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 15019 |
| Devices > Device Setup... |  | available | `true` | 15022 |
| Devices > VST Connections... | F4 | unavailable | `false` | 15026 |
| Edit > Key Commands... |  | available | `true` | 15010 |
| Edit > Redo |  | unavailable | `false` | 15008 |
| Edit > Undo |  | unavailable | `false` | 15007 |
| File > New Project... | Ctrl+N | available | `true` | 14989 |
| File > Open... | Ctrl+O | available | `true` | 14993 |
| File > Quit | Alt+F4 | available | `true` | 15001 |
| File > Save | Ctrl+S | unavailable | `false` | 14998 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 14999 |
| Help > About OpenTrack... |  | available | `true` | 15034 |
| MIDI > No project is open |  | unavailable | `false` | 15030 |
| Media > No project is open |  | unavailable | `false` | 15030 |
| Project > No project is open |  | unavailable | `false` | 15016 |
| Scores > No project is open |  | unavailable | `false` | 15030 |
| Transport > No project is open |  | unavailable | `false` | 15030 |
| Window > No project is open |  | unavailable | `false` | 15030 |

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

