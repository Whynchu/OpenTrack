# Keyboard command implementation audit (Windows reference keymap)

- Audited revision: `7b898c68f2272c5c7be9ef9d83f763072ae25424`
- Source: `docs/reference-keymap-windows.csv`; dispatch and key pre-handling: `crates/opentrack-app/src/app.rs` (`dispatch_reference_command`, `handle_reference_key`); ID/range generation: `crates/opentrack-core/src/commands.rs` (`KeymapProfile::windows_reference_default`).
- Method: derived each stable ID by lowercasing and replacing non-ASCII-alphanumeric runs with `_`, prefixed by normalized category; compared each ID to the dispatcher match arms, then inspected pre-dispatch key handling for documented shortcuts. A branch is not proof of full reference-application parity; implemented means a concrete app action is dispatched.
- The loader creates one command definition per CSV row and expands binding ranges into multiple chord bindings. The CSV contains 158 binding expressions; expansion produces 198 shortcut bindings. Counts distinguish rows/IDs, expressions, and expanded bindings.

## Summary

| CSV rows / stable command IDs | CSV binding expressions | Expanded shortcut bindings | Implemented | Partial | Fallback |
|---:|---:|---:|---:|---:|---:|
| 152 | 158 | 198 | 44 | 6 | 102 |

## Row-by-row mapping

