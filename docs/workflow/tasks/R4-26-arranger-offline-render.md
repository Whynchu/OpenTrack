# R4-26 — Render an arranger sequence offline

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 26; user decision (no destructive rearrangement; preview playback-only).
- Desired observable outcome: an immutable arranger sequence snapshot can be prepared off the callback from the existing playback settings. Each section's audio clips, MIDI notes/controller events and track-volume automation are copied into sequence order with source offsets, clipped to half-open section boundaries; repeats expand exactly. Unsupported clip fades/envelopes are refused rather than flattened. The constant-tempo first version renders through the existing `OfflineRenderer`.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: `PlaybackTrackSettings` contains clips, MIDI notes/controllers, volume automation and the existing insert/send graph; `OfflineRenderer` renders a beat range with exact frame boundaries. Core arranger validation supplies ordered sections/repeats.
  - User-specified: source clips are untouched; unsupported constructs must be reported.
  - Unknown: tail/discontinuity policy for delay/reverb inserts across boundaries; the first version relies on the existing offline graph and hard section cuts.
- Dependencies and decisions needed: R4-24/R4-25 delivered. Constant tempo only.
- Code scope and architecture boundaries:
  - New `opentrack-audio::arranger` with `ArrangerSequenceSegment`, `arrange_sequence` and `ArrangerSequenceError`. It clones settings and rebuilds clips/MIDI/automation at sequence positions; clips with fades/source envelopes are refused. Total length is bounded (one hour at 60 BPM).
  - No source mutation, no new DSP path; the result feeds the existing `OfflineRenderer`.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Repeats produce exact sequence length and repeated offsets; section-local source offsets/durations are correct.
  2. MIDI notes crossing a boundary are clipped; controllers outside the section are omitted and repeated inside.
  3. Volume automation points are remapped per repeat with a boundary value; generated-signal offline render is exact length and level.
  4. Empty/invalid sequences, over-length sequences and fades/envelopes are refused with typed errors.
  5. No source clip/settings are changed; full audio/app/core suites and formatting pass.
- Affected regressions: offline render length, audio/MIDI scheduling, automation, future preview.
- Essential gates: immutable sequencer input; exact boundaries; explicit refusals.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `47502a6`; commit recorded in Git history.
- Changes and reasons: added sequence transformation and typed refusals in `opentrack-audio::arranger`, with source offsets, boundary clipping, MIDI/controller/automation remapping and a bounded total length; it feeds the existing offline renderer without a second DSP path.
- Commands/checks and actual results: arranger sequence tests 3 passed; audio suite 171 passed, 3 ignored; core/app suites unaffected from 164/538. Formatting/diff checks run.
- Runtime environment and evidence paths: Windows x64 host; generated constant sources only. No listening.
- Ledger IDs updated; report command/result: README/current.md updated.
- Review findings, rework, elapsed effort if available: self-review only; test sources were initially too short for the full repeated render and were lengthened.
- Failures/unknowns/pending acceptance: audible preview and sequence UI integration (steps 27–28); insert tail/reset at section boundaries remains defined as a hard cut by construction but needs listening review.
- Next ready task: revision-4 step 27, connect audible arranger preview.
