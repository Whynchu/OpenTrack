# Read-only score view

- Status: accepted for source and automated checks; desktop review pending
- Scope: user-approved score work, first bounded slice (2026-11-03)
- Desired observable outcome: MIDI > Open Score Editor (Ctrl+R) opens a read-only grand staff for the selected MIDI part, placing notes by pitch and length from the project's time signature, and says engraving/editing is not implemented.
- Reference evidence: `staff_for_pitch`, `staff_position`, `pitch_label`, `bar_beats`, `note_head`, `note_flags`, `open_score_view`, `show_score_view` and the tests in `crates/opentrack-app/src/app/score_view.rs`.
- Dependencies: none.

## Scope

- **Grand staff:** treble for pitches at or above middle C (60), bass below; bottom lines E4 and G2; ledger lines outside the staff; `#` before accidentals (no key signature).
- **Time from the signature:** bar length is `beats_per_bar * 4 / beat_unit` quarter notes (4/4 = 4, 6/8 = 3, 3/4 = 3); bar lines are drawn across both staves.
- **Lengths:** whole/half/quarter/eighth/sixteenth by duration thresholds, hollow heads for whole/half, stems and 1–3 flags, filled heads otherwise.
- The window is horizontally scrollable for long parts, states the part name, signature, bar count and note count, and repeats that it is read-only.
- `edit.open_score_editor` (Ctrl+R) dispatches to the same view; the row is enabled when a MIDI part is selected.
- All other Scores menu rows (engraving, layout, symbols) stay unavailable until a score engine exists.

## Verification

- `cargo test -p opentrack score_view` -> 4 passed: signature bar lengths and note-shape/flags, treble/bass pitch mapping with ledger positions and pitch labels, the open-only-with-a-part rule, and the window drawing Treble/Bass/`3 note(s)`/read-only text.
- Inventory regenerated: 277 leaves, 58 available, **176 state-dependent, 35 unavailable**, 8 informational (Open Score Editor was unavailable).
- Keyboard coverage 150 of 152 (video and external sync remain).
- App suite 628 passed / 4 known device-dependent roll failures / 9 ignored (641 tests).

## Not claimed

No engraving, beaming, key signatures, rests, chords/durations beyond simple note heads, or editing. The score is a readout for review, not a notation engine.
