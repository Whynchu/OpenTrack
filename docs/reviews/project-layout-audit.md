# Project Inspector and header layout audit

Date: 2026-10-23. User requested analysis and documentation before further implementation. No product source changed in this audit.

## Evidence and conditions

External report folder: `../../../_analysis-tools/reports/opentrack-comparisons/Project-layout-audit/`. Reference `Project-layout-cubase.jpg` and current `Project-layout-opentrack.jpg` include JSON manifests. Reference is 1906x1043 at screen origin(7,0); current is1913x1041 at(7,0). Display scaling was not independently measured. Measurements below are approximate capture pixels, not proven logical layout units. Both show stopped projects with two Audio tracks and one Instrument track, Project above Mixer. Cubase has the first Audio track selected with a Volume automation lane exposed; OpenTrack has the first Audio track selected without an automation lane. Name, arm and color states differ, so those state differences must not be mistaken for geometry defects. The first OpenTrack Inspector title is effectively unreadable over its turquoise fill in the current capture.

Comparisons: `Inspector-side-by-side.png` and `Project-header-side-by-side.png` (reference left, current right). Earlier approved overall layout remains workflow-L1/04-preferred-approved-0.png; original tooltip facts remain in the tooltip sweep log. This is a new geometry audit, not replacement approval.

## Coverage and proposed changes

| ID / control | Reference expectation observed | Current capture / mismatch | Action and shortcut | Source to change | Status |
| --- | --- | --- | --- | --- | --- |
| PIA01 Inspector nesting | Controls begin near x14; right edge near170; slim frame immediately beside Project edge | Current controls begin near20; right edge near170; obvious empty outer strip and narrower usable controls | Layout only; preserve Inspector toggle | app.rs show_project Inspector Panel and parent content margins | Observed mismatch; remove cumulative inset after measuring parent frame |
| PIA02 Inspector title | Compact colored title with clear name, disclosure and round e at right | Title text almost disappears on turquoise; e is boxed and no equivalent disclosure | e opens Channel Settings; disclosure behavior still to probe | project_inspector.rs show/header/button | Observed mismatch; readable text on color and correct original glyph geometry needed |
| PIA03 Primary controls | Tight rounded enclosure, compact M/S/R/W and utility buttons; large round record projects slightly below enclosure | Current spacing and grouping are boxier; disabled controls very faint; record and utility graphics differ | Preserve existing working M/S/Record; missing backend actions stay documented | project_inspector.rs button/symbol/show | Visual mismatch; active/disabled reference states require matching state capture |
| PIA04 Volume | Wedge level symbol; black well about x65..167; number right aligned, 0.00, cyan horizontal level indicator below | Current x71..170 field; centered 0.00 dB; rising-bar symbol and tiny endpoint tick | Volume adjustment; exact click/drag/reset/modifier rules unmeasured in this audit | app.rs inspector_parameter_row | Observed; replace generic DragValue presentation with measured control geometry |
| PIA05 Pan | Bow-shaped symbol; centered C above horizontal rail and small center marker in same well as adjacent parameters | Current generic four-direction arrow; separated box; C and indicator too close/overlapping | Pan adjustment; exact gesture and pan law remain unverified | app.rs inspector_parameter_row | Observed mismatch; preserve value parsing while correcting display |
| PIA06 Track delay | Clock with side arrows; right-aligned 0.00; lower narrow rail within grouped well | Current stopwatch-like glyph, centered 0.00 ms; separate field; marker position differs | Track delay adjustment; no tooltip observed in prior sweep | app.rs inspector_parameter_row | Observed geometry mismatch; full behavior unknown |
| PIA07 Routing and preset | Input/output share framed black routing well; preset has diamond left and disclosure at right | Current two independent rectangles; preset reuses routing arrow; approximate y307 vs reference313 input start | Routing/preset choices require prior workflow evidence; currently limited | project_inspector.rs field/show | Observed mismatch; redraw grouping without inventing routing backend |
| PIA08 Inspector sections | Inserts starts around383; six rows about22px, distinct right icon cell; Quick Controls enabled-looking tint in this state | Current Inserts around379; divider/icon column absent; low contrast and different icons | Existing section expansion; substantive panel content incomplete | project_inspector.rs sections | Observed mismatch; state-dependent tint requires matching evidence |
| PIA09 Title/toolbar/info stack | Reference title near57..87; toolbar89..125; info126..155; ruler156..178 | Current title55..85; toolbar86..122; info123..152; ruler153..175; several bands about3px early | Preserve accepted close-only dock behavior; do not restore native min/max | app.rs docked_panel_header, Project toolbar and info_line | Approximate mismatch; measure common parent origin before changing heights |
| PIA10 Toolbar left cluster | Power near22, then reference-specific groupings and Touch field around220..302 | Current group count, separators and order differ; Touch around235..318 | Actions/enabled states/tooltips must follow L1 evidence; no top-menu tooltips | app.rs Project toolbar helpers | Observed mismatch; audit each icon against ordered tooltip records |
| PIA11 Transport/tool group | Reference previous/next/cycle/stop/play/record is compact around318..495; tools begin near507 | Current controls spread farther left/right, tools begin near529; icon hit boxes and silhouettes differ | Preserve working key commands; compare enabled/active states separately | app.rs Project toolbar | Observed mismatch; per-control measurements required |
| PIA12 Right toolbar cluster | Reference snap/grid/quantize/zero/color controls form distinct compact groups; visible root-key dropdown | Current generic placeholders, missing dropdown silhouettes, missing separators and uneven gaps | Some actions incomplete; inventory before enabling | app.rs toolbar_choice and Project toolbar | Observed visual/feature gaps; no new behaviors inferred |
| PIA13 Global Track List strip | Compact graphical controls and color-display button | Current textual symbols with different spacing and no matching control silhouettes | Existing show/hide controls retain semantics; unknown actions stay unknown | app.rs show_project global track controls | Observed mismatch |

