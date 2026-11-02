# OpenTrack roadmap revision 3: menus, keyboard and the remaining depth

Status: active, 2026-11-02. Revision 2 closed with 29 of 32 tasks delivered and
three blocked on material this host does not have (CPR fixtures, a runnable Linux
VST3 plug-in). This revision works the two gaps that are still implementable
here: declared-but-disabled menu entries and keymap-bound commands with no
dispatch arm.

Starting measurements (2026-11-02, after revision 2):

- declared-but-disabled menu entries: **149**
- keymap-bound commands with a working shortcut: **81 of 152** (71 without)
- app suite 393 passed, core 114, audio 152, scanner 29 + 4, plugins 18,
  vst3 14, win-jobs 1

Rules stay as in revision 2: one slice per commit with source, tests, docs,
release build and push; a feature is not accepted because it compiles; blocked
work is named rather than approximated. Every slice also refreshes `README.md`'s status sections
and measured numbers.

## Sequence

### A. Transport and timeline navigation — delivered

1. **Transport locate family.** *Delivered.* Locate Next/Previous Event (N/B),
   Locate Selection (L), Locate Selection End, Locators to Selection (P), Loop
   Selection (Shift+G), Play Selection Range (Alt+Space), plus Track Folding >
   Expand All / Collapse All and a checked Precount On. Twelve disabled entries
   became working commands; six dead shortcuts became live. 2 core and 5 app
   tests. Disabled entries 149 -> 137; shortcut coverage 75 -> 81.

2. **Keyboard coverage: navigation, transport and view.** *Delivered.* Track
   selection Up/Down/Top/Bottom and Add Up/Down; Set Left/Right Locator at the
   cursor; cursor Forward/Rewind/Fast/Nudge and To Left/Right Locator; Auto Punch
   In/Out; ruler format exchange; Split Range; Zoom Full and Zoom to Event; the
   Info view, Inspector and track-colour toggles; and automation read/write for
   every track, each as one labelled undo step where it edits the project. Shortcut
   coverage rises 90 -> 109 of 152 commands. A follow-up in the same slice added
   the selection-side keys (E/D move the locator to the cursor), Project Setup
   (Shift+S) and Toggle Selection (Ctrl+Space), taking coverage to 114. 6 app
   tests.

### B. Audio event processing

3. **Fade In, Fade Out and Silence.** *Delivered.* Audio > Process gains working
   Fade In, Fade Out and Silence. `Project::fade_audio_clips` clamps the new fade
   to the event and to whatever room the opposite fade leaves (the sum can never
   exceed the event), and `silence_audio_clips` zeroes the event gain while keeping
   the media in the pool. Both skip locked events and run as one labelled undo
   step. 2 core and 1 app tests. Disabled entries 137 -> 134.

### C. Editing and panels

5. **Paste at Origin, Quantize Setup, VST Instruments, VST Performance.**
   *Delivered.* Edit > Paste at Origin (Alt+V) pastes the clipboard at the beats
   it was copied from instead of the cursor. MIDI > Quantize Setup opens the
   quantize dialog with the toolbar grid. Devices > VST Instruments (F11) opens
   the instrument-track dialog with the catalog; VST Performance (F12) opens the
   realtime diagnostics panel. Disabled entries 130 -> 126; shortcut coverage
   114 -> 117. 1 app test.

### D. Project tools

6. **Project Statistics and Beat Calculator.** *Delivered.* New `project_tools`
   module with a pure `project_statistics` (track kinds, audio events, MIDI parts,
   notes and controllers, markers, pool entries, missing media, automation points,
   inserts, sends) and a beat calculator that converts beats and seconds at the
   project tempo, with "From Locators" prefilling the loop length. Audio >
   Statistics opens the counts; Project > Beat Calculator opens the converter;
   Project > Browser (Ctrl+B) now opens the Media tab from its own menu entry,
   matching the shortcut that already worked. 3 module and 2 app tests. Disabled
   entries 126 -> 123.

### E. Event editing

