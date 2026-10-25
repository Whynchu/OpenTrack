# T005 — Move existing ranges and combine seek/zoom

Status: review_pending. Actual implementer: GPT-6.1 Sol. User explicitly requested both interactions. Current scope is transport-v2: range-move (new, common weight 2), mixed-scrub-zoom, plus seek/range regression obligations. Existing requirement membership/weights are preserved; adding range movement is the reason for the scope increment.

## Behavior

- Hovering the top ruler strip inside the existing range shows an open hand; a captured move shows a closed hand. Outside the range, the monochrome pencil still draws a new directional range.
- Range dragging previews translation without changing project content until release. Both endpoints move together, retaining exact duration, Loop/Skip direction and the original grab offset. Snap rounds the translation to Bar increments; it does not independently round fractional endpoints. At project limits the whole interval clamps without shrinking. A stationary hand click is not an edit. Escape and Undo use the existing range edit path.
- The normal lower ruler no longer chooses an exclusive gesture axis. Horizontal pointer position continuously seeks while vertical displacement changes zoom. Down zooms in, up zooms out, and both remain available after either initial movement direction. The original musical zoom anchor is retained except where the viewport reaches project start.
- Edge scrolling accumulates independently of zoom. Clamping zoom at project start does not become a persistent pan offset. Release uses the updated zoom/scroll transform, then applies the enabled Bar snap.
- No project schema, audio processing, Mixer or tooltip changes were made. Core range commits and audio-plan rebuild still occur on release. The selected range can continue playing at its previous bounds while its new position previews.

This follows the user's stated interactions; exact reference hand shape and gesture sensitivity remain unmeasured. No current screenshot or live acceptance is claimed.

## Checks

- The existing build passed after the user closed the detected OpenTrack process. Final x64 release build passed with no process running. The app was not launched; EXAMPLE was not opened or modified.
- Ten timeline tests passed, including signed translation, fractional duration, direction, snapping, project-start and upper-limit clamps.
- Six headless actual-ruler tests passed: existing draw/seek/snap; hand cursor and captured movement for Loop and Skip; preserved duration/mode/playhead; preview without mutation; Undo; cancellation/stationary click; mixed seek/zoom after either initial direction; anchor restoration after zooming past project start and back.
- Formatting and diff checks passed. The previously documented unrelated theme-test failure was not changed or rerun. Live Windows pointer behavior, paint alignment during movement, listening and visual fidelity remain for the user's review.

## Identity

- Base commit `cbd9835` plus the existing uncommitted transport changes.
- EXE SHA-256: `10348198E8F94CC0EB19518560243DC0131A2E3FADF744F51FAF7F2CAE60E464`.
- `app.rs`: `8180A91FE9C0F302E27F61ACD07354947D27F71E161ECCE96E714AD752BCB479`.
- `timeline.rs`: `D96E0BDC76FF36939A2EBBE3190C78EC97F88D3502B92601780ED01E427710F2`.

## User review

1. Hover inside the highlighted top ruler strip. Drag with the hand left/right; verify both bounds move together, duration stays fixed, and the playhead stays put. Try both Loop and Skip, with Snap on/off; drag back to project start and release outside the ruler.
2. Click without moving; cancel a move with Escape; Undo/Redo a committed move. Draw from outside the existing range to verify the pencil still creates a new range.
3. In the lower ruler, first seek sideways, then move down/up without releasing. Repeat starting vertically, then moving sideways. Verify seeking and zoom remain available together and release snaps to the Bar grid if enabled.
4. Recheck edge holds, pointer/marker alignment, Loop/Skip playback, pinned headers, middle pan and the ruler context menu. Report remaining differences before the next packet.

Stop for the user's check. Their prior functional loop/skip success is preserved in the ledger; these new gestures remain pending runtime acceptance.
