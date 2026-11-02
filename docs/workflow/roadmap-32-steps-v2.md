# OpenTrack roadmap revision 2: 32 tasks

Status: active, 2026-11-02. Revision 1 (`roadmap-32-steps.md`) delivered its
roadmap-named work; what remained there is either runtime evidence the user must
supply, or the CPR adapter, which waits for real fixtures. This revision keeps
going on work that is testable here, and marks the exact three places that need
the user.

Rules that stay in force:

- One task per slice: source, tests, docs, release build, push.
- A feature is not accepted because it compiles. Each task states its automated
  checks and, separately, what still needs runtime or user review.
- Project schema changes stay backward compatible for OPR/OTP/RWP.
- Audio callbacks keep their bounds (no I/O, no unbounded allocation, no UI).
- Runtime-only gates are marked **needs user** and never claimed.

## Where revision 1 left off

Delivered and tested: automation (play/read/write), recording recovery and
recovery autosave, the event edit family (split, move, crossfade, duplicate,
mute, select, repeat, fill loop, crop, invert), typed routing, the Add Track
instrument catalog, per-module scan isolation with persistent quarantine, event
lock/grouping, Convert to Real Copy, Auto Select under Cursor, Enlarge Selected
Track, Automation follows Events, and the Project Logical Editor.

Still open from revision 1:

- **Step 14/20/21** — listening passes for sends, automation and the mixer.
  **Needs user** (machine + interface).
- **Step 31 CPR** — adapter against real Cubase files. **Needs user** (the user
  is preparing fixtures).
- **Step 24 remainder** — Windows job-object containment for scanner workers;
  cannot be built or exercised on this Linux host.
- 159 declared-but-disabled menu entries; none of them is named by a revision-1
  step any more.

## Sequence

### A. Navigation and the timeline (visible immediately)

1. **Zoom command family.** *Delivered.* Zoom In (H), Zoom Out (G), Zoom In
   Vertically (Shift+F), Zoom Out Vertically (Alt+S), Zoom to Selection (Shift+E)
   and Zoom to Project (Alt+Down) drive `ArrangeViewState`. Horizontal zoom keeps
   the playhead in place when it is on screen and the view centre otherwise, both
   extremes clamp to the supported zoom range, Zoom to Selection falls back to the
   loop range and refuses when neither exists. 4 tests.
2. **Markers.** *Delivered.* `Project::markers` (id, beat, name, ordered by beat,
   bounded to 1024 entries and 96-character names, with `next_marker_id` defaulting
   for older projects and an empty list that is not serialized). Commands add,
   rename, move, remove and navigate (next/previous, strict with wrap-around).
   Insert Marker (Insert), Locate Next (Shift+N), Locate Previous (Shift+B) and
   Project > Markers (Ctrl+M, a list window with locate/rename/remove) are wired;
   markers draw a line and their name in the ruler. 2 core and 5 app tests.
3. **Follow playhead.** *Delivered.* The existing persisted `autoscroll_enabled`
   toggle now drives real paging: while the transport plays, the view scrolls when
   the playhead leaves the middle 80% and puts it back at the left tenth;
   rewinding pages back. It stands aside while a pointer button is held or a wheel
   event is handled, and resumes on the next frame. 4 tests.
4. **Snap to events and cursor.** *Delivered.* `ArrangeViewState::snap_type`
   persists a mode (Grid, Grid + Events, Grid + Cursor) and is chosen from the grid
   popup, with the mode named in the snap button's tooltip. Event snaps collect
   every clip start/end and MIDI part boundary each frame; the drag snaps to a
   target only when it is inside an 8-pixel tolerance *and* closer than the grid
   point, so the grid stays predictable, and an unset grid still means snapping is
   off entirely. 2 tests.

### B. Arrangement editing depth

5. **Nudge selection.** *Delivered for left/right.* `nudge.left` (Ctrl+Left) and
   `nudge.right` (Ctrl+Right) move the selection by one grid step (a quarter bar
   when snap is off) through the same core move that honours locks, groups and
   Automation follows Events, and refuse at the project start or timeline end.
   Edge nudging (Alt+Left/Right, Alt+Shift+Left/Right) remains open.
6. **Split Loop.** *Delivered.* `Project::split_audio_clips_at_loop` splits every
   audio event crossing either locator (an event spanning the whole loop becomes
   three), skipping locked events, and the Edit menu command is one undo step.
   2 core and 1 app tests.
7. **Glue events.** *Delivered for what needs no rendering.*
   `Project::glue_audio_clips` joins contiguous pieces of one source on one track:
   same source file, equal gain, no Pitch & Time edit, no fade lineage, and
   contiguous in both timeline beats and source seconds. The earliest piece keeps
   the identity, the outer fades are zero, and Edit > Glue is one undo step.
   Joining *different* sources would need a bounce, which is task 23.
