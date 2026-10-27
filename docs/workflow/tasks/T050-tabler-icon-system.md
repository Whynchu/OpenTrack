# T050 — Tabler icon system, toolbar migration, and supplied branding

- Status: review_pending
- Scope version and requirement IDs: visual icon migration; no transport or editing behavior requirement changes.
- Desired observable outcome: one pinned Tabler SVG library and renderer; stronger but still soft outline icons across existing UI; supplied app mark and GitHub/splash artwork; preserved DAW layout and conventions.
- Reference evidence: user screenshots `Screenshot 2026-10-26 214901.png`, `215827.png`, and `221817.png` (live layout reference); user-supplied icon SVG and two wordmark SVGs (branding source).
- Dependencies and decisions needed: a live after screenshot is needed to verify actual Windows rendering, exact menu spacing, high-DPI behavior, and cursor hotspots.
- Code scope and architecture boundaries: `icon_system.rs`, `icons.rs`, existing egui/resvg stack, ToolbarIcon and related UI controls, and the requested brief startup splash. No audio engine, edit behavior, project data, button dimensions, or track layout changes.
- Acceptance checks: inspect full toolbar/mixer/track/channel icon mapping; parse and rasterize every registry asset at 100%, 125%, 150%, 200% in dark/light theme; verify release x64 local-ASIO build; inspect README, Tabler MIT notice, banner/splash SVGs; user live screenshot and cursor hotspot review.
- Affected regressions: `cargo test -p opentrack icon_system::tests::every_registered_asset_renders_at_common_display_scales_in_both_themes`; `cargo test -p opentrack icons::tests::supplied_mark_and_splash_svg_parse`; format; app release build. No full audio/edit suite needed because no audio or editing behavior changed.
- Essential gates: user visual review pending; no runtime acceptance claimed.
- Review owner and milestone stop: user review of the complete application migration and final build.

## Handoff

- Actual implementer/reviewer model: Codex; independent review pending.
- Source identity (commit plus dirty diff/artifact identity) and build hash: working tree; x64 local-ASIO executable SHA-256 `3DEE4E460FBC060ED7662100C3256CDC2FFE44262AB8BC345DADF0E88B6010B8`.
- Changes and reasons: centralized semantic Tabler registry; outline width 1.75 and near-black enabled icons on light button faces after contrast feedback; removed duplicate menu-bar mark; corrected app mark raster scale and raised native icon resolution to 256px for Windows downsampling; replaced old app mark with user-supplied SVG; added transparent GitHub banner and dark startup splash with the specified wordmark styling.
- Commands/checks and actual results: `cargo fmt --all --check` passes; `git diff --check` has no whitespace errors (Git reports expected CRLF-normalization warnings). Focused registry rendering test passes for both themes and four display scales, including splash rasterization. Supplied mark/splash SVG parsing test passes. Specimen generator runs. `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64` succeeds with local ASIO enabled; binary is staged at `builds/x64/OpenTrack.exe`. Compile reports five pre-existing dead-code warnings.
- Runtime environment and evidence paths: user Windows screenshots are the source reference; no desktop capture surface is enabled here. Review report and specimen are in `docs/reviews/`.
- Ledger IDs updated; report command/result: no behavior-ledger entries changed; task-local source inventory in `docs/reviews/tabler-application-icon-inventory.md`.
- Review findings, rework, elapsed effort if available: screenshot and cursor hotspot comparison pending.
- Next ready task: user visual review; revise mappings only if the live screenshot identifies a concrete issue.
