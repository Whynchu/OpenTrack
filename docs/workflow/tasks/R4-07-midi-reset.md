# R4-07 — MIDI Reset

- Status: review_pending (no external MIDI output exists; live destination is the bounded monitor)
- Scope version and requirement IDs: roadmap revision 4, step 7
- Desired observable outcome: MIDI > Reset releases held on-screen notes and sends All Sound Off (CC120), Reset All Controllers (CC121) and All Notes Off (CC123) on all 16 channels through the existing bounded live-MIDI queue. Recorded project notes/CC and other state are untouched. Repeated reset, no device and project close are safe.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified by the roadmap: release held live notes and sustain; include virtual keyboard; send supported All Notes Off/All Sound Off/Reset Controllers through the existing bounded MIDI path; preserve project data.
  - Observed: `AudioEngine::send_live_midi_message` is the only live MIDI destination; its renderer handles CC120/123 by releasing voices on that channel and stores CC121. There is no external MIDI output or device selection.
  - Unknown: real stuck-note behaviour on hardware, because no external destination exists.
- Dependencies and decisions needed: none. Adding external MIDI output is out of scope; the command says so.
- Code scope and architecture boundaries: app-level `reset_midi_destination`, menu wiring for the existing disabled MIDI > Reset row. No project schema, history or audio-callback change.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. No-device test: held keys are cleared, status explains that no reset messages were sent, project data and history are unchanged, repeated reset and project close are safe.
  2. Engine test: held keys are cleared, CC120/121/123 are accepted by the bounded queue (or a full queue is reported), project unchanged.
  3. Generated inventory drops MIDI > Reset from unavailable; full app suite, formatting and diff checks pass.
- Affected regressions: virtual keyboard, MIDI monitor queue, project note/CC data.
- Essential gates: project content untouched; no panic without a device; reset is repeatable; a full queue is reported rather than silently losing a reset.
- Review owner and milestone stop: user review; self-review only; hardware listening pending because no external MIDI output exists.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `4d1545b` plus this slice; commit recorded in Git history.
- Changes and reasons: `reset_midi_destination` sends note-offs for held on-screen keys first, then CC120/121/123 on channels 0-15 through `send_live_midi_message`. MIDI > Reset is now enabled. Status distinguishes no device, full queue and success.
- Commands/checks and actual results: focused reset tests 2 passed; full app suite 514 passed, 9 ignored, 0 failed; inventory now reports 271 leaves with 90 unavailable.
- Runtime environment and evidence paths: Windows x64 host, headless fixtures; no external MIDI destination or device listening.
- Review findings, rework, elapsed effort if available: the command cannot reset an external synthesizer because OpenTrack has no MIDI output; this is recorded rather than implied. Self-review only.
- Failures/unknowns/pending acceptance: hardware stuck-note listening remains impossible until a MIDI output path exists. Live queue/device interaction is unverified.
- Next ready task: revision-4 step 8, the drum-map model.
