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

18. **Auto Grid.** *Delivered.* `ArrangeViewState.auto_grid` makes snapping use
   the ruler's own zoom-derived subdivision (bar, quarter, eighth, sixteenth)
   instead of the chosen grid, so the snap matches the lines on screen; turning it
   off restores the selected grid, and snap-off still disables snapping. Shift+Q
   toggles it, the grid popup carries the checkbox, and the status names the
   resulting step. The flag persists with view state and defaults off for older
   projects. 1 view-state and 1 app tests. Shortcut coverage 143 -> 144.

19. **Edit In-place.** *Delivered.* Ctrl+Shift+I opens the selection in the editor
   that fits it: an audio event opens the Sample Editor in the lower zone (the
   in-place media surface), a MIDI part opens the MIDI editor. An empty selection
   reports instead of guessing. 1 app test. Shortcut coverage 144 -> 145.

20. **Project overview.** *Delivered.* Alt+O and Window > Project Overview show a
   compact whole-project strip: one row per track with audio events in grey and
   MIDI parts in blue, the visible arrange window as a translucent band, and the
   playhead. Clicking or dragging inside seeks. The flag persists with view state
   and defaults off. 2 app tests (toggle/persistence, render plus click-seek).
   Shortcut coverage 145 -> 146.

21. **Gain dialog and the editor entries.** *Delivered.* Audio > Process > Gain...
   opens a dB dialog over the selected events; `Project::set_audio_clip_gain_db`
   clamps to 0..4.0 linear, treats -60 dB and below as silence, and skips locked
   events. One labelled "Event Gain" undo step. MIDI > Open In-Place Editor and
   Scores > Open Selection now open the same editor as Ctrl+Shift+I (Sample
   Editor for audio, MIDI editor for parts). 1 core and 1 app tests. Disabled
   entries 122 -> 119.

22. **Auto Fades Settings and Play from Selection.** *Delivered.* Project > Auto
   Fades Settings... sets the default fade length used by Fade In/Out (0-10 s,
   persisted as an application preference; existing fades keep their length).
   Transport > Play from Selection Start/End seek to the selection edge and start
   playback, reporting when nothing is selected. 2 app tests. Disabled entries
   119 -> 116.

23. **Play until Selection Start/End and Play until Next Marker.**
   *Delivered.* Transport > Play until... starts playback with a transient stop
   target; `advance_play_until` stops the transport when the playhead reaches it,
   and any manual stop clears the target. Targets behind the cursor, a missing
   selection and a missing marker report instead of starting. 3 app tests.
   Disabled entries 116 -> 113.

24. **Menu aliases for existing commands.** *Delivered.* Media > Insert into
   Project now opens the Add Track dialog for audio, instrument and MIDI tracks;
   Media > Find Missing Files... reuses the relink folder picker (and reports when
   nothing is missing); MIDI > Functions > Merge reuses the MIDI part merge. No
   new behavior was duplicated — each entry calls the command that already
   exists. 1 app test for the relink refusal. Disabled entries 113 -> 108.

25. **Search Media.** *Delivered.* Media > Search Media... opens the Media window
   with a filter field focused. The pool filters rows by file name or full path,
   case-insensitively, with a clear button and an empty-result message; the term
   is transient and cleared when a project closes. 3 media-pool/app tests.
   Disabled entries 108 -> 107.

26. **Import and Export Pool.** *Delivered.* Media > Export Pool... writes a
   plain-text manifest (comment header, then `path<TAB>event count`); Media >
   Import Pool... adds the manifest's existing files to the pool as one labelled
   "Import Pool" undo step, reports already-present and missing files, and never
   copies media. Hand-written one-path-per-line lists, comments and duplicates
   are handled. 2 module/app tests. Disabled entries 107 -> 105.

27. **Insert Velocities.** *Delivered.* MIDI > Insert Velocities opens a dialog
   with start and end velocity; equal values set every selected note (or the whole
   part) to that velocity, different values ramp across the notes in time order via
   `midi_transform::set_velocity`/`ramp_velocity`. One labelled "Insert Velocities"
   undo step, no-change and no-part reported. 1 core and 1 app tests. Disabled
   entries 105 -> 104.