8. **Nudge Start/End Left/Right.** *Delivered.* Alt+Left/Right move the selected
   events' start and Alt+Shift+Left/Right move their end by one grid step. Core
   `nudged_clip_edge` is pure: a start move carries the source offset so the audio
   does not slide and stops at the source's first sample, an end move stops at the
   source's last sample, both keep a minimum length, fades are clamped, and a
   locked event is refused. The app resolves each clip's source length from the
   open audio sources and falls back to the known extent when a source is not
   loaded. One labelled undo step per command ("Nudge Event Start"/"Nudge Event
   End"). 3 core and 1 app tests. Shortcut coverage 117 -> 121. README numbers
   refreshed with the slice.

9. **Insert Silence.** *Delivered.* Edit > Insert Silence (Ctrl+Shift+E) inserts
   one bar at the cursor across the project. Core `insert_silence_at` splits every
   event and MIDI part crossing the point, shifts the right-hand material,
   markers and automation points, and refuses the whole edit when a locked event
   crosses it. One labelled undo step ("Insert Silence"). 2 core and 1 app tests.
   Shortcut coverage 121 -> 122.

10. **Cut Time and Delete Time.** *Delivered.* Core `remove_time_range` removes
   `[start, end)`, splits events and MIDI parts at both boundaries, closes the gap,
   shifts later clips, parts, automation and (for whole-project edits) markers,
   and refuses when a locked event overlaps the range. Ctrl+Shift+X (Cut Time)
   edits every track; Shift+Backspace (Delete Time) edits only the selected
   tracks and leaves markers alone. One labelled undo step each. 2 core and 1 app
   tests. Shortcut coverage 122 -> 124.

11. **Typed position, locators and tempo.** *Delivered.* Input Position (Shift+P),
   Input Left Locator (Shift+L), Input Right Locator (Shift+R) and Input Tempo
   (Shift+T) open one small dialog seeded from the current value; Apply sets the
   transport, the locator pair (one labelled "Set Locators" undo step) or the
   project tempo (one labelled "Set Tempo" step). Negative, non-finite and
   out-of-range values are refused without editing. 2 app tests. Shortcut
   coverage 124 -> 128.

12. **Paste Time.** *Delivered.* Ctrl+Shift+V computes the clipboard's span,
   ripple-inserts that span at the cursor through `Project::insert_silence_at`,
   then pastes the clipboard there as one labelled undo step ("Paste Time"). It
   refuses when a locked event crosses the cursor and leaves the project
   unchanged. The paste destination logic is now a shared `build_paste` used by
   the ordinary, at-origin and time pastes. 2 app tests. Shortcut coverage
   128 -> 129.

13. **Close Project.** *Delivered.* File > Close (Ctrl+W) returns to the
   no-project shell. An unmodified project closes immediately; unsaved changes
   open a Save and Close / Discard / Cancel prompt. Save writes the project file
   and only closes when the write succeeded; Discard closes without writing.
   `reset_project` and Close now share one `clear_project_state` path, so a closed
   project leaves no clipboard, transport, window or workspace state behind. 4 app
   tests. Shortcut coverage 129 -> 130.

14. **Plain-arrow navigation.** *Delivered.* Navigate Left/Right move the cursor
   by one grid step, but only while no widget holds keyboard focus, so a focused
   control keeps its own arrow handling. Navigate Add Left/Right extend the
   locator range by a step as one labelled "Set Locators" undo step, stopping at
   the project start. 1 app test. Shortcut coverage 130 -> 134.

15. **Find Selected in Pool.** *Delivered.* Ctrl+F reveals the selected audio
   event's source in the Media window: the pool marks the row with "▶ " in the
   accent colour and scrolls it into view. A MIDI-part selection and an empty
   selection report instead. The reveal path is transient and cleared when a
   project closes. 3 app and 1 module tests. Shortcut coverage 134 -> 135.

16. **Adjust Fades to Range.** *Delivered.*
   `Project::fit_audio_clip_fades_to_range` makes a selected event's fade-in cover
   what precedes the left locator and its fade-out what follows the right one, so
   the material inside the range stays at full level; events fully inside lose
   their fades, events outside and locked events are untouched. Audio > Adjust
   Fades to Range (A) is one labelled undo step. 1 core and 1 app tests. Shortcut
   coverage 135 -> 136.

17. **Marker slots.** *Delivered.* `Project::Marker` gained an optional `slot`
   (1-9); `set_marker_slot` creates or moves the marker in a slot without changing
   its identity, `marker_in_slot` addresses it, and validation rejects duplicate or
   out-of-range slots. Set Marker 1/2/3-9 (Ctrl+digit), To Marker 1/2/3-9
   (Shift+digit) and Recall Cycle Marker 1-9 (Shift+Numpad) decode the digit from
   the chord. OPR moved to version 29 so older builds refuse a file with slots
   instead of dropping them; a v28 file loads with no slots. 1 core, 2 app and 1
   migration tests. Shortcut coverage 136 -> 143.

### E. Project tools (continued)

7. **Notepad.** *Delivered.* `Project::notepad` is saved with the project, bounded
   to 100,000 characters, left out of the file when empty, and read as empty by
   older projects. Project > Notepad opens a multiline editor; closing it applies
   the text as one undo step labelled "Notepad" when it changed. 1 core and 2 app
   tests. Disabled entries 123 -> 122.

### F. Media pool

4. **Pool cleanup commands.** *Delivered.* Media > Remove Unused Media, Empty
   Trash and Remove Missing Files. Core gained `referenced_media`,
   `unused_media`, `remove_unused_media`, `missing_unreferenced_media` and
   `remove_missing_unreferenced_media`; a missing file an event still references
   is deliberately kept so relink can resolve it. Remove Unused Media drops the
   entries and never touches the files. Empty Trash drops every unused entry and
   deletes the unused files that live inside the project folder through the
   existing recoverable-deletion path, leaving files outside the project on disk
   because they may be shared; Undo restores both the files and the entries. 2
   core and 3 app tests. Disabled entries 133 -> 130.
5. **Pool import/export.** Import Pool... and Export Pool... as a manifest of the
   project's media rather than a new format.

### C. Audio event processing

5. **Fade In / Fade Out.** Apply the default fade length to the selected events'
   leading or trailing edge as one undo step, honouring locks.
6. **Normalize and Reverse through the offline renderer.** Render the selection
   like bounce, then replace it, so the commands reuse one verified path.

### E. Track and project

7. **New Version.** *Delivered.* File > New Version (Ctrl+Alt+S) writes a numbered
   copy beside the open project (`Song (2).opr`, then `Song (3).opr`) through the
   ordinary save path and keeps working in the copy, marked saved. The version
   chain counts from the base name, so "Song (2)" continues at "Song (3)". 1 app
   test. Disabled entries 134 -> 133; shortcut coverage 109 -> 110.

8. **Create Folder from selected tracks.** Group the selected tracks under a new
   group track with correct routing, as one undo step.
9. **Browser and panel commands.** Open Browser (Ctrl+B), VST Instruments (F11),
   VST Performance (F12) and the transport panel (F2) reach their existing panels
   from the reference keys.

### F. MIDI

10. **Quantize Setup and Insert Velocities.** Both dialogs wire to existing MIDI
   commands; the setup dialog chooses grid, strength and swing defaults.
11. **Tuplets.** Build N-Tuplet as one bounded MIDI note transform with tests.

### G. Consolidation

12. **Close revision 3.** Re-measure the menu inventory, the shortcut coverage and
    the suites; choose revision 4.
