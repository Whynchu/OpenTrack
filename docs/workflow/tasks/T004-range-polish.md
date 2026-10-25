# T004 — Range appearance and adaptive ruler density

Status: review_pending. Implementer: GPT-6.1 Sol. Scope: transport-v1; loop-range, skip-range, adaptive-ruler and ruler-regressions. Current user request authorizes this polish after confirming loop and skip work.

## Changes

- Pencil cursor uses white fill and black outline/tip.
- Both active Cycle buttons use the same purple Loop / orange Skip colors as the ruler range, including the floating Transport button border. Their actions and shared transport state are unchanged.
- White triangles with black outlines sit at the top locator corners and point inward. They draw above ticks and shrink for narrow ranges. Geometry follows the user's description; exact reference flag dimensions remain unmeasured.
- A low-opacity range overlay extends from the ruler through the full track canvas, including empty space and events, clipped to the selected time interval and excluding pinned headers/Inspector/scrollbars. Preview and committed ranges share the rendering.
- Ruler labels use globally anchored 1/2/4/8... bar strides based on actual bar width and measured visible label width, with a 44px minimum spacing. Tick/grid density also reduces at distant zoom. Snap remains the independent Bar grid.

Existing saved reference `G:\dev\Opentrack\_analysis-tools\reports\opentrack-comparisons\Cubase-user-open-example-project-0.jpg` shows labels at 1,5,9... at its zoom. It was inspected for density context; it has no selected loop range, so it does not establish triangle/tint reference parity. Thresholds and tint strength are implementation choices for the requested appearance, pending user review.

## Evidence and checks

- Existing x64 build passed before edits; final x64 build passed. Process absence checked before builds; no application window was opened. EXAMPLE was not touched.
- Nine timeline calculation tests passed, including adaptive density and wide labels. Three actual headless ruler-handler tests passed for snap/range/Undo/cancel regression.
- Formatting and diff checks passed. The prior unrelated full-suite theme failure remains documented; it was not changed or rerun in this rendering slice.
- No current desktop screenshot or independent live visual review was performed. The user requested to perform the review. Loop/skip audio processing and persistence source files were unchanged in this slice.
- Build SHA-256: `76416EC3F931E4BCD550BC18C2F3D447C0D6AAF16B0D7CFAA60F32F0E85056F5`.
- `app.rs` SHA-256: `294E85343C38520A66AE7AA2BA0D6923DFCE51C08ABB5405CDF1C1A2A33A1289`.
- `timeline.rs` SHA-256: `BD16422F5FCAF065BDDE034CEEA670F4A147E182B718BB7D034AFE3F25A2EA04`.
- Base commit `cbd9835` plus uncommitted transport changes. Publication and M2 acceptance remain separate.

## Review steps

1. Draw each range direction. Check the monochrome pencil, inward top corner flags, and faint tint covering only the interval through the entire track canvas.
2. Enable Cycle and compare Project and floating Transport button colors for Loop and Skip. Disable Cycle and verify their inactive state.
3. Zoom in until individual bar numbers appear; zoom out until labels move through wider intervals. Pan horizontally to check globally stable numbering and no label crowding. Check long ruler formats as well.
4. Recheck seeking/snap, loop/skip playback, range edge scrolling and the ruler context menu. Report remaining appearance/interaction differences before the next packet.

The user's functional loop/skip confirmation is retained in T003. Audible media, exact cursor/audio agreement, persistence and close gates are not inferred from that brief report. No current screenshot, reference match or full workflow acceptance is claimed.
