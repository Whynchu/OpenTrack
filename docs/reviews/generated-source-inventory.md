# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0x11e47f14b63b41f0`; keymap CSV `0xa9f1eb00c81948c7`
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

275 leaves: 44 available, 161 state-dependent, 62 unavailable, 8 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14663 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 14674 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14577 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14588 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14601 |
| Audio > Plug-ins > Distortion |  | unavailable | `false` | 14553 |
| Audio > Plug-ins > Dynamics |  | unavailable | `false` | 14553 |
| Audio > Plug-ins > EQ |  | unavailable | `false` | 14553 |
| Audio > Plug-ins > Filter |  | unavailable | `false` | 14553 |
| Audio > Plug-ins > Modulation |  | unavailable | `false` | 14553 |
| Audio > Plug-ins > Other |  | unavailable | `false` | 14553 |
| Audio > Plug-ins > Reverb |  | unavailable | `false` | 14553 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 14547 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 14496 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 14500 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 14508 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 14528 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 14524 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 14516 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 14539 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 14512 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 14504 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 14520 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 14543 |
| Audio > Realtime Processing > Add Plug-in > <label> |  | available | `true` | 14648 |
| Audio > Realtime Processing > Direct Offline Processing |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14618 |
| Audio > Realtime Processing > Remove Plug-in |  | state-dependent | `has_event` | 14655 |
| Audio > Spectrum Analyzer |  | available | `true` | 14566 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 14570 |
| Devices > Control Room Mixer |  | unavailable | `false` | 15363 |
| Devices > Control Room Overview |  | unavailable | `false` | 15363 |
| Devices > Device Setup... |  | available | `true` | 15329 |
| Devices > MIDI Device Manager |  | available | `true` | 15359 |
| Devices > MMC Master |  | unavailable | `false` | 15363 |
| Devices > Mixer | F3 | available | `true` | 15371 |
| Devices > Plug-in Information |  | available | `true` | 15338 |
| Devices > Realtime Performance... | F12 | available | `true` | 15343 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 15333 |
| Devices > VST Instruments | F11 | available | `true` | 15347 |
| Devices > Video Window | F8 | unavailable | `false` | 15428 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 14128 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 14164 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 13986 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 14117 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 13859 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 13894 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 13848 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 14009 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 14013 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 14067 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 14152 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 14075 |
| Edit > Glue |  | state-dependent | `can_glue` | 13994 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 14101 |
| Edit > History... |  | state-dependent | `self.project_open` | 13836 |
| Edit > Key Commands... |  | available | `true` | 14216 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 14109 |
| Edit > Macros |  | unavailable | `false` | 14214 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 13990 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13928 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 14080 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 14084 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 13911 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 14089 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 14097 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && self.audio_event_clipboard.is_some()` | 13863 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 14188 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 14183 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13953 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 14038 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 14042 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 14025 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 14030 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 14034 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 13830 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 14071 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 14048 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 14060 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 14056 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 14052 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 13973 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13915 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 13887 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13941 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 13824 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 14105 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 14113 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 14093 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 14208 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 14208 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 14208 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 14208 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 14208 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 14208 |
| File > Back up Project... |  | unavailable | `false` | 13663 |
| File > Cleanup... |  | unavailable | `false` | 13803 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 13639 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 13730 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13764 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 13770 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 13741 |
| File > Export > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 13753 |
| File > Export > Track Archive (OpenTrack)... |  | state-dependent | `self.project_open && self.track_archive_task.is_none()` | 13790 |
| File > Import > Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 13690 |
| File > Import > Audio... |  | available | `true` | 13675 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13722 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 13718 |
| File > Import > OMF... |  | unavailable | `false` | 13701 |
| File > Import > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 13679 |
| File > Import > Track Archive (OpenTrack v1)... |  | state-dependent | `self.project_open && !self.is_recording() && self.track_archive_import_task.is_none()` | 13705 |
| File > Import > Video File... |  | unavailable | `false` | 13701 |
| File > Key Commands... |  | available | `true` | 13809 |
| File > New Library... |  | unavailable | `false` | 13633 |
| File > New Project... | Ctrl+N | available | `true` | 13629 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 13643 |
| File > Open Library... |  | unavailable | `false` | 13638 |
| File > Open... | Ctrl+O | available | `true` | 13634 |
| File > Page Setup... |  | unavailable | `false` | 13671 |
| File > Preferences... |  | available | `true` | 13805 |
| File > Print... |  | unavailable | `false` | 13672 |
| File > Quit | Ctrl+Q | available | `true` | 13817 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 13814 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 13802 |
| File > Revert |  | unavailable | `false` | 13669 |
| File > Save | Ctrl+S | available | `true` | 13655 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 13659 |
| File > Save Library... |  | unavailable | `false` | 13668 |
| File > Save as Template... |  | state-dependent | `self.project_open` | 13664 |
| Help > About OpenTrack... |  | available | `true` | 15539 |
| Help > Credits and Copyrights |  | available | `true` | 15535 |
| Help > OpenTrack on the Web |  | unavailable | `false` | 15533 |
| Help > Registration |  | unavailable | `false` | 15534 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 14848 |
| MIDI > CC Automation Setup... |  | state-dependent | `self.project_open` | 14759 |
| MIDI > Drum Map Setup... |  | state-dependent | `self.project_open` | 14774 |
| MIDI > Input device |  | informational | `` | 14783 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14724 |
| MIDI > Logical Presets |  | state-dependent | `self.data.project.selected_midi_part_id.is_some()` | 14763 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 14841 |
| MIDI > No MIDI input devices found |  | informational | `` | 14822 |
| MIDI > Open Drum Editor |  | state-dependent | `midi_editor_available` | 14711 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14685 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 14681 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14698 |
| MIDI > Open Score Editor |  | unavailable | `false` | 14710 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 14716 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 14827 |
| MIDI > Reset |  | available | `true` | 14778 |
| MIDI > VST Expression |  | unavailable | `false` | 14758 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15047 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15058 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 15080 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15013 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 14998 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 15117 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 15024 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 15117 |
| Media > Import Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 15106 |
| Media > Import Medium... |  | available | `true` | 14989 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 14994 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 15131 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 15131 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 15131 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 15095 |
| Media > Open Loop Browser | F6 | available | `true` | 14978 |
| Media > Open MediaBay | F5 | available | `true` | 14973 |
| Media > Open Pool Window | Ctrl+P | available | `true` | 14969 |
| Media > Open Sound Browser | F7 | available | `true` | 14983 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 15084 |
| Media > Reconstruct |  | unavailable | `false` | 15117 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 15154 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 15035 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15002 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 15145 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 15149 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 15069 |
| Project > <preview_label> |  | state-dependent | `self.project_open && (self.arranger_preview.is_some() \|\| !self.data.project.arranger.sequence.is_empty())` | 14413 |
| Project > Add Track > Arranger |  | state-dependent | `self.project_open` | 14245 |
| Project > Add Track > Audio |  | available | `true` | 14231 |
| Project > Add Track > Browse Sounds... |  | unavailable | `false` | 14283 |
| Project > Add Track > FX Channel |  | available | `true` | 14231 |
| Project > Add Track > Folder |  | available | `true` | 14231 |
| Project > Add Track > Group Channel |  | available | `true` | 14231 |
| Project > Add Track > Instrument |  | available | `true` | 14231 |
| Project > Add Track > MIDI |  | available | `true` | 14231 |
| Project > Add Track > Marker |  | state-dependent | `self.project_open` | 14254 |
| Project > Add Track > Ruler |  | state-dependent | `self.project_open` | 14258 |
| Project > Add Track > Signature |  | state-dependent | `self.project_open` | 14262 |
| Project > Add Track > Tempo |  | state-dependent | `self.project_open` | 14266 |
| Project > Add Track > Transpose |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14270 |
| Project > Add Track > Video |  | unavailable | `false` | 14283 |
| Project > Auto Fades Settings... |  | available | `true` | 14442 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 14450 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 14431 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 14426 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 14373 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 14400 |
| Project > New from Template > <label> |  | available | `true` | 14367 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 14363 |
| Project > Notepad |  | state-dependent | `self.project_open` | 14438 |
| Project > Pool | Ctrl+P | available | `true` | 14338 |
| Project > Project Setup... | Shift+S | available | `true` | 14396 |
| Project > Regions... |  | state-dependent | `self.project_open` | 14404 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 14385 |
| Project > Remove Empty Tracks |  | state-dependent | `self.project_open && !self.is_recording()` | 14308 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 14346 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 14342 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 14446 |
| Project > Show Marker Lane |  | state-dependent | `self.project_open` | 14327 |
| Project > Tempo Track | Ctrl+T | state-dependent | `self.project_open` | 14454 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 14321 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 14321 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 14957 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 14957 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 14957 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 14957 |
| Scores > Align Elements > Bottom |  | unavailable | `false` | 14897 |
| Scores > Align Elements > Distribute Horizontally |  | unavailable | `false` | 14897 |
| Scores > Align Elements > Distribute Vertically |  | unavailable | `false` | 14897 |
| Scores > Align Elements > Left |  | unavailable | `false` | 14897 |
| Scores > Align Elements > Right |  | unavailable | `false` | 14897 |
| Scores > Align Elements > Top |  | unavailable | `false` | 14897 |
| Scores > Auto Layout... |  | unavailable | `false` | 14955 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14871 |
| Scores > Convert to Grace Note |  | unavailable | `false` | 14883 |
| Scores > Flip |  | unavailable | `false` | 14883 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14940 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14927 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 14953 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14914 |
| Scores > Group/Ungroup Notes |  | unavailable | `false` | 14883 |
| Scores > Hide/Show |  | unavailable | `false` | 14883 |
| Scores > Insert Slur |  | unavailable | `false` | 14883 |
| Scores > Make Chord Symbols |  | unavailable | `false` | 14909 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 14909 |
| Scores > Open Layout... |  | unavailable | `false` | 14883 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14858 |
| Scores > Page Mode |  | unavailable | `false` | 14883 |
| Scores > Reset Layout... |  | unavailable | `false` | 14955 |
| Scores > Settings... |  | unavailable | `false` | 14883 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 15185 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 15185 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 15185 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 15185 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 15185 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 15185 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 15185 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 15185 |
| Transport > Metronome On | C | available | `true` | 15283 |
| Transport > Metronome Setup... |  | available | `true` | 15279 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 15185 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 15196 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 15190 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 15212 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 15207 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 15201 |
| Transport > Post-roll from Selection End |  | state-dependent | `can_roll` | 15222 |
| Transport > Post-roll from Selection Start |  | state-dependent | `can_roll` | 15218 |
| Transport > Pre-roll to Selection End |  | state-dependent | `can_roll` | 15230 |
| Transport > Pre-roll to Selection Start |  | state-dependent | `can_roll` | 15226 |
| Transport > Precount On |  | available | `true` | 15289 |
| Transport > Project Synchronization Setup... |  | unavailable | `false` | 15320 |
| Transport > Retrospective Record | Shift+Num * | state-dependent | `self.project_open && !self.is_recording()` | 15309 |
| Transport > Roll Lengths... > Post-roll (beats, 0-64) |  | informational | `` | 15251 |
| Transport > Roll Lengths... > Pre-roll (beats, 0-64) |  | informational | `` | 15235 |
| Transport > Roll Lengths... > The selection is the selected audio events plus the selected MIDI part. |  | informational | `` | 15267 |
| Transport > Start Record at Left Locator |  | unavailable | `false` | 15277 |
| Transport > Transport Panel | F2 | available | `true` | 15167 |
| Transport > Use External Sync | T | unavailable | `false` | 15320 |
| Transport > Use Post-roll |  | unavailable | `false` | 15276 |
| Transport > Use Pre-roll |  | unavailable | `false` | 15275 |
| Window > Windows... |  | unavailable | `false` | 15460 |
| Window > Workspaces > <lock_label> |  | available | `true` | 15455 |
| Window > Workspaces > <name> |  | available | `*active` | 15433 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 15439 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 15586 |
| Devices > Device Setup... |  | available | `true` | 15589 |
| Devices > VST Connections... | F4 | unavailable | `false` | 15593 |
| Edit > Key Commands... |  | available | `true` | 15577 |
| Edit > Redo |  | unavailable | `false` | 15575 |
| Edit > Undo |  | unavailable | `false` | 15574 |
| File > New Project... | Ctrl+N | available | `true` | 15556 |
| File > Open... | Ctrl+O | available | `true` | 15560 |
| File > Quit | Alt+F4 | available | `true` | 15568 |
| File > Save | Ctrl+S | unavailable | `false` | 15565 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 15566 |
| Help > About OpenTrack... |  | available | `true` | 15601 |
| MIDI > No project is open |  | unavailable | `false` | 15597 |
| Media > No project is open |  | unavailable | `false` | 15597 |
| Project > No project is open |  | unavailable | `false` | 15583 |
| Scores > No project is open |  | unavailable | `false` | 15597 |
| Transport > No project is open |  | unavailable | `false` | 15597 |
| Window > No project is open |  | unavailable | `false` | 15597 |

No duplicate full paths.

## Bundled Windows reference keymap

152 distinct command IDs; 149 have a `dispatch_reference_command` arm; 3 have none.

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
- Open Score Editor (`Ctrl+R`, Edit) — `edit.open_score_editor`
- Use External Sync (`T`, Transport) — `transport.use_external_sync`

