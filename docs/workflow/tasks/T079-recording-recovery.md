# T079 — Incomplete-take recovery after an abnormal exit

- Status: review_pending (source + automated checks complete; device review open)
- Scope version and requirement IDs: roadmap steps 23 and 29; recording reliability.
- Desired observable outcome: after OpenTrack stops while recording (crash, kill,
  power loss), opening that project again offers the unfinished takes, restores
  any of them into the project, and never destroys audio the user did not
  explicitly discard.
- Reference evidence: user hardware verification of ordinary recording on
  2026-11-01 (Mackie interface + laptop built-ins). The abnormal-exit path itself
  is unverified on hardware.
- Dependencies and decisions needed: none. A real kill-during-recording review is
  the remaining gate.
- Code scope and architecture boundaries:
  - `audio/src/recording_recovery.rs` — WAV header repair (`repair_take`).
  - `app/src/recording_recovery.rs` — journal (`Metadata/Recording/in-progress.json`).
  - `app.rs` — writes the journal before the recorder creates files, settles each
    entry once its take is in the project, and shows the prompt.
- Acceptance checks (all automated today):
  1. A WAV whose header was never finalized reads as empty and becomes readable
     after `repair_take`, with the same samples. (`repair_restores_a_header_that_was_never_finalized`)
  2. Repairing a finalized take changes nothing. (`repair_of_a_finalized_take_changes_nothing`)
  3. A partial trailing frame is dropped. (`repair_drops_a_partial_trailing_frame`)
  4. Non-WAV and missing files report an error. (`repair_rejects_files_that_are_not_recordings`)
  5. The journal round-trips portable paths and clears on success.
  6. Committed, missing and empty takes need no decision.
  7. Settling one take keeps the rest; discarding moves the file aside without
     overwriting.
  8. Restoring repairs the header, places the clip at its recorded beat, loads the
     source and clears the journal; a missing track creates "Recovered Take".
- Affected regressions: ordinary recording and `finish_all` error reporting.
- Essential gates: persistence (journal + project), undo (one step per restore),
  recovery (discard is reversible by hand from `Metadata/Recording/Discarded`).
- Review owner and milestone stop: user, at the next desktop session.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash; user review pending.
- Source identity: commit `89b4255`; release `builds/x64/OpenTrack` `61a03d73…`.
- Changes and reasons: the recorder finalizes its WAV header only at the end, so a
  kill leaves audio that reads as empty and no project reference. The journal
  makes the intent durable before any file exists; repair reconstructs lengths
  from the real file length.
- Commands/checks and actual results: `cargo test -p opentrack-audio recording_recovery`
  4 passed; `cargo test -p opentrack` recovery tests 10 passed; full suites green
  as recorded in `docs/workflow/current.md`.
- Runtime environment and evidence paths: Linux/XWayland build; no device review.
- Review findings, rework: none yet.
- Failures/unknowns/pending acceptance: a real kill during recording; a take that
  spans a power failure with an unrecoverable partial frame beyond the last
  whole frame is truncated, which is intended but unverified with real hardware.
- Next ready task: T080.
