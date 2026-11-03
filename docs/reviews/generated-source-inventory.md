# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0x60011fd7bb171755`; keymap CSV `0xa9f1eb00c81948c7`
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

277 leaves: 63 available, 177 state-dependent, 29 unavailable, 8 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14948 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 14959 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14862 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14873 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14886 |
| Audio > Plug-ins > Distortion |  | state-dependent | `self.project_open` | 14838 |
| Audio > Plug-ins > Dynamics |  | state-dependent | `self.project_open` | 14826 |
| Audio > Plug-ins > EQ |  | state-dependent | `self.project_open` | 14830 |
| Audio > Plug-ins > Filter |  | state-dependent | `self.project_open` | 14834 |
| Audio > Plug-ins > Modulation |  | state-dependent | `self.project_open` | 14842 |
| Audio > Plug-ins > Other |  | state-dependent | `self.project_open` | 14846 |
| Audio > Plug-ins > Reverb |  | state-dependent | `self.project_open` | 14822 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 14813 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 14762 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 14766 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 14774 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 14794 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 14790 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 14782 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 14805 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 14778 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 14770 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 14786 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 14809 |
| Audio > Realtime Processing > Add Plug-in > <label> |  | available | `true` | 14933 |
| Audio > Realtime Processing > Direct Offline Processing |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14903 |
| Audio > Realtime Processing > Remove Plug-in |  | state-dependent | `has_event` | 14940 |
| Audio > Spectrum Analyzer |  | available | `true` | 14851 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 14855 |
| Devices > Control Room Mixer |  | available | `true` | 15707 |
| Devices > Control Room Overview |  | available | `true` | 15711 |
| Devices > Device Setup... |  | available | `true` | 15673 |
| Devices > MIDI Device Manager |  | available | `true` | 15703 |
| Devices > MMC Master |  | available | `true` | 15715 |
| Devices > Mixer | F3 | available | `true` | 15719 |
| Devices > Plug-in Information |  | available | `true` | 15682 |
| Devices > Realtime Performance... | F12 | available | `true` | 15687 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 15677 |
| Devices > VST Instruments | F11 | available | `true` | 15691 |
| Devices > Video Window | F8 | state-dependent | `opentrack_video::backend_available() && self.project_open` | 15776 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 14387 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 14423 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 14245 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 14376 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 14118 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 14153 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 14107 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 14268 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 14272 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 14326 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 14411 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 14334 |
| Edit > Glue |  | state-dependent | `can_glue` | 14253 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 14360 |
| Edit > History... |  | state-dependent | `self.project_open` | 14095 |
| Edit > Key Commands... |  | available | `true` | 14478 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 14368 |
| Edit > Macros |  | state-dependent | `self.project_open` | 14473 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 14249 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14187 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 14339 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 14343 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 14170 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 14348 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 14356 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && self.audio_event_clipboard.is_some()` | 14122 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 14447 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 14442 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14212 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 14297 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 14301 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 14284 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 14289 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 14293 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 14089 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 14330 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 14307 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 14319 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 14315 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 14311 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 14232 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14174 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 14146 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14200 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 14083 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 14364 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 14372 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 14352 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 14467 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 14467 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 14467 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 14467 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 14467 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 14467 |
| File > Back up Project... |  | state-dependent | `self.project_open && self.project_folder_managed` | 13866 |
| File > Cleanup... |  | state-dependent | `self.project_open && self.project_folder_managed && !self.is_recording()` | 14029 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 13842 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 13956 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13990 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 13996 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 13967 |
| File > Export > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 13979 |
| File > Export > Track Archive (OpenTrack)... |  | state-dependent | `self.project_open && self.track_archive_task.is_none()` | 14016 |
| File > Import > Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 13916 |
| File > Import > Audio... |  | available | `true` | 13901 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13948 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 13944 |
| File > Import > OMF... |  | unavailable | `false` | 13927 |
| File > Import > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 13905 |
| File > Import > Track Archive (OpenTrack v1)... |  | state-dependent | `self.project_open && !self.is_recording() && self.track_archive_import_task.is_none()` | 13931 |
| File > Import > Video File... |  | unavailable | `false` | 13927 |
| File > Key Commands... |  | available | `true` | 14045 |
| File > New Library... |  | available | `true` | 13830 |
| File > New Project... | Ctrl+N | available | `true` | 13826 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 13846 |
| File > Open Library... |  | available | `true` | 13838 |
| File > Open... | Ctrl+O | available | `true` | 13834 |
| File > Page Setup... |  | unavailable | `false` | 13897 |
| File > Preferences... |  | available | `true` | 14041 |
| File > Print... |  | unavailable | `false` | 13898 |
| File > Quit | Ctrl+Q | available | `true` | 14076 |
| File > Recent Projects > <label> |  | available | `true` | 14059 |
| File > Recent Projects > Clear Recent Projects |  | available | `true` | 14069 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 14051 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 14028 |
| File > Revert |  | state-dependent | `self.project_open` | 13892 |
| File > Save | Ctrl+S | available | `true` | 13858 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 13862 |
| File > Save Library... |  | state-dependent | `self.project_open && self.data.project.selected_track.is_some()` | 13881 |
| File > Save as Template... |  | state-dependent | `self.project_open` | 13877 |
| Help > About OpenTrack... |  | available | `true` | 15906 |
| Help > Credits and Copyrights |  | available | `true` | 15902 |
| Help > OpenTrack on the Web |  | available | `true` | 15894 |
| Help > Registration |  | available | `true` | 15898 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 15143 |
| MIDI > CC Automation Setup... |  | state-dependent | `self.project_open` | 15054 |
| MIDI > Drum Map Setup... |  | state-dependent | `self.project_open` | 15069 |
| MIDI > Input device |  | informational | `` | 15078 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15019 |
| MIDI > Logical Presets |  | state-dependent | `self.data.project.selected_midi_part_id.is_some()` | 15058 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 15136 |
| MIDI > No MIDI input devices found |  | informational | `` | 15117 |
| MIDI > Open Drum Editor |  | state-dependent | `midi_editor_available` | 15006 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14970 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 14966 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14983 |
| MIDI > Open Score Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14995 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 15011 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 15122 |
| MIDI > Reset |  | available | `true` | 15073 |
| MIDI > VST Expression |  | unavailable | `false` | 15053 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15358 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15369 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 15391 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15324 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 15309 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 15432 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 15335 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 15432 |
| Media > Import Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 15417 |
| Media > Import Medium... |  | available | `true` | 15300 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 15305 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 15445 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 15445 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 15445 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 15406 |
| Media > Open Loop Browser | F6 | available | `true` | 15289 |
| Media > Open MediaBay | F5 | available | `true` | 15284 |
| Media > Open Pool Window | Ctrl+P | available | `true` | 15280 |
| Media > Open Sound Browser | F7 | available | `true` | 15294 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 15395 |
| Media > Reconstruct |  | state-dependent | `self.project_open` | 15428 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 15468 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 15346 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15313 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 15459 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 15463 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 15380 |
| Project > <preview_label> |  | state-dependent | `self.project_open && (self.arranger_preview.is_some() \|\| !self.data.project.arranger.sequence.is_empty())` | 14679 |
| Project > Add Track > Arranger |  | state-dependent | `self.project_open` | 14507 |
| Project > Add Track > Audio |  | available | `true` | 14493 |
| Project > Add Track > Browse Sounds... |  | available | `true` | 14545 |
| Project > Add Track > FX Channel |  | available | `true` | 14493 |
| Project > Add Track > Folder |  | available | `true` | 14493 |
| Project > Add Track > Group Channel |  | available | `true` | 14493 |
| Project > Add Track > Instrument |  | available | `true` | 14493 |
| Project > Add Track > MIDI |  | available | `true` | 14493 |
| Project > Add Track > Marker |  | state-dependent | `self.project_open` | 14516 |
| Project > Add Track > Ruler |  | state-dependent | `self.project_open` | 14520 |
| Project > Add Track > Signature |  | state-dependent | `self.project_open` | 14524 |
| Project > Add Track > Tempo |  | state-dependent | `self.project_open` | 14528 |
| Project > Add Track > Transpose |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14532 |
| Project > Add Track > Video |  | unavailable | `false` | 14552 |
| Project > Auto Fades Settings... |  | available | `true` | 14708 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 14716 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 14697 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 14692 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 14639 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 14666 |
| Project > New from Template > <label> |  | available | `true` | 14633 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 14629 |
| Project > Notepad |  | state-dependent | `self.project_open` | 14704 |
| Project > Pool | Ctrl+P | available | `true` | 14604 |
| Project > Project Setup... | Shift+S | available | `true` | 14662 |
| Project > Regions... |  | state-dependent | `self.project_open` | 14670 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 14651 |
| Project > Remove Empty Tracks |  | state-dependent | `self.project_open && !self.is_recording()` | 14574 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 14612 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 14608 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 14712 |
| Project > Show Marker Lane |  | state-dependent | `self.project_open` | 14593 |
| Project > Tempo Track | Ctrl+T | state-dependent | `self.project_open` | 14720 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 14587 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 14587 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 15268 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 15268 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 15268 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 15268 |
| Scores > Align Elements > Bottom |  | unavailable | `false` | 15201 |
| Scores > Align Elements > Distribute Horizontally |  | unavailable | `false` | 15201 |
| Scores > Align Elements > Distribute Vertically |  | unavailable | `false` | 15201 |
| Scores > Align Elements > Left |  | unavailable | `false` | 15201 |
| Scores > Align Elements > Right |  | unavailable | `false` | 15201 |
| Scores > Align Elements > Top |  | unavailable | `false` | 15201 |
| Scores > Auto Layout... |  | available | `true` | 15259 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15166 |
| Scores > Convert to Grace Note |  | unavailable | `false` | 15190 |
| Scores > Flip |  | unavailable | `false` | 15190 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15244 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15231 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 15257 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15218 |
| Scores > Group/Ungroup Notes |  | unavailable | `false` | 15190 |
| Scores > Hide/Show |  | unavailable | `false` | 15190 |
| Scores > Insert Slur |  | unavailable | `false` | 15190 |
| Scores > Make Chord Symbols |  | unavailable | `false` | 15213 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 15213 |
| Scores > Open Layout... |  | available | `true` | 15186 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 15153 |
| Scores > Page Mode |  | available | `self.score_page_mode` | 15178 |
| Scores > Reset Layout... |  | available | `true` | 15263 |
| Scores > Settings... |  | available | `true` | 15182 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 15499 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 15499 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 15499 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 15499 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 15499 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 15499 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 15499 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 15499 |
| Transport > Metronome On | C | available | `true` | 15629 |
| Transport > Metronome Setup... |  | available | `true` | 15625 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 15499 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 15510 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 15504 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 15526 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 15521 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 15515 |
| Transport > Post-roll from Selection End |  | state-dependent | `can_roll` | 15536 |
| Transport > Post-roll from Selection Start |  | state-dependent | `can_roll` | 15532 |
| Transport > Pre-roll to Selection End |  | state-dependent | `can_roll` | 15544 |
| Transport > Pre-roll to Selection Start |  | state-dependent | `can_roll` | 15540 |
| Transport > Precount On |  | available | `true` | 15635 |
| Transport > Project Synchronization Setup... |  | available | `true` | 15666 |
| Transport > Retrospective Record | Shift+Num * | state-dependent | `self.project_open && !self.is_recording()` | 15655 |
| Transport > Roll Lengths... > Post-roll (beats, 0-64) |  | informational | `` | 15565 |
| Transport > Roll Lengths... > Pre-roll (beats, 0-64) |  | informational | `` | 15549 |
| Transport > Roll Lengths... > The selection is the selected audio events plus the selected MIDI part. |  | informational | `` | 15581 |
| Transport > Start Record at Left Locator |  | state-dependent | `self.project_open` | 15613 |
| Transport > Transport Panel | F2 | available | `true` | 15481 |
| Transport > Use External Sync | T | unavailable | `false` | 15670 |
| Transport > Use Post-roll |  | available | `self.use_post_roll` | 15601 |
| Transport > Use Pre-roll |  | available | `self.use_pre_roll` | 15589 |
| Window > Windows... |  | available | `true` | 15818 |
| Window > Workspaces > <lock_label> |  | available | `true` | 15813 |
| Window > Workspaces > <name> |  | available | `*active` | 15791 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 15797 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 15953 |
| Devices > Device Setup... |  | available | `true` | 15956 |
| Devices > VST Connections... | F4 | unavailable | `false` | 15960 |
| Edit > Key Commands... |  | available | `true` | 15944 |
| Edit > Redo |  | unavailable | `false` | 15942 |
| Edit > Undo |  | unavailable | `false` | 15941 |
| File > New Project... | Ctrl+N | available | `true` | 15923 |
| File > Open... | Ctrl+O | available | `true` | 15927 |
| File > Quit | Alt+F4 | available | `true` | 15935 |
| File > Save | Ctrl+S | unavailable | `false` | 15932 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 15933 |
| Help > About OpenTrack... |  | available | `true` | 15968 |
| MIDI > No project is open |  | unavailable | `false` | 15964 |
| Media > No project is open |  | unavailable | `false` | 15964 |
| Project > No project is open |  | unavailable | `false` | 15950 |
| Scores > No project is open |  | unavailable | `false` | 15964 |
| Transport > No project is open |  | unavailable | `false` | 15964 |
| Window > No project is open |  | unavailable | `false` | 15964 |

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

