# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0x372ba93b654ff4b1`; keymap CSV `0xa9f1eb00c81948c7`
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

273 leaves: 40 available, 141 state-dependent, 84 unavailable, 8 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14203 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 14214 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 14152 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14163 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 14176 |
| Audio > Plug-ins > Distortion |  | unavailable | `false` | 14131 |
| Audio > Plug-ins > Dynamics |  | unavailable | `false` | 14131 |
| Audio > Plug-ins > EQ |  | unavailable | `false` | 14131 |
| Audio > Plug-ins > Filter |  | unavailable | `false` | 14131 |
| Audio > Plug-ins > Modulation |  | unavailable | `false` | 14131 |
| Audio > Plug-ins > Other |  | unavailable | `false` | 14131 |
| Audio > Plug-ins > Reverb |  | unavailable | `false` | 14131 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 14125 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 14074 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 14078 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 14086 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 14106 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 14102 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 14094 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 14117 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 14090 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 14082 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 14098 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 14121 |
| Audio > Realtime Processing > Add Plug-in |  | unavailable | `false` | 14191 |
| Audio > Realtime Processing > Direct Offline Processing |  | unavailable | `false` | 14191 |
| Audio > Realtime Processing > Remove Plug-in |  | unavailable | `false` | 14191 |
| Audio > Spectrum Analyzer |  | unavailable | `false` | 14144 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 14145 |
| Devices > Control Room Mixer |  | unavailable | `false` | 14876 |
| Devices > Control Room Overview |  | unavailable | `false` | 14876 |
| Devices > Device Setup... |  | available | `true` | 14846 |
| Devices > MIDI Device Manager |  | unavailable | `false` | 14876 |
| Devices > MMC Master |  | unavailable | `false` | 14876 |
| Devices > Mixer | F3 | available | `true` | 14885 |
| Devices > Plug-in Information |  | available | `true` | 14855 |
| Devices > Realtime Performance... | F12 | available | `true` | 14860 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 14850 |
| Devices > VST Instruments | F11 | available | `true` | 14864 |
| Devices > Video Window | F8 | unavailable | `false` | 14942 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 13766 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 13802 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 13624 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 13755 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 13497 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 13532 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 13486 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 13647 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 13651 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 13705 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 13790 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 13713 |
| Edit > Glue |  | state-dependent | `can_glue` | 13632 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 13739 |
| Edit > History... |  | unavailable | `false` | 13477 |
| Edit > Key Commands... |  | available | `true` | 13854 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 13747 |
| Edit > Macros |  | unavailable | `false` | 13852 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 13628 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13566 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 13718 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 13722 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 13549 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 13727 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 13735 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && self.audio_event_clipboard.is_some()` | 13501 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 13826 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 13821 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13591 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 13676 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 13680 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 13663 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 13668 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 13672 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 13471 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 13709 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 13686 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 13698 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 13694 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 13690 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 13611 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 13553 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 13525 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 13579 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 13465 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 13743 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 13751 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 13731 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 13846 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 13846 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 13846 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 13846 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 13846 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 13846 |
| File > Back up Project... |  | unavailable | `false` | 13357 |
| File > Cleanup... |  | unavailable | `false` | 13444 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 13333 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 13392 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13416 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 13422 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 13403 |
| File > Export > Tempo Track... |  | unavailable | `false` | 13415 |
| File > Import > Audio CD... |  | unavailable | `false` | 13370 |
| File > Import > Audio... |  | available | `true` | 13366 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 13384 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 13380 |
| File > Import > OMF... |  | unavailable | `false` | 13370 |
| File > Import > Tempo Track... |  | unavailable | `false` | 13370 |
| File > Import > Track Archive... |  | unavailable | `false` | 13370 |
| File > Import > Video File... |  | unavailable | `false` | 13370 |
| File > Key Commands... |  | available | `true` | 13450 |
| File > New Library... |  | unavailable | `false` | 13327 |
| File > New Project... | Ctrl+N | available | `true` | 13323 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 13337 |
| File > Open Library... |  | unavailable | `false` | 13332 |
| File > Open... | Ctrl+O | available | `true` | 13328 |
| File > Page Setup... |  | unavailable | `false` | 13362 |
| File > Preferences... |  | available | `true` | 13446 |
| File > Print... |  | unavailable | `false` | 13363 |
| File > Quit | Ctrl+Q | available | `true` | 13458 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 13455 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 13443 |
| File > Revert |  | unavailable | `false` | 13360 |
| File > Save | Ctrl+S | available | `true` | 13349 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 13353 |
| File > Save Library... |  | unavailable | `false` | 13359 |
| File > Save as Template... |  | unavailable | `false` | 13358 |
| Help > About OpenTrack... |  | available | `true` | 15053 |
| Help > Credits and Copyrights |  | available | `true` | 15049 |
| Help > OpenTrack on the Web |  | unavailable | `false` | 15047 |
| Help > Registration |  | unavailable | `false` | 15048 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 14388 |
| MIDI > CC Automation Setup... |  | state-dependent | `self.project_open` | 14299 |
| MIDI > Drum Map Setup... |  | state-dependent | `self.project_open` | 14314 |
| MIDI > Input device |  | informational | `` | 14323 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14264 |
| MIDI > Logical Presets |  | state-dependent | `self.data.project.selected_midi_part_id.is_some()` | 14303 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 14381 |
| MIDI > No MIDI input devices found |  | informational | `` | 14362 |
| MIDI > Open Drum Editor |  | state-dependent | `midi_editor_available` | 14251 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14225 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 14221 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14238 |
| MIDI > Open Score Editor |  | unavailable | `false` | 14250 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 14256 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 14367 |
| MIDI > Reset |  | available | `true` | 14318 |
| MIDI > VST Expression |  | unavailable | `false` | 14298 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 14584 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 14595 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 14617 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 14550 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 14535 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 14643 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 14561 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 14643 |
| Media > Import Audio CD... |  | unavailable | `false` | 14643 |
| Media > Import Medium... |  | available | `true` | 14526 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 14531 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 14658 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 14658 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 14658 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 14632 |
| Media > Open Loop Browser | F6 | available | `true` | 14515 |
| Media > Open MediaBay | F5 | available | `true` | 14510 |
| Media > Open Pool Window | Ctrl+P | unavailable | `false` | 14509 |
| Media > Open Sound Browser | F7 | available | `true` | 14520 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 14621 |
| Media > Reconstruct |  | unavailable | `false` | 14643 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 14681 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 14572 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 14539 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 14672 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 14676 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 14606 |
| Project > Add Track > Arranger |  | unavailable | `false` | 13883 |
| Project > Add Track > Audio |  | available | `true` | 13869 |
| Project > Add Track > Browse Sounds... |  | unavailable | `false` | 13883 |
| Project > Add Track > FX Channel |  | available | `true` | 13869 |
| Project > Add Track > Folder |  | available | `true` | 13869 |
| Project > Add Track > Group Channel |  | available | `true` | 13869 |
| Project > Add Track > Instrument |  | available | `true` | 13869 |
| Project > Add Track > MIDI |  | available | `true` | 13869 |
| Project > Add Track > Marker |  | unavailable | `false` | 13883 |
| Project > Add Track > Ruler |  | unavailable | `false` | 13883 |
| Project > Add Track > Signature |  | unavailable | `false` | 13883 |
| Project > Add Track > Tempo |  | unavailable | `false` | 13883 |
| Project > Add Track > Transpose |  | unavailable | `false` | 13883 |
| Project > Add Track > Video |  | unavailable | `false` | 13883 |
| Project > Auto Fades Settings... |  | available | `true` | 14023 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 14031 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 14012 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 14007 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 13972 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 13999 |
| Project > New from Template > <label> |  | available | `true` | 13966 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 13962 |
| Project > Notepad |  | state-dependent | `self.project_open` | 14019 |
| Project > Pool | Ctrl+P | available | `true` | 13937 |
| Project > Project Setup... | Shift+S | available | `true` | 13995 |
| Project > Regions... |  | state-dependent | `self.project_open` | 14003 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 13984 |
| Project > Remove Empty Tracks |  | unavailable | `false` | 13917 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 13945 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 13941 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 14027 |
| Project > Show Marker Lane |  | state-dependent | `self.project_open` | 13926 |
| Project > Tempo Track | Ctrl+T | unavailable | `false` | 14035 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 13920 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 13920 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 14497 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 14497 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 14497 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 14497 |
| Scores > Align Elements > Bottom |  | unavailable | `false` | 14437 |
| Scores > Align Elements > Distribute Horizontally |  | unavailable | `false` | 14437 |
| Scores > Align Elements > Distribute Vertically |  | unavailable | `false` | 14437 |
| Scores > Align Elements > Left |  | unavailable | `false` | 14437 |
| Scores > Align Elements > Right |  | unavailable | `false` | 14437 |
| Scores > Align Elements > Top |  | unavailable | `false` | 14437 |
| Scores > Auto Layout... |  | unavailable | `false` | 14495 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14411 |
| Scores > Convert to Grace Note |  | unavailable | `false` | 14423 |
| Scores > Flip |  | unavailable | `false` | 14423 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14480 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14467 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 14493 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14454 |
| Scores > Group/Ungroup Notes |  | unavailable | `false` | 14423 |
| Scores > Hide/Show |  | unavailable | `false` | 14423 |
| Scores > Insert Slur |  | unavailable | `false` | 14423 |
| Scores > Make Chord Symbols |  | unavailable | `false` | 14449 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 14449 |
| Scores > Open Layout... |  | unavailable | `false` | 14423 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 14398 |
| Scores > Page Mode |  | unavailable | `false` | 14423 |
| Scores > Reset Layout... |  | unavailable | `false` | 14495 |
| Scores > Settings... |  | unavailable | `false` | 14423 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 14712 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 14712 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 14712 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 14712 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 14712 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 14712 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 14712 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 14712 |
| Transport > Metronome On | C | available | `true` | 14810 |
| Transport > Metronome Setup... |  | available | `true` | 14806 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 14712 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 14723 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 14717 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 14739 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 14734 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 14728 |
| Transport > Post-roll from Selection End |  | state-dependent | `can_roll` | 14749 |
| Transport > Post-roll from Selection Start |  | state-dependent | `can_roll` | 14745 |
| Transport > Pre-roll to Selection End |  | state-dependent | `can_roll` | 14757 |
| Transport > Pre-roll to Selection Start |  | state-dependent | `can_roll` | 14753 |
| Transport > Precount On |  | available | `true` | 14816 |
| Transport > Project Synchronization Setup... |  | unavailable | `false` | 14836 |
| Transport > Retrospective Record | Shift+Num * | unavailable | `false` | 14836 |
| Transport > Roll Lengths... > Post-roll (beats, 0-64) |  | informational | `` | 14778 |
| Transport > Roll Lengths... > Pre-roll (beats, 0-64) |  | informational | `` | 14762 |
| Transport > Roll Lengths... > The selection is the selected audio events plus the selected MIDI part. |  | informational | `` | 14794 |
| Transport > Start Record at Left Locator |  | unavailable | `false` | 14804 |
| Transport > Transport Panel | F2 | available | `true` | 14694 |
| Transport > Use External Sync | T | unavailable | `false` | 14836 |
| Transport > Use Post-roll |  | unavailable | `false` | 14803 |
| Transport > Use Pre-roll |  | unavailable | `false` | 14802 |
| Window > Windows... |  | unavailable | `false` | 14974 |
| Window > Workspaces > <lock_label> |  | available | `true` | 14969 |
| Window > Workspaces > <name> |  | available | `*active` | 14947 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 14953 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 15100 |
| Devices > Device Setup... |  | available | `true` | 15103 |
| Devices > VST Connections... | F4 | unavailable | `false` | 15107 |
| Edit > Key Commands... |  | available | `true` | 15091 |
| Edit > Redo |  | unavailable | `false` | 15089 |
| Edit > Undo |  | unavailable | `false` | 15088 |
| File > New Project... | Ctrl+N | available | `true` | 15070 |
| File > Open... | Ctrl+O | available | `true` | 15074 |
| File > Quit | Alt+F4 | available | `true` | 15082 |
| File > Save | Ctrl+S | unavailable | `false` | 15079 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 15080 |
| Help > About OpenTrack... |  | available | `true` | 15115 |
| MIDI > No project is open |  | unavailable | `false` | 15111 |
| Media > No project is open |  | unavailable | `false` | 15111 |
| Project > No project is open |  | unavailable | `false` | 15097 |
| Scores > No project is open |  | unavailable | `false` | 15111 |
| Transport > No project is open |  | unavailable | `false` | 15111 |
| Window > No project is open |  | unavailable | `false` | 15111 |

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

