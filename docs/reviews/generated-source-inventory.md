# Generated main-menu and shortcut inventory

Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.
Do not edit by hand. Regenerate after changing either source:

```text
$env:UPDATE_SOURCE_INVENTORY = "1"
cargo test -p opentrack generated_source_inventory -- --nocapture
```

- Source snapshot: `app.rs` FNV-1a 64 `0x2bacc2d3c03db39e`; keymap CSV `0x21262b776e49dde4`
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
| Audio > Advanced > Detect Silence... |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 15279 |
| Audio > Advanced > Spectrum Analyzer... |  | available | `true` | 15290 |
| Audio > Hitpoints > Calculate Hitpoints |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 15193 |
| Audio > Hitpoints > Create Markers |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 15204 |
| Audio > Hitpoints > Create Regions |  | state-dependent | `self.project_open && !self.is_recording() && self.hitpoints_clip.is_some()` | 15217 |
| Audio > Plug-ins > Distortion |  | state-dependent | `self.project_open` | 15169 |
| Audio > Plug-ins > Dynamics |  | state-dependent | `self.project_open` | 15157 |
| Audio > Plug-ins > EQ |  | state-dependent | `self.project_open` | 15161 |
| Audio > Plug-ins > Filter |  | state-dependent | `self.project_open` | 15165 |
| Audio > Plug-ins > Modulation |  | state-dependent | `self.project_open` | 15173 |
| Audio > Plug-ins > Other |  | state-dependent | `self.project_open` | 15177 |
| Audio > Plug-ins > Reverb |  | state-dependent | `self.project_open` | 15153 |
| Audio > Process > Envelope... |  | state-dependent | `has_selection` | 15144 |
| Audio > Process > Fade In |  | state-dependent | `has_selection` | 15093 |
| Audio > Process > Fade Out |  | state-dependent | `has_selection` | 15097 |
| Audio > Process > Gain... |  | state-dependent | `has_selection` | 15105 |
| Audio > Process > Merge Clipboard |  | state-dependent | `has_selection && self.audio_event_clipboard.is_some()` | 15125 |
| Audio > Process > Normalize... |  | state-dependent | `has_selection` | 15121 |
| Audio > Process > Remove DC Offset |  | state-dependent | `has_selection` | 15113 |
| Audio > Process > Resample... |  | state-dependent | `has_selection` | 15136 |
| Audio > Process > Reverse |  | state-dependent | `has_selection` | 15109 |
| Audio > Process > Silence |  | state-dependent | `has_selection` | 15101 |
| Audio > Process > Stereo Flip |  | state-dependent | `has_selection` | 15117 |
| Audio > Process > Time Stretch... |  | state-dependent | `has_selection` | 15140 |
| Audio > Realtime Processing > Add Plug-in > <label> |  | available | `true` | 15264 |
| Audio > Realtime Processing > Direct Offline Processing |  | state-dependent | `self.project_open && !self.is_recording() && has_event` | 15234 |
| Audio > Realtime Processing > Remove Plug-in |  | state-dependent | `has_event` | 15271 |
| Audio > Spectrum Analyzer |  | available | `true` | 15182 |
| Audio > Statistics |  | state-dependent | `self.project_open` | 15186 |
| Devices > Control Room Mixer |  | available | `true` | 16078 |
| Devices > Control Room Overview |  | available | `true` | 16082 |
| Devices > Device Setup... |  | available | `true` | 16044 |
| Devices > MIDI Device Manager |  | available | `true` | 16074 |
| Devices > MMC Master |  | available | `true` | 16086 |
| Devices > Mixer | F3 | available | `true` | 16090 |
| Devices > Plug-in Information |  | available | `true` | 16053 |
| Devices > Realtime Performance... | F12 | available | `true` | 16058 |
| Devices > VST Connections... | F4 | state-dependent | `self.project_open` | 16048 |
| Devices > VST Instruments | F11 | available | `true` | 16062 |
| Devices > Video Window | F8 | state-dependent | `opentrack_video::backend_available() && self.project_open` | 16147 |
| Edit > Auto Select Events under Cursor |  | state-dependent | `self.project_open` | 14718 |
| Edit > Automation follows Events |  | state-dependent | `self.project_open` | 14754 |
| Edit > Bounce Selection in Place |  | state-dependent | `can_bounce` | 14576 |
| Edit > Convert to Real Copy |  | state-dependent | `can_duplicate && self.project_folder_managed` | 14707 |
| Edit > Copy | Ctrl+C | state-dependent | `can_copy` | 14449 |
| Edit > Crossfade | X | state-dependent | `can_crossfade` | 14484 |
| Edit > Cut | Ctrl+X | state-dependent | `can_cut` | 14438 |
| Edit > Delete | Delete | state-dependent | `can_delete` | 14599 |
| Edit > Delete with Source File | Shift+Delete | state-dependent | `can_delete_audio` | 14603 |
| Edit > Duplicate | Ctrl+D | state-dependent | `can_duplicate` | 14657 |
| Edit > Enlarge Selected Track |  | state-dependent | `self.data.project.selected_track.is_some()` | 14742 |
| Edit > Fill Loop |  | state-dependent | `can_duplicate` | 14665 |
| Edit > Glue |  | state-dependent | `can_glue` | 14584 |
| Edit > Group | Ctrl+G | state-dependent | `can_duplicate` | 14691 |
| Edit > History... |  | state-dependent | `self.project_open` | 14426 |
| Edit > Key Commands... |  | available | `true` | 14809 |
| Edit > Lock | Ctrl+Shift+L | state-dependent | `can_duplicate` | 14699 |
| Edit > Macros |  | state-dependent | `self.project_open` | 14804 |
| Edit > Make Take Active |  | state-dependent | `can_activate_take` | 14580 |
| Edit > Merge MIDI Parts |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14518 |
| Edit > Move to > Left Locator |  | state-dependent | `can_duplicate` | 14670 |
| Edit > Move to > Right Locator |  | state-dependent | `can_duplicate` | 14674 |
| Edit > Move to Cursor | Ctrl+L | state-dependent | `can_move_to_cursor` | 14501 |
| Edit > Mute Events | Shift+M | state-dependent | `can_duplicate` | 14679 |
| Edit > Mute/Unmute Objects | Alt+M | state-dependent | `can_duplicate` | 14687 |
| Edit > Paste | Ctrl+V | state-dependent | `self.project_open && !self.is_recording() && self.audio_event_clipboard.is_some()` | 14453 |
| Edit > Process Project Logical Editor |  | state-dependent | `self.project_open` | 14778 |
| Edit > Project Logical Editor... |  | state-dependent | `self.project_open` | 14773 |
| Edit > Quantize MIDI... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14543 |
| Edit > Range > Crop |  | state-dependent | `has_selection` | 14628 |
| Edit > Range > Delete Range |  | state-dependent | `has_selection` | 14632 |
| Edit > Range > Select in Loop |  | state-dependent | `self.project_open` | 14615 |
| Edit > Range > Set Left Locator |  | state-dependent | `has_selection` | 14620 |
| Edit > Range > Set Right Locator |  | state-dependent | `has_selection` | 14624 |
| Edit > Redo |  | state-dependent | `self.project_history.can_redo()` | 14420 |
| Edit > Repeat... | Ctrl+K | state-dependent | `can_duplicate` | 14661 |
| Edit > Select > All Events | Ctrl+A | state-dependent | `self.project_open` | 14638 |
| Edit > Select > Events under Cursor |  | state-dependent | `self.project_open` | 14650 |
| Edit > Select > Invert |  | state-dependent | `self.project_open` | 14646 |
| Edit > Select > None | Ctrl+Shift+A | state-dependent | `self.project_open` | 14642 |
| Edit > Split Loop |  | state-dependent | `self.project_open` | 14563 |
| Edit > Split MIDI Part at Cursor |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14505 |
| Edit > Split at Cursor | Alt+X | state-dependent | `can_split` | 14477 |
| Edit > Transpose / Velocity... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 14531 |
| Edit > Undo |  | state-dependent | `self.project_history.can_undo()` | 14414 |
| Edit > Ungroup | Ctrl+U | state-dependent | `can_duplicate` | 14695 |
| Edit > Unlock | Ctrl+Shift+U | state-dependent | `can_duplicate` | 14703 |
| Edit > Unmute Events | Shift+U | state-dependent | `can_duplicate` | 14683 |
| Edit > Zoom > Zoom In | H | state-dependent | `self.project_open` | 14798 |
| Edit > Zoom > Zoom In Vertically | Shift+F | state-dependent | `self.project_open` | 14798 |
| Edit > Zoom > Zoom Out | G | state-dependent | `self.project_open` | 14798 |
| Edit > Zoom > Zoom Out Vertically | Alt+S | state-dependent | `self.project_open` | 14798 |
| Edit > Zoom > Zoom to Project | Alt+Down | state-dependent | `self.project_open` | 14798 |
| Edit > Zoom > Zoom to Selection | Shift+E | state-dependent | `self.project_open` | 14798 |
| File > Back up Project... |  | state-dependent | `self.project_open && self.project_folder_managed` | 14197 |
| File > Cleanup... |  | state-dependent | `self.project_open && self.project_folder_managed && !self.is_recording()` | 14360 |
| File > Close | Ctrl+W | state-dependent | `self.project_open` | 14173 |
| File > Export > Audio Mixdown... |  | state-dependent | `self.audio_export_task.is_none()` | 14287 |
| File > Export > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 14321 |
| File > Export > MIDI Selection... |  | state-dependent | `self.midi_file_task.is_none()` | 14327 |
| File > Export > Selected Tracks (Stems)... |  | state-dependent | `self.audio_export_task.is_none() && self.project_open` | 14298 |
| File > Export > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 14310 |
| File > Export > Track Archive (OpenTrack)... |  | state-dependent | `self.project_open && self.track_archive_task.is_none()` | 14347 |
| File > Import > Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 14247 |
| File > Import > Audio... |  | available | `true` | 14232 |
| File > Import > MIDI File... |  | state-dependent | `self.midi_file_task.is_none()` | 14279 |
| File > Import > MusicXML... |  | state-dependent | `!self.is_recording()` | 14275 |
| File > Import > OMF... |  | unavailable | `false` | 14258 |
| File > Import > Tempo Track... |  | state-dependent | `self.project_open && !self.is_recording()` | 14236 |
| File > Import > Track Archive (OpenTrack v1)... |  | state-dependent | `self.project_open && !self.is_recording() && self.track_archive_import_task.is_none()` | 14262 |
| File > Import > Video File... |  | unavailable | `false` | 14258 |
| File > Key Commands... |  | available | `true` | 14376 |
| File > New Library... |  | available | `true` | 14161 |
| File > New Project... | Ctrl+N | available | `true` | 14157 |
| File > New Version | Ctrl+Alt+S | state-dependent | `self.project_open && !self.is_recording()` | 14177 |
| File > Open Library... |  | available | `true` | 14169 |
| File > Open... | Ctrl+O | available | `true` | 14165 |
| File > Page Setup... |  | unavailable | `false` | 14228 |
| File > Preferences... |  | available | `true` | 14372 |
| File > Print... |  | unavailable | `false` | 14229 |
| File > Quit | Ctrl+Q | available | `true` | 14407 |
| File > Recent Projects > <label> |  | available | `true` | 14390 |
| File > Recent Projects > Clear Recent Projects |  | available | `true` | 14400 |
| File > Recent Projects > No Recent Projects |  | unavailable | `false` | 14382 |
| File > Replace Audio in Video File... |  | unavailable | `false` | 14359 |
| File > Revert |  | state-dependent | `self.project_open` | 14223 |
| File > Save | Ctrl+S | available | `true` | 14189 |
| File > Save As... | Ctrl+Shift+S | available | `true` | 14193 |
| File > Save Library... |  | state-dependent | `self.project_open && self.data.project.selected_track.is_some()` | 14212 |
| File > Save as Template... |  | state-dependent | `self.project_open` | 14208 |
| Help > About OpenTrack... |  | available | `true` | 16277 |
| Help > Credits and Copyrights |  | available | `true` | 16273 |
| Help > OpenTrack on the Web |  | available | `true` | 16265 |
| Help > Registration |  | available | `true` | 16269 |
| MIDI > Arm a MIDI or Instrument track to monitor its input |  | informational | `` | 15474 |
| MIDI > CC Automation Setup... |  | state-dependent | `self.project_open` | 15385 |
| MIDI > Drum Map Setup... |  | state-dependent | `self.project_open` | 15400 |
| MIDI > Input device |  | informational | `` | 15409 |
| MIDI > Insert Velocities |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15350 |
| MIDI > Logical Presets |  | state-dependent | `self.data.project.selected_midi_part_id.is_some()` | 15389 |
| MIDI > MIDI Thru needs an audio output device |  | informational | `` | 15467 |
| MIDI > No MIDI input devices found |  | informational | `` | 15448 |
| MIDI > Open Drum Editor |  | state-dependent | `midi_editor_available` | 15337 |
| MIDI > Open In-Place Editor |  | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 15301 |
| MIDI > Open Key Editor |  | state-dependent | `midi_editor_available` | 15297 |
| MIDI > Open List Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15314 |
| MIDI > Open Score Editor |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15326 |
| MIDI > Quantize Setup... |  | state-dependent | `self.project_open` | 15342 |
| MIDI > Refresh MIDI input devices |  | available | `true` | 15453 |
| MIDI > Reset |  | available | `true` | 15404 |
| MIDI > VST Expression |  | unavailable | `false` | 15384 |
| Media > Conform Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15729 |
| Media > Convert Files... |  | state-dependent | `self.project_open && !self.is_recording()` | 15740 |
| Media > Create Folder |  | state-dependent | `self.project_open` | 15762 |
| Media > Empty Trash |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15695 |
| Media > Export Pool... |  | state-dependent | `self.project_open` | 15680 |
| Media > Extract Audio from Video File |  | unavailable | `false` | 15803 |
| Media > Find Missing Files... |  | state-dependent | `pool_available && !self.data.project.missing_media().is_empty()` | 15706 |
| Media > Generate Thumbnail Cache |  | unavailable | `false` | 15803 |
| Media > Import Audio CD... |  | state-dependent | `self.project_open && !self.is_recording()` | 15788 |
| Media > Import Medium... |  | available | `true` | 15671 |
| Media > Import Pool... |  | state-dependent | `self.project_open` | 15676 |
| Media > Insert into Project > Audio Track |  | state-dependent | `self.project_open` | 15816 |
| Media > Insert into Project > Instrument Track |  | state-dependent | `self.project_open` | 15816 |
| Media > Insert into Project > MIDI Track |  | state-dependent | `self.project_open` | 15816 |
| Media > Minimize File |  | state-dependent | `self.project_open && !self.is_recording()` | 15777 |
| Media > Open Loop Browser | F6 | available | `true` | 15660 |
| Media > Open MediaBay | F5 | available | `true` | 15655 |
| Media > Open Pool Window | Ctrl+P | available | `true` | 15651 |
| Media > Open Sound Browser | F7 | available | `true` | 15665 |
| Media > Prepare Archive... |  | state-dependent | `self.project_open && !self.is_recording()` | 15766 |
| Media > Reconstruct |  | state-dependent | `self.project_open` | 15799 |
| Media > Relink Unavailable Audio... |  | state-dependent | `!unavailable_audio_paths.is_empty()` | 15839 |
| Media > Remove Missing Files |  | state-dependent | `pool_available && !self.data.project.missing_unreferenced_media().is_empty()` | 15717 |
| Media > Remove Unused Media |  | state-dependent | `pool_available && !self.data.project.unused_media().is_empty()` | 15684 |
| Media > Search Media... |  | state-dependent | `self.project_open` | 15830 |
| Media > Select in Project |  | state-dependent | `self.project_open` | 15834 |
| Media > Set Pool Record Folder |  | state-dependent | `self.project_open` | 15751 |
| Project > <preview_label> |  | state-dependent | `self.project_open && (self.arranger_preview.is_some() \|\| !self.data.project.arranger.sequence.is_empty())` | 15010 |
| Project > Add Track > Arranger |  | state-dependent | `self.project_open` | 14838 |
| Project > Add Track > Audio |  | available | `true` | 14824 |
| Project > Add Track > Browse Sounds... |  | available | `true` | 14876 |
| Project > Add Track > FX Channel |  | available | `true` | 14824 |
| Project > Add Track > Folder |  | available | `true` | 14824 |
| Project > Add Track > Group Channel |  | available | `true` | 14824 |
| Project > Add Track > Instrument |  | available | `true` | 14824 |
| Project > Add Track > MIDI |  | available | `true` | 14824 |
| Project > Add Track > Marker |  | state-dependent | `self.project_open` | 14847 |
| Project > Add Track > Ruler |  | state-dependent | `self.project_open` | 14851 |
| Project > Add Track > Signature |  | state-dependent | `self.project_open` | 14855 |
| Project > Add Track > Tempo |  | state-dependent | `self.project_open` | 14859 |
| Project > Add Track > Transpose |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 14863 |
| Project > Add Track > Video |  | unavailable | `false` | 14883 |
| Project > Auto Fades Settings... |  | available | `true` | 15039 |
| Project > Automation Panel |  | state-dependent | `self.project_open` | 15047 |
| Project > Beat Calculator... |  | state-dependent | `self.project_open` | 15028 |
| Project > Browser | Ctrl+B | state-dependent | `self.project_open` | 15023 |
| Project > Consolidate Media |  | state-dependent | `self.project_folder_managed && !self.is_recording()` | 14970 |
| Project > Markers | Ctrl+M | state-dependent | `self.project_open` | 14997 |
| Project > New from Template > <label> |  | available | `true` | 14964 |
| Project > New from Template > No templates yet. Save one from a project. |  | informational | `` | 14960 |
| Project > Notepad |  | state-dependent | `self.project_open` | 15035 |
| Project > Pool | Ctrl+P | available | `true` | 14935 |
| Project > Project Setup... | Shift+S | available | `true` | 14993 |
| Project > Regions... |  | state-dependent | `self.project_open` | 15001 |
| Project > Relink Missing Media (missing_media)... |  | state-dependent | `self.project_open && !self.is_recording() && missing_media > 0` | 14982 |
| Project > Remove Empty Tracks |  | state-dependent | `self.project_open && !self.is_recording()` | 14905 |
| Project > Save as Template... |  | state-dependent | `self.project_open` | 14943 |
| Project > Session Health... |  | state-dependent | `self.project_open` | 14939 |
| Project > Set Timecode at Cursor |  | state-dependent | `self.project_open` | 15043 |
| Project > Show Marker Lane |  | state-dependent | `self.project_open` | 14924 |
| Project > Tempo Track | Ctrl+T | state-dependent | `self.project_open` | 15051 |
| Project > Track Folding > Collapse All |  | state-dependent | `self.project_open` | 14918 |
| Project > Track Folding > Expand All |  | state-dependent | `self.project_open` | 14918 |
| Scores > Advanced Layout > Insert Page Break |  | unavailable | `false` | 15639 |
| Scores > Advanced Layout > Move Bars |  | unavailable | `false` | 15639 |
| Scores > Advanced Layout > Optimize All |  | unavailable | `false` | 15639 |
| Scores > Advanced Layout > Remove Page Break |  | unavailable | `false` | 15639 |
| Scores > Align Elements > Bottom |  | state-dependent | `self.project_open` | 15560 |
| Scores > Align Elements > Distribute Horizontally |  | state-dependent | `self.project_open` | 15566 |
| Scores > Align Elements > Distribute Vertically |  | state-dependent | `self.project_open` | 15574 |
| Scores > Align Elements > Left |  | state-dependent | `self.project_open` | 15542 |
| Scores > Align Elements > Right |  | state-dependent | `self.project_open` | 15548 |
| Scores > Align Elements > Top |  | state-dependent | `self.project_open` | 15554 |
| Scores > Auto Layout... |  | available | `true` | 15630 |
| Scores > Build N-Tuplet... |  | state-dependent | `self.project_open && self.data.project.selected_midi_part_id.is_some()` | 15497 |
| Scores > Convert to Grace Note |  | state-dependent | `self.project_open` | 15529 |
| Scores > Flip |  | state-dependent | `self.project_open` | 15525 |
| Scores > Functions > Explode |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15615 |
| Scores > Functions > Explode Pitches |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15602 |
| Scores > Functions > Make Chord Symbols |  | unavailable | `false` | 15628 |
| Scores > Functions > Merge |  | state-dependent | `self.project_open && !self.is_recording() && self.data.project.selected_midi_part_id.is_some()` | 15589 |
| Scores > Group/Ungroup Notes |  | state-dependent | `self.project_open` | 15537 |
| Scores > Hide/Show |  | state-dependent | `self.project_open` | 15521 |
| Scores > Insert Slur |  | state-dependent | `self.project_open` | 15533 |
| Scores > Make Chord Symbols |  | state-dependent | `self.project_open` | 15583 |
| Scores > Make Guitar Symbols |  | unavailable | `false` | 15587 |
| Scores > Open Layout... |  | available | `true` | 15517 |
| Scores > Open Selection | Ctrl+R | state-dependent | `self.project_open && (!self.arrange_view_state.audio_events.selected_ids().is_empty() \|\| self.data.project.selected_midi_part_id.is_some())` | 15484 |
| Scores > Page Mode |  | available | `self.score_page_mode` | 15509 |
| Scores > Reset Layout... |  | available | `true` | 15634 |
| Scores > Settings... |  | available | `true` | 15513 |
| Transport > Locate Next Event | N | state-dependent | `self.project_open` | 15870 |
| Transport > Locate Next Marker | Shift+N | state-dependent | `self.project_open` | 15870 |
| Transport > Locate Previous Event | B | state-dependent | `self.project_open` | 15870 |
| Transport > Locate Previous Marker | Shift+B | state-dependent | `self.project_open` | 15870 |
| Transport > Locate Selection | L | state-dependent | `self.project_open` | 15870 |
| Transport > Locate Selection End |  | state-dependent | `self.project_open` | 15870 |
| Transport > Locators to Selection | P | state-dependent | `self.project_open` | 15870 |
| Transport > Loop Selection | Shift+G | state-dependent | `self.project_open` | 15870 |
| Transport > Metronome On | C | available | `true` | 16000 |
| Transport > Metronome Setup... |  | available | `true` | 15996 |
| Transport > Play Selection Range | Alt+Space | state-dependent | `self.project_open` | 15870 |
| Transport > Play from Selection End |  | state-dependent | `self.project_open` | 15881 |
| Transport > Play from Selection Start |  | state-dependent | `self.project_open` | 15875 |
| Transport > Play until Next Marker |  | state-dependent | `self.project_open` | 15897 |
| Transport > Play until Selection End |  | state-dependent | `self.project_open` | 15892 |
| Transport > Play until Selection Start |  | state-dependent | `self.project_open` | 15886 |
| Transport > Post-roll from Selection End |  | state-dependent | `can_roll` | 15907 |
| Transport > Post-roll from Selection Start |  | state-dependent | `can_roll` | 15903 |
| Transport > Pre-roll to Selection End |  | state-dependent | `can_roll` | 15915 |
| Transport > Pre-roll to Selection Start |  | state-dependent | `can_roll` | 15911 |
| Transport > Precount On |  | available | `true` | 16006 |
| Transport > Project Synchronization Setup... |  | available | `true` | 16037 |
| Transport > Retrospective Record | Shift+Num * | state-dependent | `self.project_open && !self.is_recording()` | 16026 |
| Transport > Roll Lengths... > Post-roll (beats, 0-64) |  | informational | `` | 15936 |
| Transport > Roll Lengths... > Pre-roll (beats, 0-64) |  | informational | `` | 15920 |
| Transport > Roll Lengths... > The selection is the selected audio events plus the selected MIDI part. |  | informational | `` | 15952 |
| Transport > Start Record at Left Locator |  | state-dependent | `self.project_open` | 15984 |
| Transport > Transport Panel | F2 | available | `true` | 15852 |
| Transport > Use External Sync | T | unavailable | `false` | 16041 |
| Transport > Use Post-roll |  | available | `self.use_post_roll` | 15972 |
| Transport > Use Pre-roll |  | available | `self.use_pre_roll` | 15960 |
| Window > Windows... |  | available | `true` | 16189 |
| Window > Workspaces > <lock_label> |  | available | `true` | 16184 |
| Window > Workspaces > <name> |  | available | `*active` | 16162 |
| Window > Workspaces > Organize Workspaces... |  | available | `true` | 16168 |

