# T080 — Recovery autosave for unsaved project changes

- Status: review_pending (source + automated checks complete; desktop review open)
- Scope version and requirement IDs: roadmap step 29, second half. Also covers the
  window close that eframe will not let the app prompt on.
- Desired observable outcome: closing or killing OpenTrack with unsaved project
  changes does not lose them; opening the project again offers the recovery copy
  and the user chooses to take it back or drop it.
- Reference evidence: eframe 0.36 does not deliver `ViewportEvent::Close` reliably
  for the WM close button (recorded earlier in `current.md`), so a close prompt is
  impossible; the autosave is the substitute.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries:
  - `app/src/project_recovery.rs` — paths, staging, pending detection, age text.
  - `app.rs` — `maybe_autosave`, `detect_unsaved_recovery`, `show_project_recovery`,
    `apply_recovered_changes`.
  - `project_file.rs` — the copy is written with `ProjectFile::write`, the same
    validated atomic path as a save.
- Acceptance checks (automated today):
  1. An autosave lives in `Metadata/Recovery/autosave.opr` and `prepare` creates
     the folder.
  2. Missing and empty copies are not pending; clearing is idempotent.
  3. The age text is readable at every scale.
  4. Unsaved changes produce a copy that reads back as a real project file.
  5. An unmodified project, or one in record mode, produces none.
  6. The interval is respected (a drag does not rewrite the project).
  7. Recovering replaces the project with the copy, stays modified, and keeps the
     copy until the next save.
  8. A completed save and an explicit discard both remove the copy; the prompt
     renders.
  9. Opening a project whose saved file lost a track offers the copy and restores it.
- Affected regressions: save path, undo/dirty state, discard-on-close.
- Essential gates: persistence (copy + project), undo (recovery is one step),
  recovery (copy removed on save/discard).
- Review owner and milestone stop: user, at the next desktop session.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash; user review pending.
- Source identity: commit `7a25e60`; release `builds/x64/OpenTrack` `d2410089…`.
- Changes and reasons: without a close hook the only safe answer is a periodic
  copy of unsaved state, written through the validating save path so a half-written
  copy can never replace a good one.
- Commands/checks and actual results: 3 module tests + 6 app tests pass
  (`cargo test -p opentrack project_recovery`, 9 passed).
- Runtime environment and evidence paths: Linux/XWayland build; no desktop review.
- Review findings, rework: none yet.
- Failures/unknowns/pending acceptance: edits made in the last few seconds before a
  kill are lost by design (20 s interval); plug-in state is not captured in the
  copy, so a recovered project may hold older plug-in states than its last save.
- Next ready task: T081.