## Bounded change order and acceptance

1. Fix Inspector nesting and shared parameter well geometry first, using a matched first-Audio selection and identical default gain/pan/delay values. Keep Project dock and Track List boundary fixed while removing the unnecessary left buffer. Re-capture the same crop and check field alignment, icon placement, rails and label contrast in Windows light and dark modes without changing OS settings automatically.
2. Correct title/control enclosure, routing/preset frames and section icon column. Match enabled/disabled/armed states before judging contrast. Ensure field text, selected title and button glyphs remain readable.
3. Audit Project header from left to right against existing tooltip manifests. Record exact bounds, ordering, separators, active states and each supported action. Correct one toolbar group at a time; avoid a broad absolute-position rewrite (the earlier pass regressed).
4. Rebuild and capture each bounded slice. Compilation does not establish parity. Provide phone-readable crops and a full Project comparison, update the coverage table with implemented/verified status, and pause for user review at the completed slice.

Automation reveal, inline rename and track-type icons remain separately documented in project-interaction-study.md. Their implementation is still pending. Exact tooltip onset, display scaling, parameter gesture/reset rules, preference-dependent layout and most disabled-state reference comparisons remain unknown.


## Fresh edge-aware measurements — user clarification

Cubase was deliberately narrowed to expose previously clipped controls. Its outer width is evidence-capture setup, **not** the desired OpenTrack width. OpenTrack should fill the available host width and retain the same internal edge controls. Do not scale the screenshot or add a permanent blank outer gutter to reproduce the reference setup. Right-side controls should anchor to the Project content edge; the timeline absorbs width changes.

New evidence: `Project-edge-fresh-reference.jpg/json`, `Project-edge-fresh-current.jpg/json`; phone crop `Project-right-edge-comparison.png`; stacked full header `Project-header-fresh-comparison.png`, all in the external Project-layout-audit folder. Coordinates below are approximate pixels in these fresh captures. Their manifests preserve capture dimensions/origins; display scaling remains unknown. Reference title/toolbar moved vertically from the previous capture: use local band relationships, not the old absolute screen y.

| ID | Fresh reference bounds / relationships | Current difference and proposed requirement |
| --- | --- | --- |
| PIA14 Project outer edge | Reference Project right edge near1880, content right near1873; host continues to1920 | Deliberate reference narrowing. Full-width OpenTrack target; reserve internal control columns before computing timeline width |
| PIA15 Right vertical control rail | Approximately x1857..1873, width16; begins below ruler near171, continues to558 | Current timeline extends almost to1905 with no equivalent persistent rail visible. Retain scroll arrows/thumb, split handles and lower zoom area at the content right edge |
| PIA16 Upper rail segment | At x1857..1873, y171..230, lower triangular handle; next up-arrow cell near231..247 | Separate geometry is visible; exact handle actions remain unknown until probed. Do not invent names/tooltips/actions from shapes |
| PIA17 Main vertical scroll region | Approximately y247..463, including down-arrow at lower end | Must remain visible at full width; scrolling/track-height behavior needs independent acceptance |
| PIA18 Lower vertical zoom region | Approximately y466..558, narrow +/- area with triangular handle | Missing in current capture. Exact gesture and mapping unknown; match bounds first only after behavior probe |
| PIA19 Bottom navigation strip | Reference y558..574, roughly16 high; horizontal scrollbar from timeline x370 to1760; lower-right controls occupy x1760..1873 | Current Project ends at y589 without equivalent visible navigation strip. Reserve bottom strip within Project, not at bottom of whole app |
| PIA20 Bottom-right group | Small arrow/dropdown cell, minus/plus horizontal zoom slider, corner grip in roughly113px | Keep anchored to lower-right; scrollbar shrinks with host width; glyph functions/tooltips still require observation |
| PIA21 Ruler right end | Dropdown-shaped cell at x1857..1873, y148..170 | Current ruler runs to the edge with no matching terminal control. Its action remains unknown |
| PIA22 Header vertical bands | Reference title48..78, toolbar79..118, info119..147, ruler148..170; current title55..85, toolbar86..122, info123..152, ruler153..175 | Do not carry previous absolute3px offset forward: origins changed. Align band heights/relative boundaries in a common local Project frame |
| PIA23 Full toolbar width | Last visible color selector near1247..1280; blank toolbar area continues to content edge | No newly exposed toolbar buttons appear at far right in this state; do not invent overflow content. Retain full-width band and right-edge frame |
| PIA24 Inspector remeasurement | Left controls near16..170; Track List starts175; timeline starts370. Current Inspector controls near20..170, Track List175, timeline374 | Current inner inset about4px larger in this fresh state; timeline starts about4px later. Earlier6px estimate belongs to previous frame. Measure local edges before fixing parent margins |

