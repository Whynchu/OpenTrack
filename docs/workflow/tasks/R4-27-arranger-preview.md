# R4-27 — Connect audible arranger preview

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 27; user decision (preview playback-only; recording/punch/cycle unavailable during preview).
- Desired observable outcome: Project > Preview Arranger toggles a live sequence preview built from the same immutable sequence transformation as the offline renderer. The transport plays the sequence through the existing engine plan; the toolbar shows sequence and source positions. Preview refuses active recording and Cycle. Stopping, rapid stop, seek/stop, project content edits or project close restore the ordinary timeline plan and stop cleanly so no MIDI notes hang.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: the app already publishes engine plans from `PlaybackTrackSettings`, and R4-26 builds sequenced tracks with exact boundaries. `set_transport_playing(false)` is the shared stop path; project edits call `mark_modified`.
  - User-specified: playback-only first version; no recording/punch/conflicting cycle.
  - Unknown: live listening and block-boundary behavior; no audio device was opened in this headless run.
- Dependencies and decisions needed: R4-24–R4-26 delivered. Preview uses the current offline playback snapshot, so it requires a prepared plan and an open engine.
- Code scope and architecture boundaries:
  - App preview state maps sections/repeats to engine sequence beats; engine publication uses the existing `set_playback_plan_with_hosted_output` path with no loop range.
  - Project > Preview Arranger toggles start/stop; toolbar displays `seq` and `source` beats. Stop restores the ordinary plan via `rebuild_audio_plan`.
  - `mark_modified` stops preview before content changes take effect. Project reset clears preview. Cycle/recording are refused.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Preview refuses Cycle and an empty sequence with reasons; refuses recording by the same rule.
  2. Source/sequence position mapping follows section order and repeats.
  3. Stop uses the shared transport stop path, restores ordinary plan/position and leaves no preview state.
  4. Content edits stop preview; sequence rendering/offline length agreement remains covered by R4-26 tests.
  5. Full app/audio/core suites and formatting pass; listening remains pending.
- Affected regressions: transport stop path, playback plan publication, project edits, toolbar layout.
- Essential gates: playback-only; clean restore; no hanging MIDI state.
- Review owner and milestone stop: user review; self-review only. Audible behavior is not claimed.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `3fc06e7`; commit recorded in Git history.
- Changes and reasons: added preview state, start/stop, source/sequence mapping, engine plan publication through the existing path, toolbar position readout, cycle/recording refusal, stop-path restore, and content-edit cancellation.
- Commands/checks and actual results: focused preview/refusal/position test passed; full app suite 540 passed, 9 ignored; audio 171/3 ignored and core 164 from the preceding steps; inventory 274 leaves with 84 unavailable.
- Runtime environment and evidence paths: Windows x64 host; no audio device/desktop session opened. No listening evidence.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; no live block-boundary, seek-between-items, rapid-stop or MIDI-hang check was possible without an engine.
- Failures/unknowns/pending acceptance: audible preview, rapid stop, seeking between sequence items, project close during preview and MIDI hanging remain runtime-pending. Add Arranger UI enablement is step 28.
- Next ready task: revision-4 step 28, enable Add Arranger and review the complete path.
