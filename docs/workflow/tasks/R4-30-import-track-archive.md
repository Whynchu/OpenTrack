# R4-30 — Enable Import Track Archive with a real round trip

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 30.
- Desired observable outcome: File > Import > Track Archive (OpenTrack v1)... validates the manifest signature/version, bounded counts, relative media paths and canonical containment, copies media into the project, allocates new track/clip/MIDI IDs, remaps hierarchy and internal bus routes, disconnects unresolved external routes with a report, and appends imported tracks as one undoable history step. Malformed/path-escaping packages and missing files change nothing.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: R4-29 writes `opentrack-track-archive` v1 manifests. `Project` has stable ID counters and validation. Imported media must live in the managed project folder.
  - User-specified: OpenTrack format only; no OMF/CPR compatibility claim.
  - Unknown: desktop dialog flow and fresh-process save/reopen; headless round trip covers data.
- Dependencies and decisions needed: R4-29 delivered. Import currently runs synchronously on the UI thread; large packages may briefly block a frame (recorded limitation).
- Code scope and architecture boundaries:
  - `track_archive::prepare_import` validates, copies media, remaps IDs/hierarchy/routes and returns prepared tracks; `refresh_counters` updates project ID counters.
  - App import clones the project, appends prepared tracks, refreshes counters, validates, then commits one "Import Track Archive" history step. No partial project assignment.
  - Import menu wording names the implemented OpenTrack v1 format.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Export→import round trip preserves hierarchy/media and allocates fresh IDs/names; source project remains unchanged.
  2. Duplicate names receive a suffix; unresolved bus routes are disconnected and reported.
  3. Malformed format/version, traversal paths, missing media and invalid data are refused without changing tracks.
  4. Import is one undo step; imported media remains in the project folder after undo/redo.
  5. Fresh-project save/reopen can read imported tracks; full suites/inventory/formatting pass.
- Affected regressions: project IDs/validation, routing, history, media lifetime, import menu.
- Essential gates: atomic apply after preparation; path containment; no lost sources.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `2c83470`; commit recorded in Git history.
- Changes and reasons: added manifest validation, contained media copy, ID/hierarchy/route remapping, counter refresh, atomic app import and the OpenTrack v1 import menu entry.
- Commands/checks and actual results: archive tests 3 passed (export, external route report, import round trip and traversal refusal); full app suite 544 passed, 9 ignored; inventory 275 leaves with 83 unavailable. Core/audio unchanged from 164/171.
- Runtime environment and evidence paths: Windows x64 host; headless filesystem only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; `refresh_counters` was added after validation exposed stale ID counters.
- Failures/unknowns/pending acceptance: desktop picker flow, large-package UI blocking, fresh-process reopen and full project save/reopen remain pending; import is not backgrounded.
- Next ready task: revision-4 step 31, revision acceptance and compatibility pass.
