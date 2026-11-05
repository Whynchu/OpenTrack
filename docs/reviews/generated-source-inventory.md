# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0xa55db496895d38e2`; keymap CSV `0xa9f1eb00c81948c7`
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

277 leaves: 63 available, 189 state-dependent, 17 unavailable, 8 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 15687 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 15698 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 15601 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 15612 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 15625 |
| Audio > Plug-ins > Distortion |  | state-dependent | `self.project_open` | 15577 |
| Audio > Plug-ins > Dynamics |  | state-dependent | `self.project_open` | 15565 |
| Audio > Plug-ins > EQ |  | state-dependent | `self.project_open` | 15569 |
| Audio > Plug-ins > Filter |  | state-dependent | `self.project_open` | 15573 |
| Audio > Plug-ins > Modulation |  | state-dependent | `self.project_open` | 15581 |
| Audio > Plug-ins > Other |  | state-dependent | `self.project_open` | 15585 |
| Audio > Plug-ins > Reverb |  | state-dependent | `self.project_open` | 15561 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 15552 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 15501 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 15505 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 15513 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 15533 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 15529 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 15521 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 15544 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 15517 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 15509 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 15525 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 15548 |
| Audio > Realtime Processing > Add Plug-in > <label> |  | available | `true` | 15672 |
| Audio > Realtime Processing > Direct Offline Processing |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 15642 |
| Audio > Realtime Processing > Remove Plug-in |  | state-dependent | `has_event` | 15679 |
| Audio > Spectrum Analyzer |  | available | `true` | 15590 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 15594 |
| Devices > Control Room Mixer |  | available | `true` | 16488 |
| Devices > Control Room Overview |  | available | `true` | 16492 |
| Devices > Device Setup... |  | available | `true` | 16453 |
| Devices > MIDI Device Manager |  | available | `true` | 16484 |
| Devices > MMC Master |  | available | `true` | 16496 |
| Devices > Mixer | F3 | available | `true` | 16500 |
| Devices > Plug-in Information |  | available | `true` | 16462 |
| Devices > Realtime Performance... | F12 | available | `true` | 16467 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 16457 |
| Devices > VST Instruments | F11 | available | `true` | 16471 |
| Devices > Video Window | F8 | state-dependent | `opentrack_video::backend_available() && self.project_open` | 16557 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 15125 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 15161 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 14983 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 15114 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 14845 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 14891 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 14830 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 15006 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 15010 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 15064 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 15149 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 15072 |
| Edit > Glue |  | state-dependent | `can_glue` | 14991 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 15098 |
| Edit > History... |  | state-dependent | `self.project_open` | 14819 |
| Edit > Key Commands... |  | available | `true` | 15216 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 15106 |
| Edit > Macros |  | state-dependent | `self.project_open` | 15211 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 14987 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14925 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 15077 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 15081 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 14908 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 15086 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 15094 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && (self.audio_event_clipboard.is_some() \|\| self.midi_note_clipboard.is_some())` | 14855 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 15185 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 15180 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14950 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 15035 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 15039 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 15022 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 15027 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 15031 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 14813 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 15068 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 15045 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 15057 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 15053 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 15049 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 14970 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14912 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 14884 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14938 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 14807 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 15102 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 15110 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 15090 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 15205 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 15205 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 15205 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 15205 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 15205 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 15205 |
| File > Back up Project... |  | state-dependent | `self.project_open && self.project_folder_managed` | 14590 |
| File > Cleanup... |  | state-dependent | `self.project_open && self.project_folder_managed && !self.is_recording()` | 14753 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 14566 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 14680 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 14714 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 14720 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 14691 |
| File > Export > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 14703 |
| File > Export > Track Archive (OpenTrack)... |  | state-dependent | `self.project_open && self.track_archive_task.is_none()` | 14740 |
| File > Import > Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 14640 |
| File > Import > Audio... |  | available | `true` | 14625 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 14672 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 14668 |
| File > Import > OMF... |  | unavailable | `false` | 14651 |
| File > Import > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 14629 |
| File > Import > Track Archive (OpenTrack v1)... |  | state-dependent | `self.project_open && !self.is_recording() && self.track_archive_import_task.is_none()` | 14655 |
| File > Import > Video File... |  | unavailable | `false` | 14651 |
| File > Key Commands... |  | available | `true` | 14769 |
| File > New Library... |  | available | `true` | 14554 |
| File > New Project... | Ctrl+N | available | `true` | 14550 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 14570 |
| File > Open Library... |  | available | `true` | 14562 |
| File > Open... | Ctrl+O | available | `true` | 14558 |
| File > Page Setup... |  | unavailable | `false` | 14621 |
| File > Preferences... |  | available | `true` | 14765 |
| File > Print... |  | unavailable | `false` | 14622 |
| File > Quit | Ctrl+Q | available | `true` | 14800 |
| File > Recent Projects > <label> |  | available | `true` | 14783 |
| File > Recent Projects > Clear Recent Projects |  | available | `true` | 14793 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 14775 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 14752 |
| File > Revert |  | state-dependent | `self.project_open` | 14616 |
| File > Save | Ctrl+S | available | `true` | 14582 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 14586 |
| File > Save Library... |  | state-dependent | `self.project_open && self.data.project.selected_track.is_some()` | 14605 |
| File > Save as Template... |  | state-dependent | `self.project_open` | 14601 |
| Help > About OpenTrack... |  | available | `true` | 16687 |
| Help > Credits and Copyrights |  | available | `true` | 16683 |
| Help > OpenTrack on the Web |  | available | `true` | 16675 |
| Help > Registration |  | available | `true` | 16679 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 15882 |
| MIDI > CC Automation Setup... |  | state-dependent | `self.project_open` | 15793 |
| MIDI > Drum Map Setup... |  | state-dependent | `self.project_open` | 15808 |
| MIDI > Input device |  | informational | `` | 15817 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15758 |
| MIDI > Logical Presets |  | state-dependent | `self.data.project.selected_midi_part_id.is_some()` | 15797 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 15875 |
| MIDI > No MIDI input devices found |  | informational | `` | 15856 |
| MIDI > Open Drum Editor |  | state-dependent | `midi_editor_available` | 15745 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 15709 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 15705 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15722 |
| MIDI > Open Score Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15734 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 15750 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 15861 |
| MIDI > Reset |  | available | `true` | 15812 |
| MIDI > VST Expression |  | unavailable | `false` | 15792 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 16137 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 16148 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 16170 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 16103 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 16088 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 16211 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 16114 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 16211 |
| Media > Import Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 16196 |
| Media > Import Medium... |  | available | `true` | 16079 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 16084 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 16224 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 16224 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 16224 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 16185 |
| Media > Open Loop Browser | F6 | available | `true` | 16068 |
| Media > Open MediaBay | F5 | available | `true` | 16063 |
| Media > Open Pool Window | Ctrl+P | available | `true` | 16059 |
| Media > Open Sound Browser | F7 | available | `true` | 16073 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 16174 |
| Media > Reconstruct |  | state-dependent | `self.project_open` | 16207 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 16248 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 16125 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 16092 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 16239 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 16243 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 16159 |
| Project > <preview_label> |  | state-dependent | `self.project_open && (self.arranger_preview.is_some() \|\| !self.data.project.arranger.sequence.is_empty())` | 15418 |
| Project > Add Track > Arranger |  | state-dependent | `self.project_open` | 15246 |
| Project > Add Track > Audio |  | available | `true` | 15231 |
| Project > Add Track > Browse Sounds... |  | available | `true` | 15284 |
| Project > Add Track > FX Channel |  | available | `true` | 15231 |
| Project > Add Track > Folder |  | available | `true` | 15231 |
| Project > Add Track > Group Channel |  | available | `true` | 15231 |
| Project > Add Track > Instrument |  | available | `true` | 15231 |
| Project > Add Track > MIDI |  | available | `true` | 15231 |
| Project > Add Track > Marker |  | state-dependent | `self.project_open` | 15255 |
| Project > Add Track > Ruler |  | state-dependent | `self.project_open` | 15259 |
| Project > Add Track > Signature |  | state-dependent | `self.project_open` | 15263 |
| Project > Add Track > Tempo |  | state-dependent | `self.project_open` | 15267 |
| Project > Add Track > Transpose |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15271 |
| Project > Add Track > Video |  | unavailable | `false` | 15291 |
| Project > Auto Fades Settings... |  | available | `true` | 15447 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 15455 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 15436 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 15431 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 15378 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 15405 |
| Project > New from Template > <label> |  | available | `true` | 15372 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 15368 |
| Project > Notepad |  | state-dependent | `self.project_open` | 15443 |
| Project > Pool | Ctrl+P | available | `true` | 15343 |
| Project > Project Setup... | Shift+S | available | `true` | 15401 |
| Project > Regions... |  | state-dependent | `self.project_open` | 15409 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 15390 |
| Project > Remove Empty Tracks |  | state-dependent | `self.project_open && !self.is_recording()` | 15313 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 15351 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 15347 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 15451 |
| Project > Show Marker Lane |  | state-dependent | `self.project_open` | 15332 |
| Project > Tempo Track | Ctrl+T | state-dependent | `self.project_open` | 15459 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 15326 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 15326 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 16047 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 16047 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 16047 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 16047 |
| Scores > Align Elements > Bottom |  | state-dependent | `self.project_open` | 15968 |
| Scores > Align Elements > Distribute Horizontally |  | state-dependent | `self.project_open` | 15974 |
| Scores > Align Elements > Distribute Vertically |  | state-dependent | `self.project_open` | 15982 |
| Scores > Align Elements > Left |  | state-dependent | `self.project_open` | 15950 |
| Scores > Align Elements > Right |  | state-dependent | `self.project_open` | 15956 |
| Scores > Align Elements > Top |  | state-dependent | `self.project_open` | 15962 |
| Scores > Auto Layout... |  | available | `true` | 16038 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15905 |
| Scores > Convert to Grace Note |  | state-dependent | `self.project_open` | 15937 |
| Scores > Flip |  | state-dependent | `self.project_open` | 15933 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 16023 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 16010 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 16036 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15997 |
| Scores > Group/Ungroup Notes |  | state-dependent | `self.project_open` | 15945 |
| Scores > Hide/Show |  | state-dependent | `self.project_open` | 15929 |
| Scores > Insert Slur |  | state-dependent | `self.project_open` | 15941 |
| Scores > Make Chord Symbols |  | state-dependent | `self.project_open` | 15991 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 15995 |
| Scores > Open Layout... |  | available | `true` | 15925 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 15892 |
| Scores > Page Mode |  | available | `self.score_page_mode` | 15917 |
| Scores > Reset Layout... |  | available | `true` | 16042 |
| Scores > Settings... |  | available | `true` | 15921 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 16279 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 16279 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 16279 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 16279 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 16279 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 16279 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 16279 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 16279 |
| Transport > Metronome On | C | available | `true` | 16409 |
| Transport > Metronome Setup... |  | available | `true` | 16405 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 16279 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 16290 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 16284 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 16306 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 16301 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 16295 |
| Transport > Post-roll from Selection End |  | state-dependent | `can_roll` | 16316 |
| Transport > Post-roll from Selection Start |  | state-dependent | `can_roll` | 16312 |
| Transport > Pre-roll to Selection End |  | state-dependent | `can_roll` | 16324 |
| Transport > Pre-roll to Selection Start |  | state-dependent | `can_roll` | 16320 |
| Transport > Precount On |  | available | `true` | 16415 |
| Transport > Project Synchronization Setup... |  | available | `true` | 16446 |
| Transport > Retrospective Record | Shift+Num * | state-dependent | `self.project_open && !self.is_recording()` | 16435 |
| Transport > Roll Lengths... > Post-roll (beats, 0-64) |  | informational | `` | 16345 |
| Transport > Roll Lengths... > Pre-roll (beats, 0-64) |  | informational | `` | 16329 |
| Transport > Roll Lengths... > The selection is the selected audio events plus the selected MIDI part. |  | informational | `` | 16361 |
| Transport > Start Record at Left Locator |  | state-dependent | `self.project_open` | 16393 |
| Transport > Transport Panel | F2 | available | `true` | 16261 |
| Transport > Use External Sync | T | unavailable | `false` | 16450 |
| Transport > Use Post-roll |  | available | `self.use_post_roll` | 16381 |
| Transport > Use Pre-roll |  | available | `self.use_pre_roll` | 16369 |
| Window > Windows... |  | available | `true` | 16599 |
| Window > Workspaces > <lock_label> |  | available | `true` | 16594 |
| Window > Workspaces > <name> |  | available | `*active` | 16572 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 16578 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 16737 |
| Devices > Device Setup... |  | available | `true` | 16740 |
| Devices > VST Connections... | F4 | unavailable | `false` | 16744 |
| Edit > Key Commands... |  | available | `true` | 16728 |
| Edit > Redo |  | unavailable | `false` | 16726 |
| Edit > Undo |  | unavailable | `false` | 16725 |
| File > New Project... | Ctrl+N | available | `true` | 16707 |
| File > Open... | Ctrl+O | available | `true` | 16711 |
| File > Quit | Alt+F4 | available | `true` | 16719 |
| File > Save | Ctrl+S | unavailable | `false` | 16716 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 16717 |
| Help > About OpenTrack... |  | available | `true` | 16752 |
| MIDI > No project is open |  | unavailable | `false` | 16748 |
| Media > No project is open |  | unavailable | `false` | 16748 |
| Project > No project is open |  | unavailable | `false` | 16734 |
| Scores > No project is open |  | unavailable | `false` | 16748 |
| Transport > No project is open |  | unavailable | `false` | 16748 |
| Window > No project is open |  | unavailable | `false` | 16748 |

No duplicate full paths.

## Bundled Windows reference keymap

152 distinct command IDs; 151 have a `dispatch_reference_command` arm; 1 have none.

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
| `devices.video` | Devices | Video | F8 | yes |
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
| `edit.open_score_editor` | Edit | Open Score Editor | Ctrl+R | yes |
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
| `project.open_tempo_track_editor` | Project | Open Tempo Track Editor | Ctrl+T | yes |
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
| `transport.retrospective_record` | Transport | Retrospective Record | Shift+Num * | yes |
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

- Use External Sync (`T`, Transport) — `transport.use_external_sync`

