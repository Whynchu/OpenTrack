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
  Performance panels, the four Nudge Start/End Left/Right edge trims, Insert Silence, Cut Time and Delete Time, typed Position / Locator / Tempo entry, Paste Time, Close Project, and plain-arrow cursor navigation with additive range keys, Find Selected in Pool, Adjust Fades to Range, the numeric marker slots (Set/To Marker 1-9 and Recall Cycle Marker), Auto Grid, Edit In-place, the project overview, and the virtual keyboard.
- Twelve Transport and Track Folding menu entries moved from disabled to
  working in the first slice; the later slices were keyboard-only and did not
  change the menu inventory.
- A test audits the bundled keymap against the dispatch table on every run and
  asserts a growing set of essential commands stays handled. A second test
  asserts contrast ratios in both themes.

## Coverage

The bundled Windows reference keymap declares **152** distinct commands
(**158** literal chord entries; the runtime profile expands them to 198 for
numpad ranges). **147** commands have a dispatch
arm; **5** still do not. The menus remain the way to reach the
uncovered features.

## Bound commands without a dispatch arm

### Devices

- Video (`F8`) — `devices.video`

### Edit

- Open Score Editor (`Ctrl+R`) — `edit.open_score_editor`

### Project

- Open Tempo Track Editor (`Ctrl+T`) — `project.open_tempo_track_editor`

### Transport

- Retrospective Record (`Shift+NumpadMultiply`) — `transport.retrospective_record`
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