8. **Range delete and trim to range.** *Delete range delivered.*
   `Project::delete_range_from_audio_clips` removes `[start, end)` from the
   selected events: an event spanning the range keeps both outer parts, one that
   starts inside keeps its tail under the same id, one fully inside is removed,
   locked events are skipped, and the gap is not closed (this is not a ripple
   edit). Edit > Range > Delete Range runs it as one undo step. Trim-to-range is
   the existing Crop command; nothing further is needed there.
9. **Undo history window.** *Delivered.* Every history entry now carries a label,
   set by the command that made the change (Split at Cursor, Duplicate, Nudge,
   Glue, Process Logical Editor, …) and falling back to "Edit". Edit > Undo
   History... lists the steps newest-first with the undone ones marked, their
   snapshot size on hover, and jumps to any step by applying the undos or redos in
   order; the bounded history limit is unchanged. 5 tests.

### C. Project and media robustness

10. **Media pool consolidation.** *Delivered.* Project > Consolidate Media copies
   every referenced source outside the project into `Audio/Imports` once, repoints
   every event, rewrites the media pool and the open audio sources, and reports
   what moved, what was already local and what is missing. Missing sources are
   reported, never dropped. One undo step. 2 tests.
11. **Missing media detection and relink.** *Delivered.* `Project::missing_media`
   reports referenced sources that are not on disk, and `relink_media` repoints
   the events and the media pool. Project > Relink Missing Media (N)... opens a
   folder picker, searches it (depth 6, 20 000 entries) for files with the same
   name case-insensitively, and applies the matches as one undo step labelled
   "Relink Media", opening the relinked audio and reporting anything still
   missing. 1 core and 1 app test.
12. **Multiple recovery points.** *Delivered.* Autosave now writes timestamped
   points (`Metadata/Recovery/recovery-<ms>.opr`), keeps the newest 5 and prunes
   the rest on disk. Opening a project with points lists each one with its age and
   size, recovers the chosen copy, or discards them all; a single legacy
   `autosave.opr` is still offered. 2 tests.
13. **Project templates.** *Delivered.* `Project::as_template` keeps tracks,
   groups, routing, gains, EQ, inserts, sends and automation lanes and drops
   events, MIDI parts, markers, the media pool and the selection, resetting the
   timeline. Project > Save as Template... names it; Project > New from Template
   lists the saved ones with their track counts and starts a new project from the
   chosen one. Templates live in the app data directory
   (`templates/*.otemplate.json`, `OPENTRACK_DATA_DIR` overrides it for tests) with
   a versioned format, sanitized names and a bounded listing. 5 tests.
14. **Session health check.** *Delivered.* Project > Session Health... collects
   the checks OpenTrack already performs — save validation, routing validation,
   missing media (with the relink command named), failed VST3 inserts, an undo
   snapshot approaching its limit, a stale insert failure from a deleted channel —
   plus a summary of tracks/events/parts/markers and the project folder and file,
   ordered errors → warnings → info with a one-line status. 6 tests.

### D. MIDI depth

15. **Quantize with strength and swing.** *Delivered.*
   `opentrack_core::midi_quantize` quantizes the selected notes (or the whole part)
   to a beat grid with a strength fraction, a swing offset on every second line,
   and optional length snapping, clamping every note inside its part and leaving
   already-quantized notes alone. The Edit menu's "Quantize MIDI..." dialog starts
   from the toolbar's quantize grid and applies as one undo step labelled
   "Quantize Notes". A live preview is not part of this slice. 7 core and 4 app
   tests.
16. **Transpose and velocity tools.** *Delivered.*
   `opentrack_core::midi_transform` transposes the selection by whole semitones,
   skipping notes that would leave 0..=127 rather than clamping them onto an
   unasked octave, and offsets velocity with clamping at 1 and 127. The Edit menu's
   "Transpose / Velocity..." dialog applies either as its own labelled undo step.
   4 core and 2 app tests.
17. **MIDI part editing.** *Delivered.* `Project::split_midi_part` cuts a part at
   an absolute beat — notes and controller events before the cut stay, those after
   move, and a note crossing the cut is itself split so no played note is lost —
   and `Project::merge_midi_parts` joins parts on one track into the earliest
   part's span, keeping every note at its musical position and following the
   selection. The arrange's Split tool now calls the same core rule. Edit > Split
   MIDI Part at Cursor and Merge MIDI Parts are labelled undo steps; merge takes
   the selected part plus any part touching or overlapping it. 2 core and 2 app
   tests.

### E. Recording and monitoring

18. **Punch in/out recording.** *Delivered at control rate.* A persisted toggle
   (Transport > Punch In/Out) records only between the locators: with the playhead
   before the in point, record arms and the transport rolls; capture starts when
   the playhead reaches the in locator and stops at the out locator while playback
   continues. The decision runs on the control thread, so a punch lands within a
   frame of the locator rather than sample-accurately — the engine was not
   restructured for this and the difference is stated rather than hidden. 5 tests.
   Needs device review with a real interface.
