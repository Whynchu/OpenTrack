# R5-09 — Transpose dialog

- Status: accepted (already delivered before revision 5; verified rather than rebuilt)
- Scope version and requirement IDs: revision 5, step 9
- Desired observable outcome: a bounded transpose UI over the existing MIDI transform, with selection scope, one labelled history step and honest refusals.
- Reference evidence (IDs/paths): `crates/opentrack-app/src/app.rs` (`transpose_selected_notes`, `show_midi_transform_dialog`, Edit > Transpose / Velocity…), `crates/opentrack-core/src/midi_transform.rs` (`transpose_notes`, `MAX_TRANSPOSE_SEMITONES`).
- Dependencies and decisions needed: none. Scope decision below.
- Code scope and architecture boundaries: no new code; the dialog already applies the transform through the one existing command.
- Acceptance checks: existing tests at `app.rs` cover +12, 0 and −1 semitones; the dialog clamps to `MAX_TRANSPOSE_SEMITONES`, states the selection size, refuses with a reason when no part is selected, and only writes history when `transpose_notes` reports a change.
- Affected regressions: none.
- Essential gates: undo (one step), no-op writes nothing, out-of-range refusal.
- Review owner and milestone stop: self-review.

## Scope decision

- `Project > Add Track > Transpose` names a Cubase **transpose track**, which is a different feature from this dialog. It stays unavailable and is not pointed at the dialog, because that would misdescribe the menu row. Step 22 records that mapping decision.
- No second transpose UI is added; the existing "Transpose / Velocity..." dialog is the single path.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: `df39448` verification pass.
- Changes and reasons: none; evidence recorded so the step is not counted as an unimplemented gap.
- Commands/checks and actual results: `cargo test -p opentrack transpose` → the existing transpose/velocity tests pass (part of the 554-passing suite at this checkpoint).
- Runtime environment and evidence paths: Kubuntu Linux; dialog layout review pending on the desktop.
- Ledger IDs updated: none.
- Review findings, rework: the packet's assumption that a dialog was missing was wrong; it already exists, so only the Add Track mapping needed a decision.
- Failures/unknowns/pending acceptance: desktop review of the dialog wording.
- Next ready task: R5-10.
