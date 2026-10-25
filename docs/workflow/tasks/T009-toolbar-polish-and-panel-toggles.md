# T009 — Toolbar glyphs, enabled text and panel toggles

Status: review_pending. Actual implementer: GPT-6.1 Sol. User authority: brighten implemented Grid/Quantize fields, improve nearby toolbar symbols and repeated power symbols, use a plain black selection arrow, and make Mixer/panel buttons blue only when active/open with close-on-next-click and a next-action tooltip. The user closed OpenTrack for the build.

Transport scope remains transport-v4: enabled grid-control presentation is an existing requirement refinement. Panel toggles and glyph appearance are covered below without adding unrelated items to the transport percentage. Whole-DAW/window-management coverage remains unestablished.

## Changes

- Grid/Quantize text and dropdown arrows use strong foreground ink (dark theme 230 gray, light theme 24 gray). Inert fields keep muted ink. Existing field geometry, menus and actions remain.
- Extracted original vector glyph rendering to `icons.rs`. Power now has a symmetrical 12-o'clock gap, vertical stem and rounded stroke endpoints, scaled for compact switches. Project toolbar and Channel Settings insert/send/EQ powers share it. The effect-header power glyph also shares it; this is an icon-only change, not a Compressor feature batch.
- Object Selection uses a conventional solid black pointer. Scissors, glue bottle, eraser, pencil, magnet, grid-mode readout, zero-crossing wave and color palette are clearer. Remaining toolbar glyph geometry is retained in the shared renderer. Button dimensions/grouping and selected colors are retained; bevel highlights are quieter. No external/copied image assets or new dependencies.
- Mixer formerly passed a hard-coded selected=true and only set open=true. It now reads/toggles `mixer_window.open`, requests focus only when opening, and uses Open Mixer / Close Mixer tooltips. Pool shares the same actual window-state toggle; its open/close state is blue only while open.
- Inspector and Info Line retain their functioning toggles and selected states, with Show/Hide next-action tooltips. Overview/other unimplemented actions were not falsely activated. Cycle retains the user's previously specified purple/orange mode colors. Mixer strip appearance and behavior are untouched.

## Coverage and verification

| Target | Expected action/state | Source | Evidence/status |
| --- | --- | --- | --- |
| Grid / Quantize fields | Clickable with strong normal text and arrow | app.rs toolbar_choice_with_sense | Both themes' emitted text colors checked; popup regressions pass; live pending |
| Object Selection and neighboring glyphs | Black cursor and legible original symbols | icons.rs paint_toolbar | Headless geometry reviewed in both themes; live pending |
| Repeated power glyphs | Top-centered ring gap and stem at compact sizes | icons.rs power; channel_settings.rs power | Headless size samples reviewed; live pending |
| Mixer / Pool buttons | Blue iff open, click again closes; tooltip describes next action | app.rs toolbar_window_toggle | Headless test toggles the actual NativeWindowState open/focus fields; live close/dock review pending |
| Inspector / Info Line | Blue iff shown; Show/Hide tooltip | app.rs Project toolbar | Existing toggles retained; source reviewed; live pending |

- Existing and final x64 builds passed with no OpenTrack process running. No application instance launched; EXAMPLE untouched.
- Two toolbar tests passed: actual Mixer window-state open/close toggling and enabled dropdown text in both themes. Eleven ruler/menu regression tests passed. Formatting and diff checks passed. The previously documented unrelated full-suite theme assertion remains unresolved and was not rerun.
- Initial optional preview export hit the outside-folder write restriction. The harness now clears unused font-atlas updates before assertions/export; cache export succeeded, and the artifacts were copied to the authorized external review folder with reviewed filesystem access. This was a headless export failure, not a running-app failure.
- Optional preview export is controlled by OPENTRACK_ICON_REVIEW_JSON. The sheet is generated from actual egui shape geometry/colors; System.Drawing rasterizes the geometry and approximates Segoe UI font rendering. It is not a desktop screenshot or a reference-parity comparison.

## Review artifact and conditions

Image: `G:\dev\Opentrack\_analysis-tools\reports\opentrack-comparisons\2026-10-24\T009-toolbar-review.png`; manifest: adjacent `T009-toolbar-review-manifest.json`. The image was shown in chat.

- 2800×460 output; synthetic 2× presentation scale; both dark/light themes. Desktop resolution/scaling and window positions are not measured because no window was opened. No live panels were open in this review.
- The glyph row explicitly enables all icons for comparison; it does not claim every represented action is implemented. The bottom row compares inactive/active Mixer and power states and compact power sizes. User instructions are the design reference; exact reference glyph/style parity remains unmeasured.

## Identity

- Base commit `cbd9835` plus existing uncommitted work.
- EXE SHA-256: `630599410BE46E7D4A0911262D5EA26228DCC1CB08D69D6E1714253062EC6725`.
- app.rs: `E598F44695F2C68FCC9F67DD38C07F0A65ACFD6381DE089D8435CCED8CD0BA55`.
- icons.rs: `B80887A6B7EFE32A6C3895772470604CBD21C344DD95B87DD9213CEB91C1292B`.
- channel_settings.rs: `3E740B2C0A1FF01B01B71401172171F4BD56C6626698772008F5C7791659D75F`.
- main.rs: `D66B43B1377F5A874C03EE9E48021233829EE7ABCE1B3B3CB6B54E29438DE73E`.

## User check

1. Check the active Grid/Quantize captions against the muted Touch field; open both menus and verify snap behavior remains correct.
2. Inspect the black object pointer, adjacent tool symbols and power symbols at normal Windows scaling, including Channel Settings switches.
3. Open/close Mixer and Pool using their toolbar buttons. Verify blue follows real state, a second click closes, title-bar close clears blue, and the tooltip offers the opposite action. Recheck Inspector/Info Line Show/Hide behavior.
4. Confirm accepted Mixer strips and the transport/ruler workflow remain intact. Stop for feedback before another substantial packet. No live or M2 acceptance is claimed from this sheet/build.

## Simple-icon follow-up ? 2026-10-24

User requested simpler, more elegant toolbar symbols after finding the previous set strange. This supersedes the original icon appearance pending review, without changing panel behavior or transport scope.

Changed only icons.rs: lighter 1.25 px toolbar strokes, compact 86% geometry for more interior space, narrower conventional black pointer, slider-block Mixer glyph, simpler glue/eraser silhouettes, monochrome color swatches, and removal of the extra zero-crossing dot. Shared power drawing and button layout/state logic remain unchanged. No dependencies added. Implementation model: current session model; no model switch or delegated review claimed.

Two existing toolbar presentation/state tests passed. Formatting and diff checks passed. Inspected actual egui geometry in a synthetic dark/light preview (2800 x 460, 2x presentation; font rasterization approximated). Image: `G:\dev\Opentrack\_analysis-tools\reports\opentrack-comparisons\2026-10-24\T009-toolbar-simple-review.png`. Same preview conditions as above; not a live screenshot or reference match. Prior T009 screenshot remains preserved. Live appearance acceptance is pending user review at normal display scale. Application remains closed; EXAMPLE untouched.

Follow-up release EXE SHA-256: `00AB986E197E560D2D5A630131B5220DC4675D2F650A5B9EA1EA52AA10C3FA19`. icons.rs SHA-256: `4293398B324A18687F1F6BA3CB47A6F140EE06C505009BAC6B0EAD17D68092C7`. x64 build passed. Earlier identity above describes the previous revision.
