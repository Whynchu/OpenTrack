# R5-10 — Retrospective capture contract

- Status: accepted
- Scope version and requirement IDs: revision 5, step 10
- Desired observable outcome: a bounded, in-memory window of recent MIDI input that can be turned into a part, with documented limits and no disk writes.
- Reference evidence (IDs/paths): `crates/opentrack-app/src/app/retrospective.rs` (new); `handle_midi_input_events` in `app.rs`.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: new focused module; `app.rs` pushes incoming events and dispatches the command.
- Acceptance checks: buffer caps at 8,192 events and 60 seconds; pairing is per key and channel; a held key becomes a short note instead of disappearing; the buffer is cleared on project close and after use.
- Affected regressions: none; note that `handle_midi_input_events` now iterates events once more per frame, which is bounded by the input queue.
- Essential gates: memory bound, no persistence, no fabricated notes.
- Review owner and milestone stop: self-review.

## Contract

- Window: 60 seconds or 8,192 events, whichever binds first. Events older than the window are dropped as new ones arrive.
- Timestamping uses the same arrival-age correction as MIDI recording, so captured beats line up with the transport position.
- Unclosed keys get a minimum 0.25-beat note ending at the last captured beat.
- Nothing is written to disk; the buffer is dropped when a project closes.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet and R5-11.
- Changes and reasons: `Capture::push/clear/notes` plus `capture_retrospective_event`.
- Commands/checks and actual results: `cargo test -p opentrack retrospective` → 3 passed.
- Runtime environment: Kubuntu Linux; no MIDI hardware attached, so capture is exercised with synthetic events only.
- Ledger IDs updated: none.
- Review findings: none.
- Failures/unknowns/pending acceptance: a real controller run is needed to confirm arrival-age correction under device timing jitter.
- Next ready task: R5-11.
