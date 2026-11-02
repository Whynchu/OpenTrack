# R4-13 — Define and persist FX-channel identity

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 13; user decision 2026-11-02 (FX channels are shared returns accepting sends from multiple tracks with cycle rejection; hold changes to existing behavior for review).
- Desired observable outcome: a project can contain a distinct FX channel kind with no audio/MIDI events, no input route and no folder membership, but with the normal channel fader, pan, EQ, inserts and bus signal path. The kind persists through the native document (OPR 35) and older projects load unchanged. Invalid content is rejected by validation rather than silently dropped. This step does not yet enable Add FX Channel or sends to FX; steps 14–15 own those.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: FX channels are shared effect returns; multiple tracks may send to them; feedback cycles are rejected; already implemented behavior is preserved unless held for review.
  - Observed: `TrackKind` previously held Audio, Group, Instrument and Midi only. Group channels already provide bus DSP, insert processing, fader/pan/EQ and routing in both live and offline paths through `PlaybackTrackSettings.is_group`. `Project::validate` already rejects audio clips outside Audio tracks and MIDI parts outside Midi/Instrument tracks. Track duplicates clone the kind and channel settings; deletion already reparents children and does not treat a removed track's clips as disposable media.
  - Observed: the live plan maps `track.kind == Group` to the engine bus flag. The export path shares the same descriptor builder. Mixer and Channel Settings iterate all project tracks, so an FX channel's fader, EQ and inserts already have a display/consumer once the kind exists.
  - Unknown: desktop visual/interaction review; send routing, add menus and end-to-end FX workflow belong to steps 14–16.
- Dependencies and decisions needed: none. Step 14 needs the kind persisted and validated first. No new dependency.
- Code scope and architecture boundaries:
  - `opentrack-core::project`: add `TrackKind::Fx`, `is_bus()`/`owns_midi()` helpers, creation prefix/width handling, and validation (no parent group, no input route, no clips, no MIDI parts). Duplication/removal/history use existing generic paths.
  - `opentrack-app`: map `kind.is_bus()` to the playback/export bus flag; label FX rows, icons, statistics and the add-track configuration dialog as FX; keep automation read/write unavailable on FX like other buses.
  - `project_file`: bump OPR to 35 and add an FX round-trip test. No change to group routing semantics in this step.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. `add_track(Fx)` creates a uniquely named FX channel with stereo/mono width, output to Stereo Out, no input route and empty clips/MIDI; `validate` accepts it and the live/offline plan treats it as a bus.
  2. Pushed clips or MIDI parts, a parent group, or an input route on an FX channel are rejected with the named validation error; core add commands refuse to create media/MIDI on it.
  3. Duplicating an FX channel copies kind, gain, pan, EQ and inserts with fresh identity; deleting it removes only that channel.
  4. Serialization round-trips the kind; OPR 35 opens older projects and persists an FX channel. Generated inventory and full suites pass.
- Affected regressions: project validation, group/parent logic, duplicate/delete, playback-plan bus mapping, export-plan bus mapping, statistics, add-track dialog labels, OPR migration.
- Essential gates: content-only invalid data is rejected; no events on FX; persistence and old-project defaults; no change to existing group/audio routing behavior.
- Review owner and milestone stop: user review. Self-review only in this run; no independent reviewer claimed.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: based on `fbdba76`; commit recorded in Git history.
- Changes and reasons: added the persisted FX kind with bus semantics and validation; mapped FX to the existing bus DSP instead of a second engine path; labelled FX in Project rows, statistics, icons and the add-track dialog; bumped OPR to 35. Add FX menu/sends remain disabled by design until steps 14–15. Packet prepared after the source edits for this slice; the scope above is the implemented scope.
- Commands/checks and actual results: focused core FX tests 2 passed; OPR FX round-trip 1 passed; core suite 154 passed; full app suite 529 passed, 9 ignored; formatting and diff checks run. Inventory remains 271 project-open leaves with 86 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless tests only. No desktop, listening or hardware review.
- Ledger IDs updated; report command/result: README native version raised to OPR 35; current.md updated after checks.
- Review findings, rework, elapsed effort if available: self-review only; the statistics test was updated for the new FX row and fixture.
- Failures/unknowns/pending acceptance: FX insertion/fader audible behavior, send routing, add/delete UI workflow and desktop visuals remain pending; step 14 must extend typed destinations and cycle checks to FX.
- Next ready task: revision-4 step 14, route sends to FX channels.