No duplicate full paths.

## Empty-project shell

18 leaves: 6 available, 0 state-dependent, 12 unavailable, 0 informational.

| Path | Shortcut | Status | Availability expression | Source line |
|---|---|---|---|---:|
| Audio > No project is open |  | unavailable | `false` | 16327 |
| Devices > Device Setup... |  | available | `true` | 16330 |
| Devices > VST Connections... | F4 | unavailable | `false` | 16334 |
| Edit > Key Commands... |  | available | `true` | 16318 |
| Edit > Redo |  | unavailable | `false` | 16316 |
| Edit > Undo |  | unavailable | `false` | 16315 |
| File > New Project... | Ctrl+N | available | `true` | 16297 |
| File > Open... | Ctrl+O | available | `true` | 16301 |
| File > Quit | Alt+F4 | available | `true` | 16309 |
| File > Save | Ctrl+S | unavailable | `false` | 16306 |
| File > Save As... | Ctrl+Shift+S | unavailable | `false` | 16307 |
| Help > About OpenTrack... |  | available | `true` | 16342 |
| MIDI > No project is open |  | unavailable | `false` | 16338 |
| Media > No project is open |  | unavailable | `false` | 16338 |
| Project > No project is open |  | unavailable | `false` | 16324 |
| Scores > No project is open |  | unavailable | `false` | 16338 |
| Transport > No project is open |  | unavailable | `false` | 16338 |
| Window > No project is open |  | unavailable | `false` | 16338 |

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

