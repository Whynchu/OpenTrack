# R4-02 — Review the recent media pipeline before extending it

- Status: review_pending
- Scope version and requirement IDs: roadmap revision 4, step 2
- Desired observable outcome: slices 41-56 are reviewed against locked clips, partial failure, source identity/range/channel conversion, pool mapping, undo/file lifetime and stale results; confirmed defects are fixed in bounded commits; `pool_record_folder` cannot write outside the project tree through lexical forms or an existing junction/symlink.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed source: `crates/opentrack-app/src/media_pool.rs` (conform/convert/detect silence/pool folders), `audio_events.rs`, `audio_export.rs`, `app.rs` import destination, `opentrack-core/src/project.rs` pool-folder validation, `opentrack-audio` WAV processing.
  - Slices under review: `f25e6db` (41) through `12a01e0` (56).
  - Existing evidence: app/core/audio suites reported passing at the planning baseline; not rerun here yet.
  - Unknown: live audio listening and UI interaction; non-NTFS link behaviour on other hosts.
- Dependencies and decisions needed: none. Locked-clip semantics are interpreted as: panel bulk bakes operate on pool sources and remain available, but a source used by a locked event must not be changed in a way that alters that event's placement or duration without explicit user intent (see findings below).
- Code scope and architecture boundaries: a small app filesystem-containment module used by import/export/conform/convert destinations; lexical validation in `opentrack-core`; focused tests. No new dependencies, schema change or broad refactor.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. A configured pool folder that resolves through a junction/symlink to a directory outside the project is refused and no directory/file is created outside.
  2. Absolute, drive-relative, UNC, `..`, ADS (`:`), trailing-dot/space and Windows device-name components are refused by `set_pool_record_folder`; inherited bad values are rejected by `Project::validate`.
  3. A multi-file convert whose second item cannot be read leaves the first converted file cleaned up, the project unchanged and no undo step.
  4. Existing pool/media tests and the focused core/app checks pass; `git diff --check` passes.
- Affected regressions: media import/export destinations, conform/convert, core project validation, project round trip.
- Essential gates: no original overwrite; no partial committed project; rejected paths create nothing; undo/reopen retain usable media (runtime reopen remains pending).
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `7aff803` plus this slice; commit recorded in Git history.
- Changes and reasons:
  - New `crates/opentrack-app/src/media_paths.rs` resolves every pool destination through the project root, rejects an existing junction/symlink whose canonical target leaves the project, and re-checks after creation. Import (`audio_import_destination`) and all generated WAV destinations (`unique_wav_path`) use it, covering import, render/export, conform, convert, reverse, DC removal, stereo flip, envelope and normalize outputs.
  - `Project::set_pool_record_folder` now also refuses trailing-dot/space components, invalid filename characters, and Windows device names (`CON`, `NUL`, `COM1`…), while still refusing absolute, drive-relative, UNC, ADS-colon and `..` forms.
  - Destructive source bakes now refuse the whole operation when a selected event is locked, matching the timeline operations: reverse, DC removal, stereo flip, merge clipboard, detect silence, gain envelope and normalize. Pool-wide convert/conform remain source maintenance, not event edits, and are documented as such.
- Commands/checks and actual results:
  - `cargo test -p opentrack-core` — 146 passed.
  - `cargo test -p opentrack-audio` — 167 passed, 3 ignored.
  - `cargo test -p opentrack --bin opentrack` — 497 passed, 9 ignored, 0 failed. (Plain `cargo test -p opentrack` is blocked by three pre-existing VST probe examples that reference `crate::crash_log`; this is the documented baseline limitation, not a new failure.)
  - Focused: junction test creates a real `mklink /J` escape on this host, asserts refusal and that the outside `Imports` folder is not created; failed-second-convert test asserts cleanup, unchanged project and no history; locked-reverse test asserts no file and no history; core folder test covers drive-relative, UNC, ADS, device names and trailing dots/spaces.
  - `cargo fmt --all -- --check` and `git diff --check` pass.
- Runtime environment and evidence paths: Windows x64 host; system-temp generated WAVs; no app launched. The junction test output confirms the active host path.
- Review findings:
  - No partial project commit found in conform/convert: each builds a candidate project, cleans created files on failure, validates, and only then takes one labelled undo step.
  - Generated WAVs keep the original sources; failures remove only files created by the operation.
  - Import/close/project-change paths guard the single background import worker; conform/convert/processing are synchronous and cannot publish a stale result.
  - Locked bakes were a real gap and are fixed. Pool-wide conform still changes timing for locked users' shared sources; that remains an explicit limitation because it is source maintenance, not an event edit.
- Ledger IDs updated; report command/result: regenerated `docs/reviews/generated-source-inventory.md` after the source change.
- Failures/unknowns/pending acceptance: live desktop interaction, listening, and non-NTFS link behavior on other hosts remain unverified. Context menus are still outside the generated inventory.
- Next ready task: revision-4 step 3, establish the command boundary for this revision.
