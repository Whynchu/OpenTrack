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
work is named rather than approximated.

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
   coverage rises 90 -> 109 of 152 commands. 4 app tests.

### B. Audio event processing

3. **Fade In, Fade Out and Silence.** *Delivered.* Audio > Process gains working
   Fade In, Fade Out and Silence. `Project::fade_audio_clips` clamps the new fade
   to the event and to whatever room the opposite fade leaves (the sum can never
   exceed the event), and `silence_audio_clips` zeroes the event gain while keeping
   the media in the pool. Both skip locked events and run as one labelled undo
   step. 2 core and 1 app tests. Disabled entries 137 -> 134.

### C. Media pool

4. **Pool cleanup commands.** Remove Unused Media, Empty Trash and Select in
   Project over the existing pool model, each one undo step with a report of what
   moved. Missing-file commands reuse the revision-2 relink/remove logic.
5. **Pool import/export.** Import Pool... and Export Pool... as a manifest of the
   project's media rather than a new format.

### C. Audio event processing

5. **Fade In / Fade Out.** Apply the default fade length to the selected events'
   leading or trailing edge as one undo step, honouring locks.
6. **Normalize and Reverse through the offline renderer.** Render the selection
   like bounce, then replace it, so the commands reuse one verified path.

### E. Track and project

7. **Create Folder from selected tracks.** Group the selected tracks under a new
   group track with correct routing, as one undo step.
8. **New Version.** Save a numbered copy of the project beside the current file
   and keep working in it (Ctrl+Alt+S).
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
