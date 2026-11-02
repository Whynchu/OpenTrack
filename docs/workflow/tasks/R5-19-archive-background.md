# R5-19 — Background Track Archive import

- Status: accepted
- Scope version and requirement IDs: revision 5, step 19
- Desired observable outcome: importing a Track Archive runs off the UI thread with progress, cancellation, stale-result rejection and no partial project publication.
- Reference evidence (IDs/paths): `prepare_import`, `start_track_archive_import`, `poll_track_archive_import`, `cancel_track_archive_import`, `finish_track_archive_import`, `TrackArchiveImportTask` in `crates/opentrack-app/src/app/track_archive.rs`.
- Dependencies and decisions needed: R5-19's predecessor (archive round trip) is revision 4; no user decision needed.
- Code scope and architecture boundaries: the worker only uses `prepare_import` (file work + track remapping); the project is touched on the UI thread when the result arrives.
- Acceptance checks: cancellation before any copy stages nothing and reports; a background import completes and publishes one "Import Track Archive" step; a result whose project snapshot is stale is rejected without touching the project; the menu row is gated while a task runs.
- Affected regressions: none; the existing `prepare_import` tests gained the cancel/progress arguments.
- Essential gates: bounded job, cancellation, stale rejection, single history step, no partial publish.
- Review owner and milestone stop: self-review.

## Contract

- `prepare_import` checks the cancel flag before creating the import folder and before each media copy, and reports progress as `(copied, total)`.
- The task carries a project fingerprint taken at start; if the project changed by the time the result arrives, the result is discarded with a reason instead of being applied to a project that moved on.
- Dropping the task (project close, project switch) sets the cancel flag, so a stale worker cannot publish.
- Diagnostics: the status line reports `Importing track archive: N of M media file(s)` while running.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: replaced the synchronous import with a worker + `TrackArchiveImportTask`; added cancel/progress to `prepare_import`; split publication into `finish_track_archive_import`; gated the menu row while a task exists.
- Commands/checks and actual results: `cargo test -p opentrack track_archive` → 6 passed (3 new); app suite 569 passed / 4 known device failures / 9 ignored; core 166; audio 170.
- Runtime environment: Kubuntu Linux; the background worker is exercised in a test that waits for completion.
- Ledger IDs updated: inventory regenerated after the menu gating change.
- Review findings: the first cancel check ran after creating the import folder, so a cancelled import left an empty folder; the check now runs first and the test asserts no staged entries.
- Failures/unknowns/pending acceptance: a huge archive with thousands of media files is not exercised here; the 4096-entry manifest bound is the only guard.
- Next ready task: R5-20.