19. **Input monitoring modes.** *Delivered.* `Track::monitor_mode` persists Off,
   Auto (while armed) or On, and `monitors_input` is what the playback plan and the
   input-monitor request read, replacing the old boolean. The strip's MON control
   cycles the three states and names the current one on hover; the Channel Settings
   snapshot carries the mode and forces Off on device bus strips. A project saved
   with the old `monitor` flag loads as On, so no one loses monitoring on upgrade.
   3 app and 1 core test.
20. **Loop recording takes.** *Delivered.* A recording made with Cycle active is
   split into one clip per pass (`Project::add_loop_take_clips`): each pass starts
   at the loop start and reads further into the recorded file, all passes share a
   `take_group`, and only the newest is `take_active`. Inactive takes are kept,
   drawn dimmed with a `T`, and excluded from playback by `clip_is_audible`. Edit >
   Make Take Active switches which pass plays as one undo step. 3 core and 2 app
   tests. A device run of a real multi-pass take remains.
21. **Metronome configuration.** *Delivered.* The engine's click now takes an
   accent mask (`MetronomeSettings::accent_mask`, one bit per beat of the bar) in
   addition to the level and pitch it already applied. Metronome Setup adds a
   per-beat accent row clipped to the project's bar length and a 0–4 bar count-in;
   both persist (`metronome-accent-mask`, `metronome-count-in-bars`). A count-in
   rolls the transport back, forces the click on for the lead without changing the
   user's toggle, and starts capture at the original position; punch takes
   precedence when both are on. 3 engine and 4 app tests. Listening review
   remains.

### F. Delivery and interchange

22. **Stem export.** *Delivered.* File > Export > Selected Tracks (Stems)... opens
   the mixdown dialog with a stems checkbox and a folder picker instead of a file
   picker. Each audible track is rendered through the same `OfflineRenderer` and
   the same format/range/sample rate as a mixdown, with only that track's channel
   in the graph: sends are dropped (they belong to the mix) and the output is
   forced to the stereo master, while EQ, inserts, hosted inserts, gain and
   automation stay. Files are numbered and name-sanitized (`001 Drums.wav`), a
   track with no audio writes nothing and the status reports the shortfall. 5
   tests, one of which renders two real WAVs end to end.
23. **Bounce selection in place.** *Delivered.* Edit > Bounce Selection in Place
   renders the selected events' span through their own track's channel (the same
   `OfflineRenderer` and settings as a stem, sends dropped, output to the master)
   into `Audio/Imports`, then replaces the selected events with one event covering
   the span as a single undo step labelled "Bounce in Place". Replaced audio stays
   in the media pool, a locked event blocks the bounce, a selection spanning two
   tracks is refused, and the render runs in a background thread with a cancel
   button. 2 core and 1 app test; the app test renders a real two-event bounce and
   checks the resulting file, span and history label. Listening review remains.
24. **MIDI export of a selection** in addition to the whole project.
25. **CPR adapter (blocked).** Version-specific reader with an explicit support
    report and opaque preservation. **Needs user fixtures**; until they arrive
    this task stays unstarted and unclaimed.
26. **CPR fixture harness (blocked, independent half).** A round-trip test
    harness that takes a folder of fixtures and records per-file findings, so
    the adapter starts from evidence the moment files exist.

### G. Platform and performance

27. **Windows job-object containment for scanner workers** — implemented behind
    `cfg(windows)` with the isolation contract kept; cannot be exercised on this
    host, so it ships unverified and marked as such.
28. **Callback budget audit.** Instrument the audio callback path with bounded
    counters, assert no allocation in the callback in tests, and record the
    measured worst case per stage.
29. **Linux VST3 editor embedding** — X11 embed path behind the existing host
    boundary. **Unverified without an installed plug-in**; ships only if a test
    plug-in can be produced in the same task, otherwise it stays planned.
30. **Startup time and large-project load.** Measure and improve the load path
    with a generated large project fixture.

### H. Consolidation

31. **Accessibility and keyboard coverage.** Focus order, labelled controls,
    contrast of disabled text, and a report of which menu entries are reachable
    by keyboard alone.
32. **Consolidate revision 2.** Reconcile this plan with the delivered code,
    re-measure the menu inventory and the suites, and choose revision 3.

## Dependencies and honest blockers

Only three items depend on the user: the listening passes (tasks not in this
plan; revision-1 steps 14/20/21), CPR fixtures (tasks 25 and 26), and anything
that needs Windows or an installed VST3 plug-in to be verified (tasks 27, 29).
Every other task is implementable and testable on this machine.

## Verification rules for this revision

- Each task adds tests where the behavior is testable without a device.
- A task that changes the project schema ships a round-trip test and a legacy
  default test.
- A task that changes the audio plan states its callback cost.
- Docs updated in the same commit: this file's task line, `current.md`, and the
  relevant task packet.