Acceptance: at full-width OpenTrack, verify every rail cell, scrollbar, zoom group and corner remains visible. Repeat at a narrower host width to confirm anchoring rather than screenshot scaling. Preserve the user-approved close-only dock headers; reference min/max buttons are visible evidence but intentionally not requested product behavior. No product code or build was changed in this measurement pass.


## Corrected outer-maximized capture — supersedes window-state measurements above

User clarified: maximize only the outer application. Preserve the Project/Mixer split; do not maximize Cubase's Project MDI child. The briefly maximized-child capture is excluded from measurement evidence. New authoritative capture pair: `Project-outer-max-reference.jpg/json` and `Project-outer-max-current.jpg/json`. Both show outer maximize/restore title glyphs and split layouts. Cubase Project child remains restored/narrowed as user arranged, exposing its right edge; OpenTrack retains its docked split. Both transports are stopped in final captures. OpenTrack cursor differs after the interrupted earlier playback; cursor position does not affect this geometry audit. No project files saved or source implementation changed.

Three new comparisons: `Inspector-outer-max.png`, `Header-outer-max.png` (stacked), `Right-edge-outer-max.png`. Approximate capture-pixel bounds below supersede earlier absolute positions; preserve intrinsic sizes when stretching OpenTrack to the host edges. Display scale still unmeasured.

| Region | Cubase, outer maximized / restored Project | OpenTrack, outer maximized / docked Project | Consequence |
| --- | --- | --- | --- |
| Inspector | Control left16, right170; list starts175; timeline370 | Control left20, right169; list175; timeline373 | About4px extra control inset; about3px extra timeline offset |
| Parameter well | x66..168, y227..294, about102x67; shared frame | x70..169, y221..283; three separate fields | Fix grouped well, symbols, right-aligned gain/delay and center pan rail; compare local offsets to Inspector title |
| Inspector title | y149..169, around20px | y149..165, around16px | Title and following control group heights differ; label remains low contrast in current |
| Project title | y49..79 | y47..78 | Similar band height; different outer/native hierarchy explains origin offsets |
| Toolbar | y79..119, about40px | y78..115, about37px | Match local height and measured group positions; avoid compensating with arbitrary outer offsets |
| Info line | y119..148, about29px | y115..145, about30px | Toolbar boundary accounts for most ruler offset |
| Ruler | y148..171, about23px | y145..168, about23px | Retain ruler height; correct preceding stack |
| Right rail | x1857..1873, about16px; y171..558 | No corresponding permanent rail visible; timeline reaches1904 | Reserve rail inside full-width content edge, not an outside gap |
| Bottom strip | y558..574, about16px | Project runs to581 without equivalent strip | Reserve bottom navigation inside Project; do not use Mixer or application bottom scrollbar |
| Lower-right controls | aboutx1760..1873, width113 | Missing in Project capture | Anchor group inside content right; extend timeline/scrollbar into remaining width |

The reference's approximately40px outer right gap is a user-created evidence setup. It is not a product requirement. Keep OpenTrack full width while retaining the measured16px rail and bottom strip. Source targets remain show_project Inspector frame/scroll allocation, inspector_parameter_row, project_inspector::show, and Project toolbar/info-line construction. Exact control actions and tooltip timing remain as previously recorded/unknown.


## Full toolbar and far-right review coverage

The review output must include **all** of the Project toolbar, from Activate Project through Color Selector, the remaining blank band to the right, the terminal ruler cell, the entire vertical navigation rail and the bottom-right controls. Inspector crops alone are insufficient. New captures `Project-full-controls-reference/current.jpg` retain the full windows; JSON manifests state their actual bounds. Window dimensions changed during this live pass; compare local toolbar/edge relationships, not absolute screen offsets, and use the preceding outer-maximized pair for the controlled1920x1032 geometry baseline. No child maximize was performed in this pass. No product source changes.

