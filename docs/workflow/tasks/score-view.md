# Read-only score view

- Status: accepted for source and automated checks; desktop review pending
- Scope: user-approved score work, first bounded slice (2026-11-03)
- Desired observable outcome: MIDI > Open Score Editor (Ctrl+R) opens a grand staff for the selected MIDI part, places notes by pitch and length from the time signature, and supports selecting, moving, transposing and deleting notes, page layout and zoom.
- Reference evidence: `staff_for_pitch`, `staff_position`, `pitch_label`, `bar_beats`, `note_head`, `note_flags`, `open_score_view`, `show_score_view` and the tests in `crates/opentrack-app/src/app/score_view.rs`.
- Dependencies: none.

## Scope

- **Grand staff:** treble for pitches at or above middle C (60), bass below; bottom lines E4 and G2; ledger lines outside the staff; `#` before accidentals (no key signature).
- **Time from the signature:** bar length is `beats_per_bar * 4 / beat_unit` quarter notes (4/4 = 4, 6/8 = 3, 3/4 = 3); bar lines are drawn across both staves.
- **Lengths:** whole/half/quarter/eighth/sixteenth by duration thresholds, hollow heads for whole/half, stems and 1–3 flags, filled heads otherwise.
- **Editing:** click a note to select it (the MIDI editor selection follows). Buttons and keys move it by 1/4 beat (Shift: 1 beat), transpose ±1 semitone (Shift: ±12) or delete it. Move is one "Move Notes" undo step clamped inside the part; transpose/delete reuse the MIDI commands and their undo labels.
- **Page Mode and Settings:** the Scores menu rows toggle four-bar systems (`score-page-mode-v1`) and open a zoom slider (`score-zoom-v1`, 0.5–2.0). Both are view settings, not project data.
- The window is horizontally scrollable for long parts, states the part name, signature, bar count and note count, and repeats that it is read-only.
- `edit.open_score_editor` (Ctrl+R) dispatches to the same view; the row is enabled when a MIDI part is selected.
- All other Scores menu rows (symbols, slurs, grace notes, alignment, layout) stay unavailable until an engraving engine exists.

## Verification

- `cargo test -p opentrack score_view` -> 6 passed: signature/page mapping, pitch/staff mapping, selection plus move-clamp/transpose/delete with their undo labels, page mode and settings toggles, the open-only-with-a-part rule, and the window drawing Treble/Bass/`3 note(s)`/Page Mode.
- Inventory regenerated: 277 leaves, **60 available**, 176 state-dependent, **33 unavailable**, 8 informational (Page Mode and Settings moved out of the disabled list).
- Keyboard coverage 150 of 152 (video and external sync remain).
- App suite 630 passed / 4 known device-dependent roll failures / 9 ignored (643 tests).

## Not claimed

No engraving, beaming, key signatures, rests, chords/spanners or symbols. Moves are clamped inside the part and snap to fixed 1/4 or 1-beat steps; there is no drag editing.