| Page | Category | Reference command | Stable command ID | Windows binding(s) | Keymap/runtime context | Assessment | Dispatcher/pre-handler mapping |
|---:|---|---|---|---|---|---|---|
| 484 | Audio | Adjust Fades to Range | `audio.adjust_fades_to_range` | A | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Audio | Auto Grid | `audio.auto_grid` | Shift+Q | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Audio | Crossfade | `audio.crossfade` | X | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Audio | Find Selected in Pool | `audio.find_selected_in_pool` | Ctrl+F | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Automation | Toggle Read Enable All Tracks | `automation.toggle_read_enable_all_tracks` | Alt+R | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Automation | Toggle Write Enable All Tracks | `automation.toggle_write_enable_all_tracks` | Alt+W | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Devices | Mixer | `devices.mixer` | F3 | Global keymap; dispatcher requires project open | Implemented | focus Mixer tab |
| 484 | Devices | Virtual Keyboard | `devices.virtual_keyboard` | Alt+K | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Devices | Video | `devices.video` | F8 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Devices | VST Connections | `devices.vst_connections` | F4 | Global keymap; dispatcher requires project open | Partial | F4 is intercepted before dispatch to toggle VST Connections (dispatcher itself has no branch). |
| 484 | Devices | VST Instruments | `devices.vst_instruments` | F11 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Devices | VST Performance | `devices.vst_performance` | F12 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Autoscroll | `edit.autoscroll` | F | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Copy | `edit.copy` | Ctrl+C | Global keymap; dispatcher requires project open | Implemented | copy selected audio events |
| 484 | Edit | Cut | `edit.cut` | Ctrl+X | Global keymap; dispatcher requires project open | Implemented | cut selected audio events |
| 484 | Edit | Cut Time | `edit.cut_time` | Ctrl+Shift+X | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Delete | `edit.delete` | Delete; Backspace | Global keymap; dispatcher requires project open | Partial | Delete key is intercepted for project selection deletion (and MIDI-note deletion in editor); Backspace falls through to unimplemented dispatcher fallback. |
| 484 | Edit | Delete Time | `edit.delete_time` | Shift+Backspace | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Duplicate | `edit.duplicate` | Ctrl+D | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Edit In-place | `edit.edit_in_place` | Ctrl+Shift+I | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Group | `edit.group` | Ctrl+G | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Insert Silence | `edit.insert_silence` | Ctrl+Shift+E | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Left Selection Side to Cursor | `edit.left_selection_side_to_cursor` | E | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Lock | `edit.lock` | Ctrl+Shift+L | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Move to Cursor | `edit.move_to_cursor` | Ctrl+L | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Mute | `edit.mute` | M | Global keymap; dispatcher requires project open | Implemented | toggle selected track mute |
| 484 | Edit | Mute Events | `edit.mute_events` | Shift+M | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Mute/Unmute Objects | `edit.mute_unmute_objects` | Alt+M | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Open Default Editor | `edit.open_default_editor` | Ctrl+E | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Open Score Editor | `edit.open_score_editor` | Ctrl+R | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Open/Close Editor | `edit.open_close_editor` | Return | Editor binding; dispatcher requires project open | Partial | only sets MIDI editor closed; does not open/toggle it. |
| 484 | Edit | Paste | `edit.paste` | Ctrl+V | Global keymap; dispatcher requires project open | Implemented | paste audio events |
| 484 | Edit | Paste at Origin | `edit.paste_at_origin` | Alt+V | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Paste Time | `edit.paste_time` | Ctrl+Shift+V | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Record Enable | `edit.record_enable` | R | Global keymap; dispatcher requires project open | Implemented | toggle selected track arm |
| 484 | Edit | Redo | `edit.redo` | Ctrl+Shift+Z | Global keymap; dispatcher requires project open | Implemented | project redo |
| 484 | Edit | Repeat | `edit.repeat` | Ctrl+K | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Right Selection Side to Cursor | `edit.right_selection_side_to_cursor` | D | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Select All | `edit.select_all` | Ctrl+A | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Select None | `edit.select_none` | Ctrl+Shift+A | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Snap On/Off | `edit.snap_on_off` | J | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Solo | `edit.solo` | S | Global keymap; dispatcher requires project open | Implemented | toggle selected track solo |
| 484 | Edit | Split At Cursor | `edit.split_at_cursor` | Alt+X | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Split Range | `edit.split_range` | Shift+X | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Undo | `edit.undo` | Ctrl+Z | Global keymap; dispatcher requires project open | Implemented | project undo |
| 484 | Edit | Ungroup | `edit.ungroup` | Ctrl+U | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Unlock | `edit.unlock` | Ctrl+Shift+U | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 484 | Edit | Unmute Events | `edit.unmute_events` | Shift+U | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Editors | Show/Hide Info view | `editors.show_hide_info_view` | Ctrl+I | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Editors | Show/Hide Inspector | `editors.show_hide_inspector` | Alt+I | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Editors | Show/Hide Overview | `editors.show_hide_overview` | Alt+O | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | File | Close | `file.close` | Ctrl+W | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | File | New | `file.new` | Ctrl+N | Global; bypasses project-open gate | Implemented | direct project creation request |
| 485 | File | Open | `file.open` | Ctrl+O | Global; bypasses project-open gate | Implemented | direct project open request |
| 485 | File | Quit | `file.quit` | Ctrl+Q | Global; bypasses project-open gate | Implemented | quit/unsaved-changes flow |
| 485 | File | Save | `file.save` | Ctrl+S | Global keymap; dispatcher requires project open | Implemented | project save |
| 485 | File | Save As | `file.save_as` | Ctrl+Shift+S | Global keymap; dispatcher requires project open | Implemented | project Save As |
| 485 | File | Save New Version | `file.save_new_version` | Ctrl+Alt+S | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Media | Open Loop Browser | `media.open_loop_browser` | F6 | Global keymap; dispatcher requires project open | Implemented | focus Loop media category |
| 485 | Media | Open MediaBay | `media.open_mediabay` | F5 | Global keymap; dispatcher requires project open | Implemented | focus MediaBay category |
| 485 | Media | Open Sound Browser | `media.open_sound_browser` | F7 | Global keymap; dispatcher requires project open | Implemented | focus Sound media category |
| 485 | MIDI | Quantize | `midi.quantize` | Q | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Add Down | `navigate.add_down` | Shift+Down | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Add Left | `navigate.add_left` | Shift+Left | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Add Right | `navigate.add_right` | Shift+Right | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Add Up | `navigate.add_up` | Shift+Up | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Down | `navigate.down` | Down | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Left | `navigate.left` | Left | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Right | `navigate.right` | Right | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Up | `navigate.up` | Up | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Bottom | `navigate.bottom` | End | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Top | `navigate.top` | Home | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Navigate | Toggle Selection | `navigate.toggle_selection` | Ctrl+Space | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Nudge | End Left | `nudge.end_left` | Alt+Shift+Left | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Nudge | End Right | `nudge.end_right` | Alt+Shift+Right | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Nudge | Left | `nudge.left` | Ctrl+Left | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Nudge | Right | `nudge.right` | Ctrl+Right | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Nudge | Start Left | `nudge.start_left` | Alt+Left | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Nudge | Start Right | `nudge.start_right` | Alt+Right | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Project | Open Browser | `project.open_browser` | Ctrl+B | Global keymap; dispatcher requires project open | Partial | focuses Media tab/category 0; does not open a browser window. |
| 485 | Project | Open Markers | `project.open_markers` | Ctrl+M | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Project | Open/Close Pool | `project.open_close_pool` | Ctrl+P | Global keymap; dispatcher requires project open | Partial | Ctrl+P is intercepted before dispatch to focus Media tab; does not open/close a distinct Pool window. |
| 485 | Project | Open Tempo Track Editor | `project.open_tempo_track_editor` | Ctrl+T | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 485 | Project | Setup | `project.setup` | Shift+S | Global keymap; dispatcher requires project open | Partial | Shift+S is intercepted to open Project Setup (dispatcher itself has no branch). |
| 485 | Project | Show/Hide Track Colors | `project.show_hide_track_colors` | Shift+C | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Tool | Delete tool | `tool.delete_tool` | 5 | Global keymap; dispatcher requires project open | Implemented | delete tool state |
| 486 | Tool | Draw tool | `tool.draw_tool` | 8 | Global keymap; dispatcher requires project open | Implemented | draw tool state |
| 486 | Tool | Drumstick tool | `tool.drumstick_tool` | 0 | Global keymap; dispatcher requires project open | Implemented | drumstick tool state |
| 486 | Tool | Glue tool | `tool.glue_tool` | 4 | Global keymap; dispatcher requires project open | Implemented | glue tool state |
| 486 | Tool | Mute tool | `tool.mute_tool` | 7 | Global keymap; dispatcher requires project open | Implemented | mute tool state |
| 486 | Tool | Next Tool | `tool.next_tool` | F10 | Global keymap; dispatcher requires project open | Implemented | cycle tool forward |
| 486 | Tool | Play tool | `tool.play_tool` | 9 | Global keymap; dispatcher requires project open | Implemented | play tool state |
| 486 | Tool | Previous Tool | `tool.previous_tool` | F9 | Global keymap; dispatcher requires project open | Implemented | cycle tool backward |
| 486 | Tool | Range tool | `tool.range_tool` | 2 | Global keymap; dispatcher requires project open | Implemented | range tool state |
| 486 | Tool | Select tool | `tool.select_tool` | 1 | Global keymap; dispatcher requires project open | Implemented | select tool state |
| 486 | Tool | Split tool | `tool.split_tool` | 3 | Global keymap; dispatcher requires project open | Implemented | split tool state |
| 486 | Tool | Zoom tool | `tool.zoom_tool` | 6 | Global keymap; dispatcher requires project open | Implemented | zoom tool state |
| 486 | Transport | Auto Punch In | `transport.auto_punch_in` | I | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Auto Punch Out | `transport.auto_punch_out` | O | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Cycle | `transport.cycle` | NumpadSlash | Global keymap; dispatcher requires project open | Implemented | toggle cycle |
| 486 | Transport | Exchange time formats | `transport.exchange_time_formats` | Period | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Fast Forward | `transport.fast_forward` | Shift+NumpadAdd | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Fast Rewind | `transport.fast_rewind` | Shift+NumpadSubtract | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Forward | `transport.forward` | NumpadSubtract | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Input Left Locator | `transport.input_left_locator` | Shift+L | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Input Position | `transport.input_position` | Shift+P | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Input Right Locator | `transport.input_right_locator` | Shift+R | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Input Tempo | `transport.input_tempo` | Shift+T | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Insert Marker | `transport.insert_marker` | Insert | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Locate Next Event | `transport.locate_next_event` | N | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Locate Next Marker | `transport.locate_next_marker` | Shift+N | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Locate Previous Event | `transport.locate_previous_event` | B | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Locate Previous Marker | `transport.locate_previous_marker` | Shift+B | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Locate Selection | `transport.locate_selection` | L | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Locators to Selection | `transport.locators_to_selection` | P | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Loop Selection | `transport.loop_selection` | Shift+G | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Metronome On | `transport.metronome_on` | C | Global keymap; dispatcher requires project open | Implemented | toggle metronome |
| 486 | Transport | Nudge Cursor right | `transport.nudge_cursor_right` | Ctrl+NumpadSubtract | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Nudge Cursor left | `transport.nudge_cursor_left` | Ctrl+NumpadAdd | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Panel (Transport panel) | `transport.panel_transport_panel` | F2 | Global keymap; dispatcher requires project open | Implemented | toggle Transport window |
| 486 | Transport | Play Selection Range | `transport.play_selection_range` | Alt+Space | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Recall Cycle Marker 1 to 9 | `transport.recall_cycle_marker_1_to_9` | Shift+Numpad1..9 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Record | `transport.record` | NumpadMultiply | Global keymap; dispatcher requires project open | Implemented | toggle recording |
| 486 | Transport | Retrospective Record | `transport.retrospective_record` | Shift+NumpadMultiply | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Return to Zero | `transport.return_to_zero` | NumpadDecimal; NumpadComma | Global keymap; dispatcher requires project open | Implemented | seek transport to zero |
| 486 | Transport | Rewind | `transport.rewind` | NumpadSubtract | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Set Left Locator | `transport.set_left_locator` | Ctrl+Numpad1 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Set Marker 1 | `transport.set_marker_1` | Ctrl+1 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Set Marker 2 | `transport.set_marker_2` | Ctrl+2 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Set Marker 3 to 9 | `transport.set_marker_3_to_9` | Ctrl+Numpad3..9; Ctrl+3..9 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Set Right Locator | `transport.set_right_locator` | Ctrl+Numpad2 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Play | `transport.play` | Enter | Global keymap; dispatcher requires project open | Implemented | start playback (also finishes recording) |
| 486 | Transport | Start/Stop | `transport.start_stop` | Space | Global keymap; dispatcher requires project open | Implemented | toggle playback |
| 486 | Transport | Stop | `transport.stop` | Numpad0 | Global keymap; dispatcher requires project open | Implemented | stop playback |
| 486 | Transport | To Left Locator | `transport.to_left_locator` | Numpad1 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | To Marker 1 | `transport.to_marker_1` | Shift+1 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | To Marker 2 | `transport.to_marker_2` | Shift+2 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | To Marker 3 to 9 | `transport.to_marker_3_to_9` | Numpad3..9; Shift+3..9 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | To Right Locator | `transport.to_right_locator` | Numpad2 | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Transport | Use External Sync | `transport.use_external_sync` | T | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Workspace | Lock/Unlock Active Workspace | `workspace.lock_unlock_active_workspace` | Alt+Numpad0 | Global keymap; dispatcher requires project open | Implemented | toggle workspace lock |
| 486 | Workspace | New | `workspace.new` | Ctrl+Numpad0 | Global keymap; dispatcher requires project open | Implemented | create workspace |
| 486 | Workspace | Organize | `workspace.organize` | W | Global keymap; dispatcher requires project open | Implemented | open workspace organizer |
| 486 | Workspace | Workspace 1 to 9 | `workspace.workspace_1_to_9` | Alt+Numpad1..9 | Global keymap; dispatcher requires project open | Implemented | activate workspace by numpad digit |
| 486 | Zoom | Zoom Full | `zoom.zoom_full` | Shift+F | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Zoom | Zoom In | `zoom.zoom_in` | H | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Zoom | Zoom In Tracks | `zoom.zoom_in_tracks` | Alt+Down | Global keymap; dispatcher requires project open | Implemented | vertical track zoom in |
| 486 | Zoom | Zoom Out | `zoom.zoom_out` | G | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Zoom | Zoom Out Tracks | `zoom.zoom_out_tracks` | Alt+Up; Ctrl+Up | Global keymap; dispatcher requires project open | Implemented | vertical track zoom out |
| 486 | Zoom | Zoom to Event | `zoom.zoom_to_event` | Shift+E | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Zoom | Zoom to Selection | `zoom.zoom_to_selection` | Alt+S | Global keymap; dispatcher requires project open | Fallback | No dispatcher branch or identified pre-dispatch action; resolves to `Not implemented yet: <label>`. |
| 486 | Zoom | Zoom Tracks Exclusive | `zoom.zoom_tracks_exclusive` | Z; Ctrl+Down | Global keymap; dispatcher requires project open | Implemented | toggle selected-track exclusive zoom |

## Dispatch constraints and unknowns

- Before lookup, shortcut processing is suppressed while a key assignment is pending or modal media/rate-change error state is active. Most reference commands are also gated on `project_open`; only `file.new`, `file.open`, and `file.quit` bypass that gate. The table classifies command actions assuming normal project state unless its row calls out a pre-handler.
- The loaded command catalog and binding scopes do not establish support for manual application contexts. `edit.open_close_editor` is explicitly scoped to `editor`; other bindings are global. Runtime lookup supplies `editor` plus `arrange` while the MIDI editor is open, otherwise `arrange`.
- Unknown / not audited here: whether the source manual?s operations match a particular app build; interactive GUI outcome of each shortcut; all downstream behavior of called handlers; whether menu actions provide equivalent behavior. Unhandled commands explicitly hit the dispatcher fallback, while separate ordinary UI key behavior outside this reference-key path is not treated as implementation.
- Binding counts are expanded according to the loader?s digit-range expansion. ?Generated IDs? refers to the loader?s one normalized stable ID per CSV row; digit expansions generate bindings, not separate catalog IDs.
