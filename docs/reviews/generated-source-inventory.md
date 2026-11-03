# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0x34757a0e41cc867e`; keymap CSV `0xa9f1eb00c81948c7`
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

277 leaves: 60 available, 176 state-dependent, 33 unavailable, 8 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14922 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 14933 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14836 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14847 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14860 |
| Audio > Plug-ins > Distortion |  | state-dependent | `self.project_open` | 14812 |
| Audio > Plug-ins > Dynamics |  | state-dependent | `self.project_open` | 14800 |
| Audio > Plug-ins > EQ |  | state-dependent | `self.project_open` | 14804 |
| Audio > Plug-ins > Filter |  | state-dependent | `self.project_open` | 14808 |
| Audio > Plug-ins > Modulation |  | state-dependent | `self.project_open` | 14816 |
| Audio > Plug-ins > Other |  | state-dependent | `self.project_open` | 14820 |
| Audio > Plug-ins > Reverb |  | state-dependent | `self.project_open` | 14796 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 14787 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 14736 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 14740 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 14748 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 14768 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 14764 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 14756 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 14779 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 14752 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 14744 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 14760 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 14783 |
| Audio > Realtime Processing > Add Plug-in > <label> |  | available | `true` | 14907 |
| Audio > Realtime Processing > Direct Offline Processing |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14877 |
| Audio > Realtime Processing > Remove Plug-in |  | state-dependent | `has_event` | 14914 |
| Audio > Spectrum Analyzer |  | available | `true` | 14825 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 14829 |
| Devices > Control Room Mixer |  | available | `true` | 15671 |
| Devices > Control Room Overview |  | available | `true` | 15675 |
| Devices > Device Setup... |  | available | `true` | 15637 |
| Devices > MIDI Device Manager |  | available | `true` | 15667 |
| Devices > MMC Master |  | available | `true` | 15679 |
| Devices > Mixer | F3 | available | `true` | 15683 |
| Devices > Plug-in Information |  | available | `true` | 15646 |
| Devices > Realtime Performance... | F12 | available | `true` | 15651 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 15641 |
| Devices > VST Instruments | F11 | available | `true` | 15655 |
| Devices > Video Window | F8 | unavailable | `false` | 15740 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 14361 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 14397 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 14219 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 14350 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 14092 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 14127 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 14081 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 14242 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 14246 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 14300 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 14385 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 14308 |
| Edit > Glue |  | state-dependent | `can_glue` | 14227 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 14334 |
| Edit > History... |  | state-dependent | `self.project_open` | 14069 |
| Edit > Key Commands... |  | available | `true` | 14452 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 14342 |
| Edit > Macros |  | state-dependent | `self.project_open` | 14447 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 14223 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14161 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 14313 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 14317 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 14144 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 14322 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 14330 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && self.audio_event_clipboard.is_some()` | 14096 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 14421 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 14416 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14186 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 14271 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 14275 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 14258 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 14263 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 14267 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 14063 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 14304 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 14281 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 14293 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 14289 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 14285 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 14206 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14148 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 14120 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14174 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 14057 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 14338 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 14346 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 14326 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 14441 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 14441 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 14441 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 14441 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 14441 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 14441 |
| File > Back up Project... |  | state-dependent | `self.project_open && self.project_folder_managed` | 13840 |
| File > Cleanup... |  | state-dependent | `self.project_open && self.project_folder_managed && !self.is_recording()` | 14003 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 13816 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 13930 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13964 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 13970 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 13941 |
| File > Export > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 13953 |
| File > Export > Track Archive (OpenTrack)... |  | state-dependent | `self.project_open && self.track_archive_task.is_none()` | 13990 |
| File > Import > Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 13890 |
| File > Import > Audio... |  | available | `true` | 13875 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13922 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 13918 |
| File > Import > OMF... |  | unavailable | `false` | 13901 |
| File > Import > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 13879 |
| File > Import > Track Archive (OpenTrack v1)... |  | state-dependent | `self.project_open && !self.is_recording() && self.track_archive_import_task.is_none()` | 13905 |
| File > Import > Video File... |  | unavailable | `false` | 13901 |
| File > Key Commands... |  | available | `true` | 14019 |
| File > New Library... |  | available | `true` | 13804 |
| File > New Project... | Ctrl+N | available | `true` | 13800 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 13820 |
| File > Open Library... |  | available | `true` | 13812 |
| File > Open... | Ctrl+O | available | `true` | 13808 |
| File > Page Setup... |  | unavailable | `false` | 13871 |
| File > Preferences... |  | available | `true` | 14015 |
| File > Print... |  | unavailable | `false` | 13872 |
| File > Quit | Ctrl+Q | available | `true` | 14050 |
| File > Recent Projects > <label> |  | available | `true` | 14033 |
| File > Recent Projects > Clear Recent Projects |  | available | `true` | 14043 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 14025 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 14002 |
| File > Revert |  | state-dependent | `self.project_open` | 13866 |
| File > Save | Ctrl+S | available | `true` | 13832 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 13836 |
| File > Save Library... |  | state-dependent | `self.project_open && self.data.project.selected_track.is_some()` | 13855 |
| File > Save as Template... |  | state-dependent | `self.project_open` | 13851 |
| Help > About OpenTrack... |  | available | `true` | 15860 |
| Help > Credits and Copyrights |  | available | `true` | 15856 |
| Help > OpenTrack on the Web |  | available | `true` | 15848 |
| Help > Registration |  | available | `true` | 15852 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 15117 |
| MIDI > CC Automation Setup... |  | state-dependent | `self.project_open` | 15028 |
| MIDI > Drum Map Setup... |  | state-dependent | `self.project_open` | 15043 |
| MIDI > Input device |  | informational | `` | 15052 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14993 |
| MIDI > Logical Presets |  | state-dependent | `self.data.project.selected_midi_part_id.is_some()` | 15032 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 15110 |
| MIDI > No MIDI input devices found |  | informational | `` | 15091 |
| MIDI > Open Drum Editor |  | state-dependent | `midi_editor_available` | 14980 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14944 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 14940 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14957 |
| MIDI > Open Score Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14969 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 14985 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 15096 |
| MIDI > Reset |  | available | `true` | 15047 |
| MIDI > VST Expression |  | unavailable | `false` | 15027 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15322 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15333 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 15355 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15288 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 15273 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 15396 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 15299 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 15396 |
| Media > Import Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 15381 |
| Media > Import Medium... |  | available | `true` | 15264 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 15269 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 15409 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 15409 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 15409 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 15370 |
| Media > Open Loop Browser | F6 | available | `true` | 15253 |
| Media > Open MediaBay | F5 | available | `true` | 15248 |
| Media > Open Pool Window | Ctrl+P | available | `true` | 15244 |
| Media > Open Sound Browser | F7 | available | `true` | 15258 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 15359 |
| Media > Reconstruct |  | state-dependent | `self.project_open` | 15392 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 15432 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 15310 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15277 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 15423 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 15427 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 15344 |
| Project > <preview_label> |  | state-dependent | `self.project_open && (self.arranger_preview.is_some() \|\| !self.data.project.arranger.sequence.is_empty())` | 14653 |
| Project > Add Track > Arranger |  | state-dependent | `self.project_open` | 14481 |
| Project > Add Track > Audio |  | available | `true` | 14467 |
| Project > Add Track > Browse Sounds... |  | available | `true` | 14519 |
| Project > Add Track > FX Channel |  | available | `true` | 14467 |
| Project > Add Track > Folder |  | available | `true` | 14467 |
| Project > Add Track > Group Channel |  | available | `true` | 14467 |
| Project > Add Track > Instrument |  | available | `true` | 14467 |
| Project > Add Track > MIDI |  | available | `true` | 14467 |
| Project > Add Track > Marker |  | state-dependent | `self.project_open` | 14490 |
| Project > Add Track > Ruler |  | state-dependent | `self.project_open` | 14494 |
| Project > Add Track > Signature |  | state-dependent | `self.project_open` | 14498 |
| Project > Add Track > Tempo |  | state-dependent | `self.project_open` | 14502 |
| Project > Add Track > Transpose |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14506 |
| Project > Add Track > Video |  | unavailable | `false` | 14526 |
| Project > Auto Fades Settings... |  | available | `true` | 14682 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 14690 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 14671 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 14666 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 14613 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 14640 |
| Project > New from Template > <label> |  | available | `true` | 14607 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 14603 |
| Project > Notepad |  | state-dependent | `self.project_open` | 14678 |
| Project > Pool | Ctrl+P | available | `true` | 14578 |
| Project > Project Setup... | Shift+S | available | `true` | 14636 |
| Project > Regions... |  | state-dependent | `self.project_open` | 14644 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 14625 |
| Project > Remove Empty Tracks |  | state-dependent | `self.project_open && !self.is_recording()` | 14548 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 14586 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 14582 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 14686 |
| Project > Show Marker Lane |  | state-dependent | `self.project_open` | 14567 |
| Project > Tempo Track | Ctrl+T | state-dependent | `self.project_open` | 14694 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 14561 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 14561 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 15232 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 15232 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 15232 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 15232 |
| Scores > Align Elements > Bottom |  | unavailable | `false` | 15172 |
| Scores > Align Elements > Distribute Horizontally |  | unavailable | `false` | 15172 |
| Scores > Align Elements > Distribute Vertically |  | unavailable | `false` | 15172 |
| Scores > Align Elements > Left |  | unavailable | `false` | 15172 |
| Scores > Align Elements > Right |  | unavailable | `false` | 15172 |
| Scores > Align Elements > Top |  | unavailable | `false` | 15172 |
| Scores > Auto Layout... |  | unavailable | `false` | 15230 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15140 |
| Scores > Convert to Grace Note |  | unavailable | `false` | 15160 |
| Scores > Flip |  | unavailable | `false` | 15160 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15215 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15202 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 15228 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15189 |
| Scores > Group/Ungroup Notes |  | unavailable | `false` | 15160 |
| Scores > Hide/Show |  | unavailable | `false` | 15160 |
| Scores > Insert Slur |  | unavailable | `false` | 15160 |
| Scores > Make Chord Symbols |  | unavailable | `false` | 15184 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 15184 |
| Scores > Open Layout... |  | unavailable | `false` | 15160 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 15127 |
| Scores > Page Mode |  | available | `self.score_page_mode` | 15152 |
| Scores > Reset Layout... |  | unavailable | `false` | 15230 |
| Scores > Settings... |  | available | `true` | 15156 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 15463 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 15463 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 15463 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 15463 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 15463 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 15463 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 15463 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 15463 |
| Transport > Metronome On | C | available | `true` | 15593 |
| Transport > Metronome Setup... |  | available | `true` | 15589 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 15463 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 15474 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 15468 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 15490 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 15485 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 15479 |
| Transport > Post-roll from Selection End |  | state-dependent | `can_roll` | 15500 |
| Transport > Post-roll from Selection Start |  | state-dependent | `can_roll` | 15496 |
| Transport > Pre-roll to Selection End |  | state-dependent | `can_roll` | 15508 |
| Transport > Pre-roll to Selection Start |  | state-dependent | `can_roll` | 15504 |
| Transport > Precount On |  | available | `true` | 15599 |
| Transport > Project Synchronization Setup... |  | available | `true` | 15630 |
| Transport > Retrospective Record | Shift+Num * | state-dependent | `self.project_open && !self.is_recording()` | 15619 |
| Transport > Roll Lengths... > Post-roll (beats, 0-64) |  | informational | `` | 15529 |
| Transport > Roll Lengths... > Pre-roll (beats, 0-64) |  | informational | `` | 15513 |
| Transport > Roll Lengths... > The selection is the selected audio events plus the selected MIDI part. |  | informational | `` | 15545 |
| Transport > Start Record at Left Locator |  | state-dependent | `self.project_open` | 15577 |
| Transport > Transport Panel | F2 | available | `true` | 15445 |
| Transport > Use External Sync | T | unavailable | `false` | 15634 |
| Transport > Use Post-roll |  | available | `self.use_post_roll` | 15565 |
| Transport > Use Pre-roll |  | available | `self.use_pre_roll` | 15553 |
| Window > Windows... |  | available | `true` | 15772 |
| Window > Workspaces > <lock_label> |  | available | `true` | 15767 |
| Window > Workspaces > <name> |  | available | `*active` | 15745 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 15751 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 15907 |
| Devices > Device Setup... |  | available | `true` | 15910 |
| Devices > VST Connections... | F4 | unavailable | `false` | 15914 |
| Edit > Key Commands... |  | available | `true` | 15898 |
| Edit > Redo |  | unavailable | `false` | 15896 |
| Edit > Undo |  | unavailable | `false` | 15895 |
| File > New Project... | Ctrl+N | available | `true` | 15877 |
| File > Open... | Ctrl+O | available | `true` | 15881 |
| File > Quit | Alt+F4 | available | `true` | 15889 |
| File > Save | Ctrl+S | unavailable | `false` | 15886 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 15887 |
| Help > About OpenTrack... |  | available | `true` | 15922 |
| MIDI > No project is open |  | unavailable | `false` | 15918 |
| Media > No project is open |  | unavailable | `false` | 15918 |
| Project > No project is open |  | unavailable | `false` | 15904 |
| Scores > No project is open |  | unavailable | `false` | 15918 |
| Transport > No project is open |  | unavailable | `false` | 15918 |
| Window > No project is open |  | unavailable | `false` | 15918 |

No duplicate full paths.

## Bundled Windows reference keymap

152 distinct command IDs; 150 have a `dispatch_reference_command` arm; 2 have none.

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

- Video (`F8`, Devices) — `devices.video`
- Use External Sync (`T`, Transport) — `transport.use_external_sync`

