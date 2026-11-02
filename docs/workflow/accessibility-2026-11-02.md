# Accessibility and keyboard coverage (2026-11-02)

Scope: revision-2 task 31. This records what was measured in the running
application's own code and theme, not a claim about assistive-technology
conformance.

## What changed

- Disabled and secondary text now meets the 3:1 WCAG contrast ratio for
  non-text UI in both themes. The theme's weak colour is kept and moved
  only as far toward the body text colour as needed
  (`theme::readable_muted`). Before this change the dark theme measured
  2.27:1.

- Seven reference shortcuts that previously did nothing now reach real
  commands: Select All (Ctrl+A), Select None (Ctrl+Shift+A), Delete
  (Delete/Backspace), Snap On/Off (J), Autoscroll (F), Quantize (Q) and
  VST Connections (F4).

- A test audits the bundled keymap against the dispatch table on every
  run, asserts twenty essential commands stay handled, and prints the
  coverage counts used below. A second test asserts the contrast ratios
  for both themes.

## Coverage

The bundled Windows reference keymap binds **152** distinct commands across **198**
chords. **75** commands have a dispatch arm; **77** still do not, so their chord
does nothing yet. Counted as chords, 121 of 198 reach an arm. The menus remain
the way to reach the uncovered features.

## Bound commands without a dispatch arm

### Audio

- Adjust Fades to Range (`A`) — `audio.adjust_fades_to_range`
- Auto Grid (`Shift+Q`) — `audio.auto_grid`
- Find Selected in Pool (`Ctrl+F`) — `audio.find_selected_in_pool`

### Automation

- Toggle Read Enable All Tracks (`Alt+R`) — `automation.toggle_read_enable_all_tracks`
- Toggle Write Enable All Tracks (`Alt+W`) — `automation.toggle_write_enable_all_tracks`

### Devices

- Video (`F8`) — `devices.video`
- Virtual Keyboard (`Alt+K`) — `devices.virtual_keyboard`
- VST Instruments (`F11`) — `devices.vst_instruments`
- VST Performance (`F12`) — `devices.vst_performance`

### Edit

- Cut Time (`Ctrl+Shift+X`) — `edit.cut_time`
- Delete Time (`Shift+Backspace`) — `edit.delete_time`
- Edit In-place (`Ctrl+Shift+I`) — `edit.edit_in_place`
- Insert Silence (`Ctrl+Shift+E`) — `edit.insert_silence`
- Left Selection Side to Cursor (`E`) — `edit.left_selection_side_to_cursor`
- Open Score Editor (`Ctrl+R`) — `edit.open_score_editor`
- Paste at Origin (`Alt+V`) — `edit.paste_at_origin`
- Paste Time (`Ctrl+Shift+V`) — `edit.paste_time`
- Right Selection Side to Cursor (`D`) — `edit.right_selection_side_to_cursor`
- Split Range (`Shift+X`) — `edit.split_range`

### Editors

- Show/Hide Info view (`Ctrl+I`) — `editors.show_hide_info_view`
- Show/Hide Inspector (`Alt+I`) — `editors.show_hide_inspector`
- Show/Hide Overview (`Alt+O`) — `editors.show_hide_overview`

### File

- Close (`Ctrl+W`) — `file.close`
- Save New Version (`Ctrl+Alt+S`) — `file.save_new_version`

### Navigate

- Add Down (`Shift+Down`) — `navigate.add_down`
- Add Left (`Shift+Left`) — `navigate.add_left`
- Add Right (`Shift+Right`) — `navigate.add_right`
- Add Up (`Shift+Up`) — `navigate.add_up`
- Bottom (`End`) — `navigate.bottom`
- Down (`Down`) — `navigate.down`
- Left (`Left`) — `navigate.left`
- Right (`Right`) — `navigate.right`
- Toggle Selection (`Ctrl+Space`) — `navigate.toggle_selection`
- Top (`Home`) — `navigate.top`
- Up (`Up`) — `navigate.up`

### Nudge

