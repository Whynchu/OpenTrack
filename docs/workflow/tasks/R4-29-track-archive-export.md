# R4-29 — Specify and export OpenTrack Track Archive v1

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 29; no user decision beyond the roadmap.
- Desired observable outcome: File > Export > Track Archive (OpenTrack)... writes a directory package containing a signed manifest (`format: opentrack-track-archive`, `version: 1`), selected tracks with their owning bus/folder hierarchy, referenced media copied under `Media/`, and explicit reports of external route links and external VST3 module dependencies. This is OpenTrack's own format, explicitly not Cubase Track Archive or OMF. Collisions refuse the export, shared media is copied once, and the package is staged then renamed so a failure leaves nothing published and never modifies the source files.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: `Track` already serializes channel/routing/insert/MIDI state. Media identity is by source path. File dialogs and background workers already exist.
  - User-specified: documented OpenTrack package, no false third-party compatibility claim.
  - Unknown: large-package progress/cancellation UI and import (step 30).
- Dependencies and decisions needed: R4-13–R4-15 kind/hierarchy support and R4-24 arranger data are available; the archive stores full tracks. Arranger sections are included in the project file, not duplicated in the archive manifest in v1 (documented limitation).
- Code scope and architecture boundaries:
  - New `app::track_archive` manifest/report/export; selection expands to owning buses/folders and descendants; media deduplicated by source path.
  - Export runs in a background thread; staging directory under the destination parent; rename only after manifest and all media are written.
  - File > Export entry opens a folder picker and reports external routes/plug-ins in the status message.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. A package writes manifest + copied media; shared media appears once; source files remain.
  2. Selected audio inside a group includes the owning group; external inserts/sends are reported.
  3. An existing destination is refused; a missing media source refuses publication and removes staging.
  4. Package format/version fields are explicit and stable; no third-party format claim is made.
  5. Full app/core/audio suites, inventory and formatting pass.
- Affected regressions: export menu, project data cloning, media file lifetime, background task polling.
- Essential gates: complete-or-nothing publication; source preservation; explicit unsupported/external report.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `a1cdf57`; commit recorded in Git history.
- Changes and reasons: added the v1 manifest/report/export with hierarchy expansion, media deduplication, staging/rename publication, external route/plug-in reporting and a background export task; added the File > Export entry.
- Commands/checks and actual results: archive tests 2 passed; full app suite 543 passed, 9 ignored; inventory 275 leaves with 83 unavailable. Core/audio suites unchanged from 164/171.
- Runtime environment and evidence paths: Windows x64 host; headless filesystem only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; hierarchy expansion now includes owning buses/folders as well as descendants. Cancellation/progress is limited to the status line; step 30 import will validate the manifest.
- Failures/unknowns/pending acceptance: desktop folder-picker flow, large-file progress/cancellation, MIDI/plug-in state import and round-trip come in step 30.
- Next ready task: revision-4 step 30, enable Import Track Archive with a real round trip.
