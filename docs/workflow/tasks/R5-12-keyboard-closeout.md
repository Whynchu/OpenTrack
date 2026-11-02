# R5-12 — Virtual keyboard review close-out

- Status: accepted for source and headless checks; desktop/listening pending
- Scope version and requirement IDs: revision 5, step 12
- Desired observable outcome: the Alt+K keyboard releases everything it starts, never leaves the live-MIDI monitor enabled after closing, and interacts correctly with MIDI Reset.
- Reference evidence (IDs/paths): `virtual_keyboard_*` in `app.rs`; `reset_midi_destination`; `MIDI Reset` report strings.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: one regression test; no behaviour change needed.
- Acceptance checks: MIDI Reset clears held keys and reports the missing device; closing releases held keys and clears the open flag; monitor gating already disables only when no hardware stream exists.
- Affected regressions: none.
- Essential gates: no stuck on-screen notes, no monitor left enabled, project close clears held keys.
- Review owner and milestone stop: self-review; audible result stays with the user.

## Findings

- `reset_midi_destination` already clears `virtual_keyboard_held` and sends note-offs before the reset controllers, and `close_virtual_keyboard` releases held keys and disables the monitor only when no hardware MIDI stream exists, so real input is not affected.
- The missing piece was evidence: there was no test for either interaction. `midi_reset_and_close_release_held_keys` now covers both.
- Audible verification (does a note sound through an armed instrument track, and does it stop) needs a device and is recorded as pending, not passed.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: added the interaction test; no product code changed.
- Commands/checks and actual results: `cargo test -p opentrack midi_reset_and_close` → passed; full app suite at this checkpoint 558 passing with the four known device failures.
- Runtime environment: Kubuntu Linux, no audio device in the harness.
- Ledger IDs updated: none.
- Review findings: none beyond the missing test.
- Failures/unknowns/pending acceptance: desktop and listening review of the keyboard, monitor gating and MIDI Reset.
- Next ready task: R5-13.