28. **Build N-Tuplet.** *Delivered.* Scores > Build N-Tuplet... opens a dialog for
   the note count (2-16) and span in beats, seeded from the selection. Core
   `midi_transform::build_tuplet` places exactly that many selected notes on even
   slots from the earliest note, gives each the slot length, clamps the span to the
   part end, and refuses a count that does not match the selection rather than
   dropping notes. One labelled "Build N-Tuplet" undo step. 1 core and 2 app
   tests. Disabled entries 104 -> 103.

29. **Explode Pitches.** *Delivered.* MIDI > Functions > Explode Pitches splits the
   selected part into one new MIDI track per distinct pitch: each new part keeps
   the original span and only that pitch's notes, the source part is removed, and
   controller events are copied once into the lowest-pitched part instead of being
   lost or duplicated. Bounded to 16 pitches, and a refused explode leaves the
   project unchanged. One labelled "Explode Pitches" undo step. 2 core and 1 app
   tests. Disabled entries 103 -> 102.

30. **Set Timecode at Cursor.** *Delivered.* `Project::timecode_offset_seconds`
   (bounded to 99:59:59, OPR version 30 with a v29 migration test) makes timecode
   start anywhere; `set_timecode_at_cursor` computes the offset from the cursor and
   `timecode_seconds_at` reads it back. Project > Set Timecode at Cursor opens a
   seconds dialog with a live HH:MM:SS:FF preview, applies as one labelled "Set
   Timecode" undo step, and the timecode ruler formats include the offset while
   bars and seconds do not. 1 core, 1 migration and 2 app tests. Disabled entries
   102 -> 101.

31. **Automation Panel.** *Delivered.* Project > Automation Panel lists every
   track with stored automation (plus the selected track), shows each lane's target
   and point count, and toggles that track's lane visibility in the arrange through
   the same `automation_visible` state the arrange reads, with a Hide All action.
   2 app tests. Disabled entries 101 -> 100.

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

## Revision 3 close-out (2026-11-02)

Delivered in this revision: the transport locate family, track folding, nudge and
selection keys, Insert Silence, Cut/Delete Time, typed position/locator/tempo,
Paste Time, Close Project, plain-arrow navigation, Find Selected in Pool, Adjust
Fades to Range, marker slots (OPR 29), Auto Grid, Edit In-place, the project
overview, the Gain dialog and editor entries, Auto Fades Settings, Play from
Selection, Play-until commands, menu aliases, Search Media, Import/Export Pool,
Insert Velocities, Build N-Tuplet, Explode Pitches, Set Timecode at Cursor
(OPR 30), and this Automation Panel — 31 functional slices plus this
consolidation.

Final measurements:

| Suite | Result |
|---|---|
| opentrack (app) | 463 passed, 0 failed, 9 ignored |
| opentrack-core | 134 passed |
| opentrack-audio | 152 passed, 0 failed, 3 ignored |
| opentrack-plugin-scan | 29 + 4 passed |
| opentrack-plugins | 18 passed |
| opentrack-vst3 | 14 passed, 2 ignored |
| opentrack-win-jobs | 1 passed |

- Declared-but-disabled menu entries: **159 (revision-2 start) -> 100**
- Keymap-bound commands with a working shortcut: **75 -> 146 of 152**
- Native format: **OPR version 30**; versions 1-29 migrate on open.

Remaining work is now almost entirely feature-scoped:

- **Shortcuts without a target (6):** Video, Virtual Keyboard, Open Score Editor,
  Open Tempo Track Editor, Retrospective Record, Use External Sync.
- **Interchange:** CPR (needs real fixtures), MusicXML, OMF, Track Archive, Audio CD.
- **Audio-file processing:** resample, conform, minimize, prepare archive, DC
  offset, reverse, stereo flip, time stretch.
- **Absent features:** score editor, tempo track, video, external sync,
  retrospective capture, control room, MIDI device manager, pool folders, extra
  mixers, time/remaining-time displays.
- **User verification:** every device/listening item from revision 2 still stands;
  Windows job containment is compiled but not exercised on Windows.

## Post-close continuation

Work continues on the same inert-gap objective after the close-out; each item is
still one commit with tests and refreshed counts.

32. **Reverse.** *Delivered.* Audio > Process > Reverse writes the selected
   events' used audio range, reversed, to new float32 media under `Audio/Imports`
   through `opentrack_audio::reverse_wav_range` and repoints each event at it in one
   labelled "Reverse" undo step. Position and length are preserved; pitch/time
   analysis and source-fade lineage are cleared because they describe the old
   source. A failed reversal deletes its partial files, and the command is bounded
   to 64 events. 2 audio and 1 app tests. Disabled entries 100 -> 99.

