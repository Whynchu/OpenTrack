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
3. **Follow playhead.** A persisted toggle plus scroll behaviour that keeps the
   playhead visible during playback, and stops fighting the user while they
   drag-scroll.
4. **Grid and snap for the arrange beyond the current presets:** snap to events
   and to the cursor, with the mode shown in the grid control.

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
7. **Glue events.** Join adjacent or overlapping events on one track into a
   single event that keeps both sources' audio readable.
8. **Range delete and trim to range.** Delete the loop range from the selected
   events, and trim to range as the inverse of Crop.
9. **Undo history window.** List recent undo steps, jump to one, persisted per
   session; the truncation limit stays bounded.

### C. Project and media robustness

10. **Media pool consolidation.** Copy every referenced source into the project's
    `Audio/` folder and repoint references, with a report of what moved.
11. **Missing media detection and relink.** Report events whose source is gone,
    search a folder for matching names, and relink as one undo step.
12. **Multiple recovery points.** Keep the last N recovery copies and let the
    user pick one, instead of one autosave file.
13. **Project templates.** Save the current project's structure as a template and
    create new projects from it.
14. **Session health check.** A dialog that reports unsupported sample rates,
    missing media, dangling routes, failed inserts and oversized projects in one
    place.

### D. MIDI depth

15. **Quantize with strength and swing**, as one bounded core command plus a
    dialog, with preview.
16. **Transpose and velocity tools** over the selected notes.
17. **MIDI part editing**: split a part at the cursor and merge adjacent parts.

### E. Recording and monitoring

18. **Punch in/out recording** driven by the locators, with a persisted toggle.
19. **Input monitoring modes** (off/auto/on) per track, persisted, reflected in
    the plan and the mixer.
20. **Loop recording takes.** Repeated cycle passes create take lanes instead of
    overlapping events, with a way to pick the active take.
21. **Metronome configuration** (accent pattern, level, count-in) persisted and
    applied by the engine.

### F. Delivery and interchange

22. **Stem export.** Render every track (or the selected ones) to its own WAV
    with the existing offline renderer, sharing the mixdown dialog's settings.
23. **Bounce selection in place.** Render the selected range through the track
    chain into a new event on that track.
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
