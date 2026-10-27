# T048 - Transport window branding, focus and pinning

- Status: implementation complete; x64 local-ASIO release build passed; user review pending. Implementer: Codex session; no independent review.
- Scope version: `transport-v18-native-transport-window`.
- Requirement IDs: `transport-child-window-shortcuts`, `transport-window-always-on-top`, `opentrack-window-icon`.
- Desired outcome: VST Audio Channel Settings shows the OpenTrack icon at the top-left of its native title bar. Transport shortcuts such as Space work while Channel Settings or Transport has focus. The compact Transport keeps its 837x69 content, offers a right-click Always on Top checkbox, and when pinned remains above Channel Settings. Pin and screen bounds persist in app preferences.
- Feature decisions: user direction at `docs/workflow/README.md#t048-direction-2026-10-26`; existing 837x69 Transport layout in `transport_panel.rs`; native Channel Settings viewport and shared keymap are implementation baselines.
- Dependencies and decisions: the existing Windows physical-key subclass is attached only to the main HWND. Use it only for the root viewport; route key events in child viewports through their focused egui input context. Keep Channel Settings' existing owner-relative pin semantics. The new Transport pin applies only to the Transport native viewport and uses `WindowLevel::AlwaysOnTop`, so it can cover Channel Settings. The first generated app mark was replaced in T049 by a tape-reel silhouette based on the user's description; the historical image asset is not present in this repository.
- Plan:
  1. Add an original small OpenTrack icon and apply it to the app root and native child viewports, including VST Audio Channel Settings.
  2. Make root-only physical key capture explicit; process child viewport egui key events so Space and other keymap shortcuts work in those windows.
  3. Move the existing compact Transport into a borderless native viewport while preserving its geometry and draggable behavior.
  4. Add a right-click Always on Top checkbox, persist the preference and window bounds, and use native window level so Transport stays above Channel Settings when pinned.
  5. Format, build x64 with the existing local-ASIO configuration, update the ledger/report, and leave live focus/z-order review to the user.
- Code scope: `crates/opentrack-app/src/{app.rs,icons.rs,main.rs,transport_panel.rs}`, `docs/workflow/{README.md,current.md,ledger.json,metrics.md}`, and this packet. No changes to audio processing, project serialization, or Channel Settings' owner-relative pin behavior.

## Acceptance checks

- Open VST Audio Channel Settings and verify the OpenTrack mark appears in the top-left title-bar icon.
- Focus Channel Settings and press Space; confirm transport starts/stops. Repeat with the Transport window focused and verify other transport keymap commands work.
- Right-click the Transport, enable Always on Top, and confirm it stays above VST Audio Channel Settings. Disable the option and confirm ordinary stacking returns.
- Drag the borderless Transport from an unoccupied panel area; verify its position remains usable and is restored on restart.
- Confirm the 837x69 content, all buttons, right-click menu, and Window-menu/F2 visibility toggle still work.
- Build the x64 release with local ASIO enabled. Do not launch the app as part of the code/build handoff. Visual, focus, z-order, and persistence checks remain user review gates.

## Handoff

- Actual model: GPT-6; no subagents.
- Source identity/build hash: executable SHA-256 `A91EA6C01C8652A0C9AA1987C9109CE8D1CA1AC4D55957DF6397890989327461`; `app.rs` `2966E99F6683ED0B8ED647695522C91F25AA21CAA88F5D7EA196B636257BAC0A`; `icons.rs` `396E0B45F4261BE40EAFB3F793E48A5132320755EA023E1161BB2A32FE61D024`; `main.rs` `6234724FA06ADB80F4A9A19F4B751BC6C50779B3491A11F98178D8D8B463DCDE`; `transport_panel.rs` `568A55148DE8A8D27FBF524A32A2C5C7554942C6CB30FD4EA3231108A1B0A8E1`. The separate T047 executable remains unchanged at `3E1A39B3D90EF52CE8688BBC3C387706265E6868B395A680484F7904CD457411`.
- Changes and reasons: the shared original OpenTrack app icon now reaches the root and native child windows. Child viewports use egui's key events while the root keeps its Win32 physical-key adapter, enabling shortcuts when Transport or Channel Settings is focused. Transport is a borderless native viewport with right-click Always on Top, owner-independent topmost stacking, restored bounds, and app-level persistence for the pin.
- Commands/checks and actual results: `cargo fmt --all` and `cargo fmt --all -- --check` passed; `./build.ps1 -Architecture x64` passed and reported local ASIO enabled; `python scripts/parity_report.py` validated the ledger and regenerated `metrics.md`. No tests were run.
- Runtime environment and evidence paths: Windows x64, local-ASIO release. OpenTrack was closed before the build and remains unlaunched. No live GUI, focus, z-order, or preference-restart review was performed.
- Ledger IDs updated; report command/result: `transport-child-window-shortcuts`, `transport-window-always-on-top`, `opentrack-window-icon`, workflow `native-transport-window-controls`, and gate `transport-window-focus-review`; parity report passed.
- Review findings, rework, elapsed effort if available: no independent review.
- Failures/unknowns/pending acceptance: live key focus, title-bar icon, always-on-top stacking, and window restore checks remain pending.
- Next ready task: user review of the x64 local-ASIO build.