33. **Remove DC Offset.** *Delivered.* Audio > Process > Remove DC Offset reads
   the event's used range, subtracts each channel's mean, writes new float32 media,
   and repoints the events in one labelled "Remove DC Offset" undo step. The status
   reports the largest removed offset in dBFS; failures clean up their files, and
   the command is bounded to 64 events. 1 audio and 1 app tests. Disabled entries
   99 -> 98.

34. **Stereo Flip.** *Delivered.* Audio > Process > Stereo Flip swaps the left and
   right channels of the selected events' used range into new float32 media and
   repoints them in one labelled "Stereo Flip" undo step. Mono events are counted
   and left alone; the same cleanup, bounds, pitch/fade reset and undo rules as the
   other source processes apply. 1 audio and 1 app tests. Disabled entries
   98 -> 97.

35. **Normalize.** *Delivered.* Audio > Process > Normalize... opens a peak-target
   dialog (dBFS); `opentrack_audio::normalize_wav_range` scans the used range,
   scales to the target, refuses silence, and writes nothing when the material is
   already at the target. Repointed events use the same cleanup/undo rules as the
   other source processes; the status names the target and how many events were
   already level or silent. 1 audio and 1 app tests. Disabled entries 97 -> 96.

36. **MusicXML import.** *Delivered.* File > Import > MusicXML... reads a partwise
   score into one MIDI track per part. The new `opentrack_core::musicxml` parser is
   bounded (8 MiB, 64 parts, 100k notes): it reads part names, divisions, notes,
   rests, chords, backups/forwards, a tempo hint, and reports ties/slurs/
   articulations/grace notes as unsupported instead of guessing. Each import is one
   labelled "Import MusicXML" undo step; malformed or note-less files report and
   add nothing. 3 core and 1 app tests. Disabled entries 96 -> 95.

37. **Transport displays.** *Delivered.* Window > Time Display and Remaining
   Record Time Display are checkable and persist as view-state flags. The project
   readout strip now builds one line from `recording_readout_line`: the recording
   rate and format, `Max` (total recordable time from the free space), and
   optionally `Time` (transport position as H:MM:SS.mmm) and `Rem` (what is still
   recordable from that position). Missing capacity reports "unavailable" instead
   of a guess. 2 app and 1 view-state tests. Disabled entries 95 -> 93.

38. **Show Panel.** *Delivered.* Window > Show Panel is now a checkable entry bound
   to the existing persisted `show_inspector` view flag, going through one
   `toggle_inspector_panel` shared with the `editors.show_hide_inspector`
   shortcut, so the menu, keymap and toolbar cannot disagree. 1 app test (both
   entry points, status text, round trip and the legacy default). Disabled entries
   93 -> 92.

39. **Prepare Archive and Minimize File.** *Delivered.* Media > Prepare Archive...
   calls the existing, tested `consolidate_project_media`, so external sources are
   gathered into the project folder without a second implementation. Media >
   Minimize File rewrites every source used by exactly one event to just that used
   range through the new `opentrack_audio::trim_wav_range`, repointing the event
   and pool entry in one labelled "Minimize File" undo step; sources shared by
   several events, ranges that run past their file, missing files and files already
   used in full are counted and reported instead of guessed at. 1 audio and 1 app
   tests. Disabled entries 92 -> 90.

40. **Select in Project.** *Delivered.* Media Pool rows are now clickable and keep a
   transient selection (highlighted with an accent outline); Find Selected in Pool
   also sets it. Media > Select in Project selects every event that uses that
   source and reports the count, refusing with a message when no row is chosen or
   no event uses the file. 2 app tests (a simulated row click plus the command's
   three outcomes). Disabled entries 90 -> 89.

41. **Merge Clipboard.** *Delivered.* Audio > Process > Merge Clipboard mixes the
   copied events into the first selected event's used audio with the new
   `opentrack_audio::merge_wav_range`: each layer is placed at its offset from the
   copy anchor, clipped to the destination's length, mono/stereo matched by
   duplication or averaging, and a different sample rate is refused rather than
   resampled silently. One labelled "Merge Clipboard" undo step; pitch analysis and
   source-fade lineage are cleared, layers outside the destination are counted and
   reported. 1 audio and 1 app tests. Disabled entries 89 -> 88.

