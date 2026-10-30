# T060 — Project window icon legibility plan

- Status: visual changes reverted at user request; restored build and runtime confirmation complete
- Scope version and requirement IDs: Project-window visual consistency follow-up to T050/T051; audio/MIDI editing behavior is unchanged. Track-height defaults and minimums are adjusted to support the layout.
- Desired observable outcome: every visible icon in the Project window is legible and consistent at 1920×1080, with a measured minimum rendered glyph size and adequate click target.
- Reference evidence: user reports several Project-window icons are too small to render cleanly. Fresh packaged-build review in EXAMPLE.opr captured a maximized 1920×1033 client area on a 1920×1080 desktop. Windows scale factor was not measured; 125%/150% follow-up remains open.
- Dependencies and decisions needed: use 1920×1080 as the design target at Windows 100% display scale, then confirm proportional scaling at 125% and 150%. User approved implementation on 2026-10-29.
- Code scope and architecture boundaries: Project-window toolbar and track-header labels/actions, plus shared icon sizing/painting only where Project uses it. No Mixer/Sample Editor redesign, action semantics, project content, or new dependency. The GroupLanes icon may be replaced in the collapse action by existing expanded/collapsed chevron assets; no other artwork redesign is planned.
- Acceptance checks:
  1. Capture a fresh Project view with representative audio, MIDI and grouped tracks; enumerate each visible icon family and record allocation rect, painted rect and actual occupied SVG bounds.
  2. Establish a 16 logical-pixel minimum for the visible glyph at 100% scaling; target 18 px for primary tools and the Group Collapse chevron. Give the control a 24×24 logical-pixel interaction target where the layout permits. If a row cannot fit these without crowding, resolve density at the row/layout level rather than shrinking the glyph below the floor; record any exception for user review.
  3. Normalize icon artwork bounds and stroke weight before increasing raster size. Confirm contrast and crispness; inspect both light/dark surfaces and inactive, hover, selected and disabled states.
  4. Review at 1920×1080 / 100%, and verify scaling at 125% and 150%. Include a screenshot-based checklist covering Project toolbar, track-header buttons, Inspector and event controls.
  5. Exercise representative controls to confirm the larger visual footprint preserves their existing actions, alignment, spacing and state feedback. Formatting, focused icon rasterization checks and an x64 local-ASIO build are required after implementation.
- Affected regressions: T050 icon registry/rasterization across common scales and themes; T051 shared button chrome; Project toolbar toggle/tool behavior, track-header actions, Inspector and event controls.
- Essential gates: no project-format or persistence changes; user visual review of the measured 1920×1080 result before acceptance.
- Review owner and milestone stop: implementation begins only after this plan is accepted; stop at user visual review of the Project window.

## Audit notes

toolbar_icon_button_tinted currently allocates several widths from 18 to 54
logical pixels at a common 25-pixel height. paint_toolbar paints at the icon's
nominal size capped by the inner rectangle, so narrow allocations reduce the
actual glyph. The shared app-icon paint helpers can clamp their output down to
8 pixels. Those rules explain why button-frame dimensions alone cannot establish
legibility. Measure actual occupied paths and raster edges, including SVG
viewBox whitespace, before changing shared sizing.

## Handoff

- Actual planner: Codex; implementation and visual review not started.
- Source identity: inspected at base commit f4efbe70625f82da20d1bd2ffc93cb7c6570bdb2; no product source changes made for this plan.
- Checks: read current T059 state, T050/T051 precedent and shared icon sizing/painting code.
- Runtime/evidence: prior Project review screenshot showed the live toolbar and track headers; fresh baseline at a controlled 1920×1080 scale is an implementation prerequisite. No screenshot retained.
- Ledger: no behavior ledger changes; plan only.
- Pending: user review of the 16 px glyph / 24 px target baseline and implementation scope.
- Next step: after approval, inventory Project icon families in the running app and implement one shared sizing rule with layout-specific spacing.

## Plan review — 2026-10-29

