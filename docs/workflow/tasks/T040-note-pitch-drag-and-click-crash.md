# T040 - Pitch-only body dragging and note-click crash

- Status: review_pending. Actual implementer/reviewer: GPT-6.1 Sol; no delegated or independent review.
- User direction: note bodies move vertically only and snap their absolute center to the destination semitone. Shift+left-drag fine-tunes pitch continuously within the note; timing is stretched using edges. Clicking a note reportedly closes OpenTrack.
- Baseline build: x64 local ASIO `DF900FE49661DB072FC7A42CA1E1EBD43A5501C849CE720D8CEA835780A2BBDF`.

## Crash evidence and correction

The actual local panic log records `f64::clamp`: `min = 1.7763568394002505e-15, max = 0.0`, with the message `min > max, or either was NaN`. This matches the body-drag horizontal constraint between adjacent notes with a rounding-sized overlap. A body press entered that code even with zero pointer motion. The backtrace is unsymbolized; the exact numeric failure is reproduced by the new touching-note fixture.

Body movement no longer computes or clamps a horizontal displacement. Edge time bounds use a finite, ordered-range guard; infeasible bounds preserve the existing edge. Inspector time fields disable infeasible ranges and do not automatically clamp existing values just because a note was selected. No panic-catching wrapper masks the failure.

## Interaction changes

- A plain click selects only; zero motion does not change pitch or timing.
- Normal body dragging changes pitch only, quantizing `detected_midi + shift + vertical_delta` to an integer MIDI center. The offset alone is not rounded, so an initially detuned note lands on the actual note center. The existing +/-12 semitone offset limit is preserved by choosing an allowed integer destination.
- Shift is captured on primary press and enables ten-times-finer continuous pitch movement (10 cents per displayed semitone row), without center snapping or time changes. Shift targets the clicked note; Ctrl retains multi-selection. Shift also suppresses contextual cutting and forces pitch mode near an edge, avoiding accidental splits/stretches during fine adjustment.
- Plain left/right edge gestures continue stretching timing using the independent editor grid. The edge target is capped at 20% of the note's width so short notes retain a usable pitch body. Handles use real note bounds rather than a viewport-clipped boundary. Captured dragging continues when a note moves out of the visible pitch range.
- Live audio processing, one-gesture Project Undo/Redo and retained analysis use T039's existing path. No new audio processor, dependency or project schema change.

## Checks and acceptance

Regression fixtures cover the logged epsilon overlap, unchanged click selection, detuned notes snapping to absolute center, ignored horizontal body motion, Shift fine pitch on a previously multi-selected group, unchanged source/destination timing, guarded inverted time bounds, edge stretching and note-only splitting. The full editor regression now checks an integer destination center rather than the old relative integer offset. Final workspace/build identity is recorded in current.md.

No OpenTrack/Cubase desktop session or hardware listening check was performed. OpenTrack remains closed and the saved EXAMPLE files are untouched. User should check single clicks, normal vertical body movement, Shift+left fine movement, edge-only timing, live audio and Undo. Overall vocal quality, exact fine sensitivity and visual acceptance remain pending. Stop at this review checkpoint.

Final checks: 191 release workspace tests pass (117 app,58 audio,16 core); format/diff/ledger and x64 local-ASIO build pass. SHA-256: `DED1D93FB9389A9F4641355DA9F43CF1D40722DE727E8471CDE99FD82ABF159D`. OpenTrack remains closed for user review.