42. **Explode (by channel).** *Delivered.* MIDI > Functions > Explode splits the
   selected part into one MIDI track per MIDI channel through the new
   `Project::explode_midi_part_by_channel`, keeping notes and controller events on
   their own channel (a controller-only channel still gets a part, so nothing is
   dropped). It shares one app path with Explode Pitches, differing only in the
   grouping, the undo label and the wording. Bounded to 16 channels and refused
   without half-applying. 2 core and 1 app tests. Disabled entries 88 -> 87.

43. **MIDI List Editor.** *Delivered.* MIDI > Open List Editor shows the selected
   part's notes by start time with start, pitch name, length, velocity and channel;
   clicking a start selects that note and locates the transport, Delete Notes
   removes the selection as one labelled "Delete Notes" undo step, and the note
   deletion path is now shared with the MIDI editor's Delete key. Bounded to the
   selected part and reports when none is selected. 2 app tests. Disabled entries
   87 -> 86.

44. **Media Pool folders.** *Delivered.* `MediaFolders` (up to 32 named folders and
   a source-to-folder map, OPR version 31 with a v30 migration test) organises the
   pool without touching media: creating, renaming, deleting and filing are project
   content, refusals leave the project unchanged, and a source that leaves the pool
   loses its assignment. Media > Create Folder opens a name dialog (one labelled
   "Create Folder" undo step); the pool gained a Folder column, a folder filter, a
   "Move to" control for the clicked row (one labelled "Move Media to Folder" step)
   and a New Folder button. 3 core, 1 project, 1 migration and 2 app tests.
   Disabled entries 86 -> 85.

45. **Hitpoints.** *Delivered.* `opentrack_audio::detect_hitpoints_wav_range`
   finds transients in an event's used range: a 10 ms energy envelope over the mono
   sum, positive energy change, local maxima above an adaptive threshold with 40 ms
   spacing and a 2048-hit bound. Media > Hitpoints > Calculate Hitpoints runs it on
   the selected event and keeps the result as transient analysis tied to that event
   id; Create Markers turns them into timeline markers as one labelled "Create
   Markers" undo step, and a stale or missing event asks for a recalculation rather
   than guessing. Create Regions stays disabled because there is no multi-range
   region model. 1 audio and 1 app tests. Disabled entries 85 -> 83.

46. **Resample.** *Delivered.* Audio > Process > Resample... opens a rate dialog
   seeded from the project rate; `opentrack_audio::resample_wav_range` converts the
   event's used range with linear interpolation, keeping duration and pitch, and
   refuses a source already at the target rate or an out-of-range target. Repointed
   events clear pitch analysis and source-fade lineage and commit one labelled
   "Resample" undo step; failures clean up their files. 1 audio and 1 app tests.
   Disabled entries 83 -> 82.

47. **Time Stretch.** *Delivered, tape-style.* Audio > Process > Time Stretch...
   opens a percentage dialog; `opentrack_audio::stretch_wav_range` resamples the
   time axis at the source rate, so the event length scales with the audio and
   pitch moves with it. The dialog and the status say that plainly, and
   pitch-preserving time stretch is recorded as not implemented rather than
   implied. 25-400% only, 100% and non-finite values refused, one labelled "Time
   Stretch" undo step. 1 audio and 1 app tests. Disabled entries 82 -> 81.

48. **Envelope.** *Delivered (destructive).* Audio > Process > Envelope... opens a
   gain-envelope editor for the selected events: click to add a point, drag to move
   one, Remove Point, Reset, and Apply. `opentrack_audio::apply_gain_envelope_wav_range`
   multiplies the used range by a piecewise-linear envelope (points are positions
   0-1 with a 0-4 linear gain, validated) and writes new media, so playback and
   render need no new state. This was deliberately destructive rather than a
   persisted per-event curve: applying baked gain needs no audio-engine change,
   while a live curve would. One labelled "Envelope" undo step; failures clean up.
   1 audio and 2 app tests. Disabled entries 81 -> 80. No OPR bump was needed
   because the envelope is baked into the audio rather than stored on the event.

Next candidates: OMF / Track Archive, then the user-dependent reviews and CPR
fixtures.
