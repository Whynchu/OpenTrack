# R5-11 — Retrospective Record command

- Status: accepted
- Scope version and requirement IDs: revision 5, step 11
- Desired observable outcome: Transport > Retrospective Record (Shift+NumpadMultiply) turns the captured buffer into a MIDI part on the armed destination as one history step, with clear reasons when it cannot.
- Reference evidence (IDs/paths): `retrospective_record` in `src/app/retrospective.rs`; the Transport menu row and the dispatch arm.
- Dependencies and decisions needed: R5-10 (accepted).
- Code scope and architecture boundaries: one command and one menu row; no change to recording or the MIDI device paths.
- Acceptance checks: empty buffer, no armed track and active recording refuse with a reason; a synthetic two-note capture becomes a part at the captured beats with the "Retrospective Record" label; the buffer is consumed.
- Affected regressions: the transport menu row moves from unavailable to state-dependent; the curated inventory guard now expects three uncovered commands.
- Essential gates: undo, no fabricated notes, buffer cleared.
- Review owner and milestone stop: self-review; hardware capture pending.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: `retrospective_record` places a part covering the captured span on the selected-or-first armed MIDI/Instrument track, copies every captured note with its channel and velocity, and refuses instead of guessing. The Transport row is enabled while a project is open and not recording.
- Commands/checks and actual results: `cargo test -p opentrack retrospective` → 3 passed; shortcut audit 149/152; inventory regenerated (275 leaves, 40 available, 149 state-dependent, 78 unavailable, 8 informational).
- Runtime environment: Kubuntu Linux.
- Ledger IDs updated: README (78 disabled rows, 149/152), accessibility report.
- Review findings: none.
- Failures/unknowns/pending acceptance: real-device capture and the audible result remain pending.
- Next ready task: R5-12.