Verdict: draft needs revision before implementation. No product source changes.

1. Correct the Group Collapse baseline. It uses GroupLanes (stacked lanes plus
   a G), not a chevron. The actual button is 22×14 logical pixels before
   horizontal control scaling; the 14-pixel height limits its icon canvas.
   The SVG has a 37-unit viewBox, 1.5-unit main strokes and a 1.25-unit G
   stroke. At a 14-pixel canvas these strokes are about 0.57 and 0.47 pixels;
   the G itself is only about 3 pixels tall. Even an 18-pixel canvas does not
   establish legibility. Include an explicit choice between simplifying this
   small-size artwork and allocating enough space for its internal detail.
   The existing exclusion of artwork changes conflicts with this need.
2. Specify the compact row layout before fixing 24×24 targets. Current control
   rows start at vertical offsets 3 and 22, only 19 pixels apart. Directly
   enlarging controls to 24 pixels causes overlap. Define row spacing, minimum
   track height, narrow-header behavior and toolbar overflow, with no clipped
   or overlapping hit targets. Test the smallest supported track height and
   nested group indentation as well as the default view.
3. Define size measurements precisely. An icon canvas, occupied artwork and
   hit target are distinct. A universal 16-pixel occupied bound is unsuitable
   for thin lines and asymmetric shapes. Use common canvas tiers, optical
   alignment and per-family visual checks, including internal-detail/stroke
   survival at 100% scale. The proposed 16/18 values are design hypotheses,
   not verified clarity thresholds.
4. Make the shared-renderer boundary executable. Shared paint helpers also
   serve other app surfaces. Pass explicit sizing metrics for the Project
   callers or audit all affected consumers; check that the MIDI tool picker,
   Mixer and Inspector icons do not change unintentionally.
5. Specify physical resolution versus logical viewport. A 1920×1080 display
   at 125% or 150% provides less logical layout space. Record client size and
   effective pixels-per-point in each capture, and include fit/overflow checks
   rather than checking only that the raster scales proportionally.

Implementation decisions (user-approved 2026-10-29): Project toolbar controls
use Project-only 26 px buttons and an 18 px icon floor, keeping transport
controls at their existing 34 px width. Primary tools and panel toggles use
24 px widths. Track-header controls use 18–20 px square targets in two rows;
the left side holds the track type/name and channel label, while actions align
to the right with a 24 px reserve before the color strip and meter. Track height
defaults to 48 px and its minimum is 40 px, enough for both control rows. Group
collapse uses the existing right/down chevrons; take-lane disclosure keeps its
existing icon. Track-type artwork keeps a 20 px canvas and automation reveal
uses an adaptive 18–24 px target. Project callers use an explicit renderer;
MIDI/Mixer shared toolbar sizing is unchanged. Existing maximum visual group
indent depth is retained.

## Rollback handoff — 2026-10-29

- User feedback: the expanded Project layout felt regressive; the user preferred
  the previous look.
- Source: restored the pre-T060 toolbar sizes and renderer, 42 px default and
  36 px minimum track height, original track-header label/control positions,
  automation disclosure size and GroupLanes button. MIDI editing behavior was
  preserved.
- Checks: `cargo fmt --all -- --check` passes.
- Package: `./build.ps1 -Architecture x64 -LocalAsio` passes; metadata confirms
  `local_asio: true`. Restored executable SHA-256:
  `CDC1D424A32DFCCA7BD0131F48D4B666D8CE91CA8AC50BFC25D48065AE2DFC58`.
- Runtime: opened `C:\Users\Samue\Documents\EXAMPLE\EXAMPLE.opr` in the
  restored packaged build. The familiar compact toolbar, track-header label
  positions, small M/S/action buttons, and GroupLanes disclosure are visible
  again. The maximized client was 1920×1033 on the 1920×1080 desktop. The
  example was saved via its normal close prompt with the user's prior
  authorization, and OpenTrack is closed. The restored screenshot was shown to
  the user; final visual acceptance remains theirs.