Readable review images: `Toolbar-section-1.png` through `4.png` cover the entire width with overlapping intervals0..510,500..1000,990..1500,1490..1906. Reference is above current. `Full-project-right-controls.png` shows reference left/current right from ruler to bottom-right navigation. These crops align local toolbar bands; they are not pixel overlays. Full originals remain the primary evidence.

| Area | Fresh visible differences and required review |
| --- | --- |
| Activate Project / Constrain Delay Compensation | Power location, active background, compensation glyph and spacing differ; use P-1 and T2-02 for hover text. Exact disabled/active behavior is not established by appearance |
| Inspector / Info Line / Overview / Pool / Mixer | Reference buttons form tightly joined groups with several distinct original symbols; OpenTrack uses separated buttons and different silhouettes. Inventory each enabled state and action against T2-04..08 |
| Automation panel / mode | Reference has automation curve button and compact black Touch dropdown; current curve glyph, gap and mode-field start/width differ. T2-09..10 |
| Previous / Next / Cycle / Stop / Play / Record | Reference transport block is aroundx318..499; current aroundx329..518, with different internal widths and spacing. T2-11..16. Compare stopped state and preserve keyboard commands |
| Editing tools | Reference joined tools aroundx507..761; current begins around528 and includes different tool ordering/silhouettes and separators. Map every tool T2-17..28 rather than using a generic icon row |
| Root key / Autoscroll / Snap | Reference black root-key dropdown and compact blue toggles contrast with current bare C and generic marks; several hit-target/dropdown silhouettes missing. T2-29..32 |
| Grid / Quantize | Reference black Bar and1/16 fields plus preceding symbols and dropdown arrows; current widths/gaps differ. T2-33..34 |
| Zero crossing / Color Selector | Reference zero-crossing tile and palette dropdown end around1280; current zero-crossing mark and palette button have different shapes. T2-35..36. Toolbar selector is separate from track color strip |
| Toolbar right remainder | Reference band continues blank to content edge in this state. Include it in full-width review; no unseen toolbar buttons inferred |
| Far-right ruler and rails | Terminal dropdown-like ruler cell; upper split handle, vertical scrollbar with arrows, lower zoom/split area. Current has no matching persistent control column. Follow PIA15..21; individual actions still unverified |
| Bottom-right | Horizontal scrollbar termination, small dropdown/arrow cell, minus/plus slider and corner grip. Missing from current Project; do not confuse Mixer scrollbar with this region |

`full-toolbar-coverage.csv` lists35 identified tooltip targets from the ordered sweep (the ambiguous T2-03 probe remains excluded), links each primary capture and names current source/screenshot. This extends review coverage; it does not mark actions, shortcuts, tooltips or geometry as matched. Top-level menus still have no tooltips.


## Implementation iteration1

First slice changes only parent Inspector inset16→12px, title16→20px, toolbar33→36px, title color contrast on assigned colors, level/pan vector glyphs and parameter label/rail presentation. Existing DragValue input/parse paths retained. Grey default swatch reset from prior source is included. x64 release build succeeded with two existing unused-function warnings. Live legacy two-track RWP opened; gain0.00, R25, delay-12.50 shown. Capture Project-fix-iteration1.jpg and comparison saved externally. This is an iteration, not accepted parity: shared-well frame, pan glyph, remaining button contrast, toolbar silhouettes and ordering, full navigation rails, inline rename and automation reveal are incomplete. Reference selected track/arm/name states differ. No audio test or complete parameter interaction verification performed. Backup source outside Git at Project-fix-backup.


## Source-only toolbar and Inspector icon follow-up — 2026-10-24

The Project toolbar strip and icon tiles now use a shared fill helper. Menu-label foreground follows the strip color in Windows dark appearance; toolbar glyph ink follows each tile fill so selected tools retain contrast in either system theme. Audio’s track-type drawing uses a waveform. The Inspector time-alignment symbol is now a clock with side arrows, matching the study’s recorded direction marks. The existing common Inspector parameter well, Volume indicator bar and Pan center rail remain code-drawn.

This is a source-only follow-up. The latest successful binary predates the selected-toolbar-glyph ink and clock-arrow tweaks; the currently running process must be closed before the next build, per the user’s instruction. A latest-source full-window capture is pending because the computer tool currently returns no app windows. Do not claim these final icon changes are visually matched from the earlier screenshot. Parameter pointer behavior, value reset, and exact well spacing remain unknown.
Additional source-only contrast adjustment: Inspector button text and glyphs now use the same fill-dependent ink choice as the Project toolbar, so selected buttons remain readable with either theme accent. This is not included in the `C3E31E...` binary and still needs visual review.