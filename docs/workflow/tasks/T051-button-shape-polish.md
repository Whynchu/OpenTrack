# T051 — Compact button shape and letter-control polish

- Status: review_pending
- Implementer/reviewer: GPT-6.1 Sol; source self-review, user visual acceptance pending.
- Desired outcome: consistent compact, softly rounded button shapes throughout OpenTrack, including conventional letter controls.
- Evidence: current README screenshots supplied by the user on 2026-10-26, Project/VST and Sample Editor/Pitch & Time views.
- Scope: shared button chrome, label fit, standard egui button corners. Preserve existing rectangles, control order, letter conventions, colors, actions, tooltips, shortcuts and audio behavior.
- Acceptance: 2–3px compact corners, minimal edge, no bevel highlight, visible pressed/hover/focus/disabled states, centered legible letters. Existing round record and channel-edit controls stay circular.

## Audit and implementation

| Control family | Source | Result |
|---|---|---|
| Project and Sample Editor toolbar | `app.rs` toolbar button helper | Shared compact chrome; near-black ink on light faces and existing state colors |
| Track header, Inspector, automation letters | `project_inspector.rs` button helper | Fitted Segoe UI Semibold M/S/R/W/E labels; same hit rectangles |
| Mixer and Channel Settings letters and utility buttons | `mixer.rs` control helper, reused by `channel_settings.rs` | Removed bevel stripe; shared edge/corners; centered circular e label |
| Floating Transport and pitch arrows | `transport_panel.rs` button helpers | Same chrome and pressed/focus treatment; dimensions retained |
| Panel close controls | `app.rs` close helper | Compact rounded shape and Tabler close symbol |
| Native egui text buttons, dialogs, secondary windows | `theme.rs` widget visuals | Consistent 3px corners in inactive, hover, pressed and open states |
| Menus, dropdown fields, faders, knobs, EQ nodes | Existing components | Inspected; retained their distinct geometry and interaction roles |
| Hosted VST effect interfaces | Plug-in window | Artwork remains owned by the plug-in |

`theme::paint_control` centralizes geometry, a subtle edge, hover, press and
focus painting. The caller continues to own the hit target, state and command.
`control_label_font` fits semibold letters to their compact control height and
falls back to the existing proportional font when the Windows font is absent.
No new dependencies, project format changes, engine changes or action changes.

## Handoff

- Verification: `cargo check -p opentrack`, formatting and whitespace checks pass. Eleven existing checks pass: Mixer bus edit targets and fader interaction (2), Transport (4), theme (3), toolbar window toggle (1), icon/splash rendering at four DPI scales (1). No new tests were added for this paint-only change.
- Build: `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64` succeeds, `local_asio: true`; SHA-256 `A19AA8B5A92AF419B9369F7D1FBAD2E5FBB6539953702329A1AF9412DF33A506`. Five existing dead-code warnings remain. Source base: `825dd88` plus this task's diff.
- Visual/runtime limits: the supplied screenshots guide this pass; no current desktop capture is available. Live appearance, actual monitor scaling and keyboard focus review remain pending user review.
- Publication: commit/push authorized by the user's ongoing instruction. Publication does not establish visual acceptance.
