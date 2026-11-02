# R4-04 — Bounded pre/post-roll transport contract

- Status: review_pending (partial: stop granularity is the UI frame, not sample-exact)
- Scope version and requirement IDs: roadmap revision 4, step 4
- Desired observable outcome: pre-roll plays from before a target and stops at it; post-roll starts at a target and stops after it. Lengths are persisted independent application preferences (0-64 beats, default 4). Missing selections, recording, active Cycle and zero lengths are refused with a reason. Manual stop/seek/project close cancels the scheduled target.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: existing `play_until` + `advance_play_until` provide the app's stop scheduling; `seek_to_beats` is the shared seek path; `selected_event_span` (step 3) resolves the selection boundary.
  - User-specified by the roadmap: nonnegative beat lengths 0-64 default 4; pre ends at target, post starts at target; clamp at project start; use ordinary engine transport; refuse recording/cycle until specified.
  - Unknown: whether a sample-accurate engine stop-at-frame is required; that is a timing decision not made by this packet.
- Dependencies and decisions needed: step 3 selection span (done). A sample-accurate stop that truncates a block requires an architecture/timing decision and is not implemented; the current stop is evaluated by the existing per-frame transport check.
- Code scope and architecture boundaries: `transport_commands.rs` owns roll lengths, plans and refusals; app composes them with transport seek/play and the existing `play_until`. Preferences are app settings, not project data, so no schema/OPR bump and no undo step.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Pure tests: independent pre/post values, 0/64 clamp, non-finite fallback, default 4, pre-roll clamp at 0, pre-roll at beat 0 refused, post-roll from both edges, zero roll/selection refused.
  2. App test: missing selection, active Cycle, zero length refuse without changing transport; a valid post-roll starts and sets a stop target; stop and seek clear it.
  3. App test: a target inside two adjacent positions does not stop early; the first check at/after the target stops transport and clears the target.
  4. Full app suite and formatting/diff checks pass. Runtime listening/desktop checks pending.
- Affected regressions: play-until commands, play-from/until selection edges, locate-selection, transport stop/start shortcuts, project close.
- Essential gates: transport changes create no project undo step; persisted lengths are app preferences; stop/cancel is observable; recording is refused before any playback starts.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `7d1782d` plus this slice; commit recorded in Git history.
- Changes and reasons:
  - `RollLengths` + `plan_roll` in `transport_commands.rs`; persisted under `transport-pre-roll-beats` / `transport-post-roll-beats`, clamped on load.
  - `schedule_roll(post, edge_end)` resolves the selection, refuses recording/Cycle/missing/zero, seeks to the plan start, starts ordinary transport, and sets the existing `play_until` to the plan stop.
  - Manual seek, Return to Zero, plain Play/Start-Stop-start and Stop clear a scheduled target; project close clears it through `set_transport_playing(false)`.
  - No menu entry is enabled in this step; steps 5-6 wire the four entries.
- Commands/checks and actual results: pure transport tests 8 passed; app roll tests 2 passed; full app suite 508 passed, 9 ignored, 0 failed; formatting/diff checks pass.
- Runtime environment and evidence paths: Windows x64 host; headless app fixtures. No desktop session.
- Review findings, rework, elapsed effort if available: the first app test assumed no audio engine; the fixture starts one, so the valid-roll branch is tested and then stopped. A separate fact: `is_recording` is session-based, so the recording refusal is source-verified but not exercised in the headless fixture. Self-review only.
- Failures/unknowns/pending acceptance:
  - The stop target is checked per UI frame (the existing transport mechanism). A target inside an audio block may overshoot audible audio by up to the block/frame interval. Sample-accurate stop needs a timing decision; do not claim it.
  - Live listening, device behaviour and focus are pending steps 5/6 and desktop review.
- Next ready task: revision-4 step 5, enable Pre-roll to Selection Start/End with labelled length controls.
