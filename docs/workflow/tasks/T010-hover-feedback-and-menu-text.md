# T010 ? Consistent hover feedback and menu text clarity

- Status: review_pending
- Scope version and requirement IDs: transport-v4 denominator unchanged; presentation follow-up, no new transport requirement IDs.
- Desired observable outcome: enabled buttons, menu actions, and direct manipulation controls give a consistent subtle visual response on hover across the Project, Mixer and Channel Settings. Their selected, disabled, recording and other semantic colors remain recognizable. File/Edit menu labels render at a comfortable size using the system Segoe UI TrueType already loaded by the Windows build.
- Reference evidence: user-described Mixer hover treatment and app/menu appearance. This specifies desired OpenTrack behavior; Cubase reference match is not claimed.
- Dependencies and decisions needed: none. Keep the accepted Mixer structure and colors.
- Code scope and architecture boundaries: shared hover color/style helper in theme; egui widget styles; custom Project toolbar, Inspector, palette and zoom controls; shared Mixer control renderer (also used by Channel Settings), fader/pan/EQ input targets. No behavior or layout changes intended.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Move over active toolbar, transport, Project Inspector, Mixer buttons, Channel Settings buttons, menu entries, and project palette/zoom controls. Each enabled target should show a restrained hover highlight.
  2. Hover faders, pan fields, and EQ bands/bars; the exact interactive target should become visibly emphasized.
  3. Check selected/blue panel toggles, Cycle loop/skip colors, record red, muted/disabled controls, and both light/dark themes; hover must not hide their state.
  4. Read File/Edit/Project/Audio menu labels at normal display scale. They use Windows Segoe UI TTF at 13 points. Egui's rasterizer does not use Windows ClearType, so native-text parity is still a live visual check.
- Affected regressions: Mixer accepted look/behavior, Project toolbar and menus, Channel Settings EQ interactions, panel state colors. User already closed OpenTrack before build. No app was launched; EXAMPLE.opr untouched.
- Essential gates: no audio, project persistence, undo, or dirty-state behavior changed.
- Review owner and milestone stop: user checks the rebuilt executable; stop for feedback.

## Handoff

- Actual implementer/reviewer model: gpt-6-luna; no delegated review.
- Source identity: current working tree based on cbd9835 plus existing authorized in-progress work. exe:sha256:E8A5C7E76DDFDED1EC1D40D248E7545E4585037BCF3043C734DFDB610BB896F8; theme.rs:sha256:DAF72CAC9C75EFA8A13BDFA0CC2363996D1AE7E7F961ECB4EE4BA663269F0CDD; mixer.rs:sha256:C3819A2C493EEF40BE6FA8CBE7F1AE37D1F26DA3FC1D2F56941E75E83344C260; channel_settings.rs:sha256:ED43CDAFC3FA92451EE2506E98DE8B265F46DF99DB73F2292A90619DA4298A27; project_controls.rs:sha256:A88960CDE654FB145BDD22613342A2D8FBB494ADD2065AB2F67C34C40726762F; project_inspector.rs:sha256:E7EC0C9572FC50F59849B30C5EB52E0DF99C66A47C933D7191F94DF0EA9B0726; app.rs:sha256:E4DAE772B071AF6806F32DA9E690130504AA144EFD019502BFB916987E51BCD9.
- Changes and reasons: shared palette-aware hover fill and widget style; applied to standard egui buttons/menus, manual Project toolbar and Inspector controls, color selectors, zoom, Mixer controls, and Channel Settings sliders/filters/graph targets. Increased the menu style from fractional 12.5 to whole 13.0 points. Windows Segoe UI TTF was already loaded; the app uses egui's rasterizer rather than native DirectWrite/ClearType.
- Commands/checks and actual results: cargo fmt --all -- --check passed; git diff --check passed; x64 release build passed. `python scripts/parity_report.py` and `--check` passed. No tests run.
- Runtime environment and evidence paths: no current screenshot; normal-scale desktop review pending.
- Ledger IDs updated; report command/result: see ledger entry `t010-hover-and-menu-font`; parity_report.py results below.
- Review findings, rework, elapsed effort if available: no independent visual review; user review pending.
- Failures/unknowns/pending acceptance: appearance at user's Windows scaling, hover coverage in nested/docked windows, and perceived font smoothness require a live review.
- Next ready task: none until user reports this presentation review.

## User feedback

2026-10-24: user said ?This is good!? and requested moving to functional interface/routing/monitoring work. General presentation approval recorded; individual light/dark, every-control and font cases were not separately reported.