- End Left (`Alt+Shift+Left`) — `nudge.end_left`
- End Right (`Alt+Shift+Right`) — `nudge.end_right`
- Start Left (`Alt+Left`) — `nudge.start_left`
- Start Right (`Alt+Right`) — `nudge.start_right`

### Project

- Open Browser (`Ctrl+B`) — `project.open_browser`
- Open Tempo Track Editor (`Ctrl+T`) — `project.open_tempo_track_editor`
- Setup (`Shift+S`) — `project.setup`
- Show/Hide Track Colors (`Shift+C`) — `project.show_hide_track_colors`

### Transport

- Auto Punch In (`I`) — `transport.auto_punch_in`
- Auto Punch Out (`O`) — `transport.auto_punch_out`
- Exchange time formats (`Period`) — `transport.exchange_time_formats`
- Fast Forward (`Shift+NumpadAdd`) — `transport.fast_forward`
- Fast Rewind (`Shift+NumpadSubtract`) — `transport.fast_rewind`
- Forward (`NumpadSubtract`) — `transport.forward`
- Input Left Locator (`Shift+L`) — `transport.input_left_locator`
- Input Position (`Shift+P`) — `transport.input_position`
- Input Right Locator (`Shift+R`) — `transport.input_right_locator`
- Input Tempo (`Shift+T`) — `transport.input_tempo`
- Locate Next Event (`N`) — `transport.locate_next_event`
- Locate Previous Event (`B`) — `transport.locate_previous_event`
- Locate Selection (`L`) — `transport.locate_selection`
- Locators to Selection (`P`) — `transport.locators_to_selection`
- Loop Selection (`Shift+G`) — `transport.loop_selection`
- Nudge Cursor left (`Ctrl+NumpadAdd`) — `transport.nudge_cursor_left`
- Nudge Cursor right (`Ctrl+NumpadSubtract`) — `transport.nudge_cursor_right`
- Play Selection Range (`Alt+Space`) — `transport.play_selection_range`
- Recall Cycle Marker 1 to 9 (`Shift+Numpad1..9`) — `transport.recall_cycle_marker_1_to_9`
- Retrospective Record (`Shift+NumpadMultiply`) — `transport.retrospective_record`
- Rewind (`NumpadSubtract`) — `transport.rewind`
- Set Left Locator (`Ctrl+Numpad1`) — `transport.set_left_locator`
- Set Marker 1 (`Ctrl+1`) — `transport.set_marker_1`
- Set Marker 2 (`Ctrl+2`) — `transport.set_marker_2`
- Set Marker 3 to 9 (`Ctrl+Numpad3..9; Ctrl+3..9`) — `transport.set_marker_3_to_9`
- Set Right Locator (`Ctrl+Numpad2`) — `transport.set_right_locator`
- To Left Locator (`Numpad1`) — `transport.to_left_locator`
- To Marker 1 (`Shift+1`) — `transport.to_marker_1`
- To Marker 2 (`Shift+2`) — `transport.to_marker_2`
- To Marker 3 to 9 (`Numpad3..9; Shift+3..9`) — `transport.to_marker_3_to_9`
- To Right Locator (`Numpad2`) — `transport.to_right_locator`
- Use External Sync (`T`) — `transport.use_external_sync`

### Zoom

- Zoom Full (`Shift+F`) — `zoom.zoom_full`
- Zoom to Event (`Shift+E`) — `zoom.zoom_to_event`

## Focus and labels

- egui moves keyboard focus with Tab between interactive widgets in layout
  order, and menu buttons are ordinary widgets, so the menu bar is reachable
  without a mouse.
- Icons and colour-only controls carry hover text that names the command
  (for example the MON control names Off/Auto/On and the next state).
- Every dialog added in revision 2 has a title, a labelled primary action
  and a Cancel; destructive actions name what they remove.

## Known gaps

- The 77 shortcuts listed above are declared by the reference keymap but not
  dispatched. They are the next accessibility work, not a finished claim.

- There is no screen-reader narration beyond egui's own widget metadata, and
  no high-contrast theme selection yet.

- Keyboard-only use was audited by source and tests, not by a manual pass in
  the running application; that review remains.

