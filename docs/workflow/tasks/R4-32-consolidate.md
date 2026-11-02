# R4-32 — Consolidate and prepare the next queue

- Status: review_pending (source/headless/build); revision closed for this run
- Scope version and requirement IDs: roadmap revision 4, step 32.
- Desired observable outcome: the final source inventory is regenerated, README/current/task packets agree with the delivered and pending work, module boundaries and `app.rs` growth are measured, and the next queue is stated without appending unlimited post-close slices.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Final inventory: 275 project-open leaves, 40 available, 145 state-dependent, 82 unavailable, 8 informational; 18 shell leaves; 147/152 bound command IDs dispatched.
  - Measured size: `app.rs` 31,512 lines at `12a01e0` → 34,262 (+2,750); focused app modules ≈2,742 lines; plus `opentrack-core::arranger` and `opentrack-audio::arranger`. `git diff 12a01e0..HEAD` totals 69 files, 10,242 insertions, 197 deletions (docs/tests included).
  - Suites: core 164 passed; audio 171 passed, 3 ignored; app 544 passed, 9 ignored; fmt/diff clean. x64 local-ASIO build: `builds/x64/OpenTrack.exe`, SHA-256 `7206DC24A44620E59D57828E6C6CBC7EEB0C0B849DAA5545AA13B434AF23A826`.
  - No desktop, listening, hardware or user acceptance; no independent reviewer.
- Dependencies and decisions needed: runtime acceptance of revision 4 is the next gate.
- Code scope and architecture boundaries: documentation/ledger consolidation only; no product edits in step 32.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Inventory regenerated from final source; README/current/handoff/counts agree.
  2. Delivered, partial, blocked and runtime-pending work listed separately.
  3. Remaining gaps name required fixtures/devices/design decisions.
  4. Runner handoff and handoff folder updated; revision closed without added post-close slices.
- Affected regressions: documentation accuracy only.
- Essential gates: no claim of runtime acceptance; no false compatibility claim.
- Review owner and milestone stop: user review; self-review only.

## Delivered

Steps 1–31: inventory, media review, command boundary, pre/post-roll, MIDI Reset, drum map/setup/editor, logical presets, CC lane setup, FX identity/sends/UI/review, folder model/commands/folding/cross-view, optional marker lane, regions editor, secondary ruler, arranger contract/editor/offline render/preview/Add Arranger, Track Archive v1 export/import, and the acceptance/build pass.

## Partial / limited

- Roll stop is per-frame and can overshoot inside a block.
- Track Archive import is synchronous and can block a UI frame on large packages.
- Arranger offline rendering refuses clip fades/source envelopes rather than flattening them; delay/reverb tails use the existing offline graph with hard section cuts.
- Track Archive v1 is OpenTrack-only; arranger sections are stored in the project file, not duplicated in the archive manifest.

## Blocked

- CPR/OMF compatibility: no real named-version fixtures.
- External sync/MMC, retrospective record, video, Control Room and offline plug-in stack: design/device decisions absent.
- Tempo/signature: one beat/sample conversion contract needed across recording, automation, DSP, MIDI export and rendering.
- Score: separate engraving/layout product decision.
- Linux VST3 editor path and hardware ASIO: platform/device validation pending.

## Runtime-pending

All desktop interaction/layout, listening, hardware ASIO, third-party VST3 compatibility and user acceptance. Concrete listening procedure for FX is in R4-16; arranger checks are in R4-27/R4-28.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). Self-review only.
- Source identity: commit containing this packet; OPR 37. Build hash as above.
- Changes and reasons: regenerated inventory, consolidated current.md into the delivery handoff, updated README counts/status, measured module growth, and closed revision 4.
- Commands/checks and actual results: `UPDATE_SOURCE_INVENTORY=1 cargo test -p opentrack generated_source_inventory` → 275 leaves; `cargo fmt --all -- --check`, `git diff --check`, core 164, audio 171/3 ignored, app 544/9 ignored.
- Runtime environment and evidence paths: Windows x64 host; `builds/x64/OpenTrack.exe`; no desktop launch.
- Ledger IDs updated; report command/result: `docs/workflow/current.md` is now concise and links `docs/workflow/handoffs/revision-4-delivery.md`.
- Review findings, rework, elapsed effort if available: self-review only. Final counts differ slightly from the step-31 packet because the Import Track Archive disabled row was enabled in step 30 (82 unavailable).
- Failures/unknowns/pending acceptance: runtime/listening/hardware/user acceptance; the build script metadata hash step needs a shell with `Get-FileHash` on a normal rebuild.
- Next ready task: user runtime review, then a new documented queue (tempo/signature, offline plug-in processing or archive background import); do not extend revision 4.
