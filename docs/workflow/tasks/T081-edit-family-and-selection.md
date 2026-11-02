# T081 — Edit family, command unification and selection identity

- Status: review_pending (source + automated checks complete; runtime review open)
- Scope version and requirement IDs: roadmap steps 25, 27 and 28.
- Desired observable outcome: the Edit menu's everyday event commands all work on
  the current selection, each is one operation shared with the leave filter, and
  every surface agrees about which track and event are selected.
- Reference evidence: the user's preferred Cubase-style event workflow, taken as a
  feature input for this slice. Duplicate places a copy after the selection span;
  Mute keeps the event in place but silent.
- Dependencies and decisions needed: none. Repeat/Fill Loop, Crop, Invert, Lock and
  event grouping (binding events so they move together) are explicitly out.
- Code scope and architecture boundaries:
  - `core/src/project.rs` — `AudioClip.muted`, `duplicate_audio_clips`,
    `set_audio_clips_muted`.
  - `app/src/audio_events.rs` — the eight commands, each with the
    `finish_frame` → `begin` → candidate → `commit_after_frame` shape.
  - `app/src/app.rs` — menu arms and `dispatch_reference_command` arms.
- Acceptance checks (automated today):
  1. Duplicating one event places the copy immediately after it, keeping source,
     trim, fades, gain and mute.
  2. Duplicating a selection shifts every event by the selection span.
  3. Duplicating nothing or an unknown id changes nothing.
  4. Muting is reversible and reports changed events.
  5. `muted` round-trips and an older project loads unmuted.
  6. A muted event is excluded from playback but keeps its position.
  7. The app actions duplicate, mute, toggle, select all and select none as undo
     steps and move the selection to the copies.
  8. Range selection picks the events overlapping the loop; locators take the
     selection's outer edges.
  9. Split at Cursor splits a selected group segment through the group.
  10. A real arrange click selects the event and moves `Project::selected_track` to
      that event's track.
- Affected regressions: clip edit commands, playback plan, arrange drawing.
- Essential gates: persistence (`muted`), undo (each command one step), audible
  result (muted events are silent), dirty state.
- Review owner and milestone stop: user, at the next desktop session.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash; user review pending.
- Source identity: commit `f22a170` (edit family), `c3f08c9` (selection regression
  and harness); release `builds/x64/OpenTrack` `8b3d9659…`.
- Changes and reasons: the menu showed Duplicate, Mute and Select as disabled
  placeholders; the commands existed nowhere. The clip needed a persisted mute
  because "silent but still arranged" is a different state from a short fade.
- Commands/checks and actual results: `cargo test -p opentrack-core` 51 passed
  (5 new); `cargo test -p opentrack` edit tests 4 passed plus the selection
  regression, whole suite green after T084.
- Runtime environment and evidence paths: Linux/XWayland build; no desktop review.
- Review findings, rework: none yet.
- Failures/unknowns/pending acceptance: Duplicate at the project's right edge can
  push a copy past `MAX_LOOP_BEATS`, in which case it is refused with a status
  message rather than clipped; not yet exercised in the UI.
- Next ready task: T082.
