# T083 — Add Instrument Track chooses from the scanned VST3 catalog

- Status: review_pending (source + automated checks complete; runtime review open)
- Scope version and requirement IDs: roadmap step 26.
- Desired observable outcome: adding an instrument track lets the user pick one of
  the scanned VST3 instruments (or none), and the chosen instrument is loaded on
  the new track and saved with the project.
- Reference evidence: user verification that VST3 insert load works on hardware
  (2026-11-01); the dialog previously showed "Instrument unavailable".
- Dependencies and decisions needed: none. Preset browsing and speaker layouts stay
  labeled unavailable.
- Code scope and architecture boundaries:
  - `app.rs` — `TrackAddDialog.instrument`, the catalog combo box, and
    `load_dialog_instrument`, which inserts the plug-in as the track's first insert.
- Acceptance checks (automated today):
  1. The chosen instrument becomes the new track's first insert with the right
     class id and name, and the index counts instruments only.
  2. No selection, an out-of-range index, or no instrument track leaves the project
     unchanged.
  3. An audio track never receives the dialog's instrument.
- Affected regressions: track creation, project save/reload of inserts.
- Essential gates: persistence (the insert is ordinary project data), runtime
  (loading the plug-in needs a desktop review).
- Review owner and milestone stop: user, at the next desktop session.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash; user review pending.
- Source identity: commit `c159734`; release `builds/x64/OpenTrack` `ba7e71da…`.
- Changes and reasons: the dialog declared the feature unavailable; the catalog was
  already scanned and only the selection was missing. Loading through the insert
  rack reuses the existing persistence and hosting path rather than a second one.
- Commands/checks and actual results:
  `cargo test -p opentrack add_track_instrument` 3 passed.
- Runtime environment and evidence paths: Linux/XWayland build; no instrument was
  loaded at runtime.
- Review findings, rework: none yet.
- Failures/unknowns/pending acceptance: loading an instrument from this dialog has
  not been exercised with real plug-ins; on Linux the native editor is still
  Windows-only.
- Next ready task: none — the next revision leads with runtime acceptance.
