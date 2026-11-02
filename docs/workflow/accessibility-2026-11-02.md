# Accessibility and keyboard coverage (2026-11-02)

Scope: revision-2 task 31 and the revision-3 menu/keyboard slices. This records
what was measured in the application's own code and theme, not a claim about
assistive-technology conformance.

## What changed

- Disabled and secondary text meets the 3:1 WCAG contrast ratio for non-text
  UI in both themes (`theme::readable_muted`). The dark theme measured
  2.27:1 before the change.
- Reference shortcuts that did nothing now reach real commands, in two slices:
  selection/toolbar commands (Select All, Select None, Delete, Snap, Autoscroll,
  Quantize, VST Connections), locate and loop commands (Locate Next/Previous
  Event, Locate Selection, Locators to Selection, Loop Selection, Play Selection
  Range), track navigation (Up/Down/Top/Bottom, Add Up/Down), locator and cursor
  movement (Set Left/Right Locator, Forward, Rewind, Fast Forward/Rewind, Nudge
  Cursor), auto punch in/out, ruler format exchange, Split Range, zoom full/
  to event, the Info view / Inspector / track-colour toggles, automation
  read/write for every track, Paste at Origin, the VST Instruments / VST
  Performance panels, the four Nudge Start/End Left/Right edge trims, Insert Silence, Cut Time and Delete Time.
- Twelve Transport and Track Folding menu entries moved from disabled to
  working in the first slice; the later slices were keyboard-only and did not
  change the menu inventory.
- A test audits the bundled keymap against the dispatch table on every run and
  asserts a growing set of essential commands stays handled. A second test
  asserts contrast ratios in both themes.

## Coverage

The bundled Windows reference keymap declares **152** distinct commands
(**158** literal chord entries; the runtime profile expands them to 198 for
numpad ranges). **124** commands have a dispatch
arm; **28** still do not. The menus remain the way to reach the
uncovered features.

## Bound commands without a dispatch arm

### Audio

- Adjust Fades to Range (`A`) — `audio.adjust_fades_to_range`
- Auto Grid (`Shift+Q`) — `audio.auto_grid`
- Find Selected in Pool (`Ctrl+F`) — `audio.find_selected_in_pool`

### Devices

- Video (`F8`) — `devices.video`
- Virtual Keyboard (`Alt+K`) — `devices.virtual_keyboard`

### Edit

- Edit In-place (`Ctrl+Shift+I`) — `edit.edit_in_place`
- Open Score Editor (`Ctrl+R`) — `edit.open_score_editor`
- Paste Time (`Ctrl+Shift+V`) — `edit.paste_time`

### Editors

- Show/Hide Overview (`Alt+O`) — `editors.show_hide_overview`

### File

- Close (`Ctrl+W`) — `file.close`

### Navigate

- Add Left (`Shift+Left`) — `navigate.add_left`
- Add Right (`Shift+Right`) — `navigate.add_right`
- Left (`Left`) — `navigate.left`
- Right (`Right`) — `navigate.right`

### Project

- Open Tempo Track Editor (`Ctrl+T`) — `project.open_tempo_track_editor`

### Transport

- Input Left Locator (`Shift+L`) — `transport.input_left_locator`
- Input Position (`Shift+P`) — `transport.input_position`
- Input Right Locator (`Shift+R`) — `transport.input_right_locator`
- Input Tempo (`Shift+T`) — `transport.input_tempo`
- Recall Cycle Marker 1 to 9 (`Shift+Numpad1..9`) — `transport.recall_cycle_marker_1_to_9`
- Retrospective Record (`Shift+NumpadMultiply`) — `transport.retrospective_record`
- Set Marker 1 (`Ctrl+1`) — `transport.set_marker_1`
- Set Marker 2 (`Ctrl+2`) — `transport.set_marker_2`
- Set Marker 3 to 9 (`Ctrl+Numpad3..9; Ctrl+3..9`) — `transport.set_marker_3_to_9`
- To Marker 1 (`Shift+1`) — `transport.to_marker_1`
- To Marker 2 (`Shift+2`) — `transport.to_marker_2`
- To Marker 3 to 9 (`Numpad3..9; Shift+3..9`) — `transport.to_marker_3_to_9`
- Use External Sync (`T`) — `transport.use_external_sync`
## Focus and labels

- egui moves keyboard focus with Tab between interactive widgets in layout
  order, and menu buttons are ordinary widgets, so the menu bar is reachable
  without a mouse.
- Shortcut dispatch is skipped while a text field has keyboard focus, so typing
  in a name field does not trigger editor commands.
- Icons and colour-only controls carry hover text that names the command and,
  where it cycles, the next state.
- Every dialog added in revision 2 has a title, a labelled primary action and a
  Cancel; destructive actions name what they remove.

## Known gaps

- The 43 shortcuts listed above are declared by the reference keymap
  but not dispatched. Most need an editor or feature that does not exist yet
  (score editor, tempo track editor, time-stretch dialogs, marker slots).
- There is no screen-reader narration beyond egui's own widget metadata, and no
  high-contrast theme selection yet.
- Keyboard-only use is audited by source and tests, not by a manual pass in the
  running application; that review remains.

