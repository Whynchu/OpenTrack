# T006 — Corner resizing and visible hand cursors

User feedback: corner extension worked well. The custom held hand was rejected; the user prefers the original native four-way cursor while held. T007 implements that preference and retains the resize interaction. This is functional user feedback, not full reference/visual acceptance of every T006 case.

Status: review_pending. Actual implementer: GPT-6.1 Sol. Authority: user requested corner-triangle extension handles, a Windows hand where available, a held/grabbing state, and middle-button hand panning from the ruler/edit area. The user closed the running app for the build. The app was not opened during this slice.

Scope: transport-v3 adds range-resize (common weight 2) to the existing ranges workflow. All existing requirement weights are unchanged. Other affected IDs: range-move, loop-range, skip-range, middle-pan, ruler-regressions.

## Cursor finding and correction

Installed egui-winit 0.36.2 forwards Grab/Grabbing to winit 0.30.13; its Windows backend maps both to `IDC_SIZEALL`, the four-way arrow. This explains why the prior source-level Grab/Grabbing assertions did not guarantee an actual hand on Windows. The backend does expose `IDC_HAND` through PointingHand but no distinct closed-hand cursor.

The current implementation uses that native Windows pointing hand for range-body hover. While held, it hides the native cursor and draws an original DPI-scaled white fist with black outline. Range movement and middle-button panning share that helper. Other platforms retain their native Grab/Grabbing mappings. No new dependencies, platform cursor handles or unsafe code were introduced. Exact cursor appearance still needs the user's Windows check.

## Handle and pan behavior

- The top-strip corner hit targets take precedence over range-body movement. Hover/drag uses the native horizontal resize cursor. Taking hold preserves the pointer offset, so grabbing inside a triangle does not jump its boundary.
- Each corner moves only its own chronological boundary. The opposite boundary and Loop/Skip mode stay fixed. Snap applies to the moved edge; Snap off permits fractional positions. At crossing, the moving edge stops before the fixed edge rather than flipping mode or creating an empty range. At project limits it clamps. Handles shrink on narrow ranges to leave a range-body move target.
- Preview remains transient; release commits through the shared core range setter and existing Undo/dirty/audio-plan path. A stationary click is not an edit. Escape/capture/edge-scroll behavior is retained.
- Middle panning can start in the ruler or track canvas. Initial press does not apply the previous pointer delta, avoiding a jump. While held it shows the same closed hand, captures outside the starting area, pans both axes, and stops on release. It does not modify project content or seek the playhead. The cursor draws after track controls so their hover state cannot replace the held hand.

These resize limits follow a documented implementation choice; exact reference behavior when crossing locators remains unmeasured. No schema, audio processing, Mixer or new tooltip changes were made.

## Checks and identity

- Existing and final x64 release builds passed with OpenTrack closed. No app instance launched; EXAMPLE untouched.
- Eight actual headless ruler-handler tests passed, including both handles for Loop/Skip, fixed opposite boundary, grab offset, preview/commit/Undo, correct resize cursor, middle panning from both target areas, no initial jump, release stopping, and existing move/draw/mixed seek/zoom regressions.
- Eleven timeline tests passed, including snapped/free resize limits and direction preservation. Formatting/diff checks passed. The prior unrelated full-suite theme failure remains unchanged; this slice did not rerun that full suite.
- EXE SHA-256: `C2705180F527BC846EB34E11931C5BA89920DD9A32D27DB4BA66DA7A68D1B3A7`.
- `app.rs`: `1D3EFAE2B2209AD13FC09DECD7E91AB31226BEE9D62B8225A19E2B80E6B7D434`.
- `timeline.rs`: `D488618134F8EF6C8797B58E258B2689F60BEEF7C25986F32A83F3735F1DD611`.
- Base commit `cbd9835` plus existing uncommitted transport work. No live screenshot, cursor visual acceptance or reference parity is claimed from compilation/headless tests.

## User check

1. Hover the range interior, then hold/drag it. Check the native hand and closed held-hand appearance, fixed length, and unchanged mode.
2. Hover each corner triangle. Drag left/right to extend or shorten that boundary; check the opposite edge remains fixed. Try Loop and Skip, Snap on/off, release outside the ruler, Escape and Undo/Redo.
3. Hold the middle button first in the ruler and then in the track canvas. Check the closed hand, no initial jump, horizontal/vertical pan, and stopping on release.
4. Recheck mixed seek/zoom, range body movement, loop/skip playback, edge scrolling and pinned headers before the next packet.

Stop for the user's review; exact Windows cursor rendering and live interaction remain pending.
