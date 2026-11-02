# T082 — Typed, validated channel routing

- Status: review_pending (source + automated checks complete; runtime review open)
- Scope version and requirement IDs: roadmap steps 22 and 16 (remainder of typed
  buses is explicitly out).
- Desired observable outcome: a channel's input and output are values with a known
  set of destinations; a routing menu can only offer what the engine can play; a
  project that stores an unplayable route says so instead of silently playing
  through the stereo master; a mono track records from a named physical input
  channel.
- Reference evidence: user hardware verification of basic recording (Mackie,
  laptop built-ins, 2026-11-01). The capture and output buses are stereo end to
  end, which bounds the vocabulary.
- Dependencies and decisions needed: none for the stereo vocabulary. Non-stereo
  buses need an audio-engine change with a listening gate.
- Code scope and architecture boundaries:
  - `core/src/routing.rs` — `InputRoute`, `OutputRoute`, `Speaker`,
    `output_destinations`, `input_destinations`, `MAX_INPUT_CHANNELS`.
  - `core/src/project.rs` — `Project::validate_routing` (detailed errors).
  - `app/src/routing.rs` — `playback_output_route`, `playback_input_channel`,
    `input_route`, `output_route`; the send rack and cycle check now parse here.
  - `app/src/channel_settings.rs` — both routing menus generated from the model.
  - `app/src/app.rs` — Add Track input picker, load-time routing report.
- Acceptance checks (automated today):
  1. Persisted spellings round-trip without migrating projects, including the
     historical mono labels.
  2. Unknown routing is reported, not treated as the master.
  3. Mono inputs validate against the capture bus width.
  4. Speaker assignment covers only the stereo output; groups expose their id.
  5. Destination lists match what validation accepts, per channel count.
  6. Labels name the physical channel.
  7. Validation accepts every route the menus offer.
  8. Validation reports a missing group (output and send), an unknown route, an
     over-wide mono input, self-routing and a feedback loop.
  9. The playback plan resolves an unresolvable route to `Disconnected`.
  10. The monitor tap follows the capture channels; an unsupported channel gets none.
  11. Opening a project with an unknown route reports it in the status message.
- Affected regressions: send rack, group cycle prevention, live monitoring,
  playback plan, project load/save.
- Essential gates: persistence (spellings unchanged), validation on save, honest
  failure instead of silent mis-routing.
- Review owner and milestone stop: user, at the next desktop session.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash; user review pending.
- Source identity: commit `f040f4c`; release `builds/x64/OpenTrack` `b118880c…`.
- Changes and reasons: routing was three independent string parsers; an unknown
  route became `Stereo`, so a wrong route sounded like a correct one. One typed
  vocabulary removes the duplication and makes validation possible.
- Commands/checks and actual results: `cargo test -p opentrack-core routing` 6
  passed; `cargo test -p opentrack-core routing_validation` 4 passed;
  `cargo test -p opentrack routing_model` 3 passed.
- Runtime environment and evidence paths: Linux/XWayland build; no desktop review.
- Review findings, rework: reading a project must stay permissive (a hand-edited
  file should still open so it can be repaired); only saving refuses. This was
  found by a test and corrected during the slice.
- Failures/unknowns/pending acceptance: physical inputs beyond 2 channels and
  speaker layouts wider than stereo are not implemented; the model expresses them
  and validation rejects them with a message.
- Next ready task: T083.
