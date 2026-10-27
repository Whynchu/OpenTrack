# Tabler icon system — toolbar and application migration

Status: source migration complete; user's live visual review pending. The user
approved continuing from the toolbar prototype through the remaining app after
requesting slightly stronger icons and removal of the menu-bar logo. The full
remaining control inventory is in
[`tabler-application-icon-inventory.md`](tabler-application-icon-inventory.md).
The toolbar retains its existing layout, order, dimensions, tooltips, labels,
state sources, shortcuts, and actions.

## Verified icon source

Assets are from Tabler Icons **v3.49.0**, tag commit `bbed884`, in
`crates/opentrack-app/assets/tabler/`. The exact vendored names are verified in
that release's `icons/outline` and `icons/filled` directories. Outline assets
use 1.75-unit strokes after the user's contrast feedback; the filled player
icons preserve familiar transport silhouettes. The MIT notice is retained
beside the assets.

## Existing Project toolbar inventory

| Location | Control | Verified function | Current asset | Replacement | State | Source |
|---|---|---|---|---|---|---|
| Project toolbar | Activate Project | Disabled placeholder | Custom power drawing | `power` | Disabled | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Constrain Delay Compensation | Disabled placeholder | Custom sliders | `clock-exclamation` | Disabled | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Inspector | Show/hide Inspector | Custom panel drawing | `layout-sidebar` | Toggle; blue when open | `app.rs` `show_toolbar` |
| Project toolbar | Event Info Line | Show/hide event info line | Custom list drawing | `list-details` | Toggle; blue when shown | `app.rs` `show_toolbar` |
| Project toolbar | Overview | Disabled placeholder | Custom overview drawing | `chart-area-line` | Disabled | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Pool | Open/close Media Pool window | Custom folder drawing | `folder` | Toggle; blue when open | `app.rs` `toolbar_window_toggle` |
| Project toolbar | Mixer | Open/close Mixer window | Custom fader drawing | `adjustments-horizontal` | Toggle; blue when open | `app.rs` `toolbar_window_toggle` |
| Project toolbar | Automation Panel | Disabled placeholder; separate Touch text choice is retained | Custom automation curve | `automation` | Disabled | `app.rs` `show_toolbar` |
| Project toolbar | Previous Marker / Zero | Seek to beat zero and stop | Custom transport triangle | `player-skip-back` (filled) | Action | `app.rs` `show_toolbar` |
| Project toolbar | Next Marker | Placeholder | Custom transport triangle | `player-skip-forward` (filled) | Disabled | `app.rs` `show_toolbar` |
| Project toolbar | Cycle | Toggle cycle playback | Custom arrows | `repeat` | Toggle; existing mode color | `app.rs` `show_toolbar` |
| Project toolbar | Stop | Stop playback/recording | Custom square | `player-stop` (filled) | Action | `app.rs` `show_toolbar` |
| Project toolbar | Play | Start playback | Custom triangle | `player-play` (filled) | Toggle; blue when playing | `app.rs` `show_toolbar` |
| Project toolbar | Record | Toggle recording | Custom red disc | `player-record` (filled), existing red | Toggle; red signal color | `app.rs` `show_toolbar`; `theme.rs` |
| Project toolbar | Object Selection | Select/move events | Windows arrow plus custom fallback | `pointer` | Selected tool | `app.rs` `show_toolbar`; `system_tool_icons.rs` |
| Project toolbar | Range Selection | Create/move time ranges | Custom rectangle | `select` | Selected tool | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Split | Split at the edit point | Scissors glyph from Windows icon font | `scissors` | Selected tool | `app.rs` `show_toolbar`; `system_tool_icons.rs` |
| Project toolbar | Glue | Join adjacent events | Custom bottle | `link` | Selected tool | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Erase | Delete events | Custom eraser | `eraser` | Selected tool | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Zoom | Change timeline zoom | Windows icon-font glyph | `zoom-in` | Selected tool | `app.rs` `show_toolbar`; `system_tool_icons.rs` |
| Project toolbar | Mute tool | Toggle event mute by editing gesture | Custom speaker mark | `volume-off` | Selected tool | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Time Warp | Disabled placeholder | Custom clock | `clock-hour-3` | Disabled | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Draw | Draw MIDI notes | Windows icon-font glyph | `pencil` | Selected tool | `app.rs` `show_toolbar`; `system_tool_icons.rs` |
| Project toolbar | Line | Disabled placeholder | Custom line drawing | `line` | Disabled | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Play tool | Start playback from timeline interaction | Custom play triangle and pointer | `player-play` (filled) | Selected tool | `app.rs` `show_toolbar`; `icons.rs` |
| Project toolbar | Color Tool | Disabled placeholder; separate color swatch dropdown is functional | Custom palette drawing | `palette` | Disabled | `app.rs` `show_toolbar` |
| Project toolbar | Project Root Key | Disabled placeholder | Custom text field | `music` | Disabled | `app.rs` `show_toolbar` |
| Project toolbar | Autoscroll | Toggle timeline follow during playback | Custom arrows | `arrow-bar-right` | Toggle; blue when enabled | `app.rs` `show_toolbar` |
| Project toolbar | Snap | Toggle grid snapping | Custom magnet | `magnet` | Toggle; blue when enabled | `app.rs` `show_toolbar` |
| Project toolbar | Snap Type | Disabled placeholder; text-based grid and quantize choices remain | Custom grid marks | `grid-dots` | Disabled | `app.rs` `show_toolbar` |
| Project toolbar | Zero Crossing | Disabled placeholder | Custom waveform | `wave-sine` | Disabled | `app.rs` `show_toolbar`; `icons.rs` |

The `Touch` automation-mode caption, `Bars + Beats`, tempo, time signature,
`Snap`, `Use Quantize`, `1/16`, and event-color swatch remain text/value controls.
The track-header `M`, `S`, `R`, and `W` conventions remain text controls.

## Rendering and scope

`icon_system.rs` is the single semantic registry and SVG renderer. It parses a
vendored SVG on cache miss, rasterizes it at the physical pixel size derived
from egui's display scale, applies a theme/state tint, and caches the texture by
icon, pixel size, and color. Existing click handlers, selected state, disabled
state, and tooltip text remain with their existing components. The Windows
icon-font cursor dependency was removed; split, draw, and zoom cursors use the
central Tabler renderer with the existing native crosshair/hidden-cursor modes.
Enabled toolbar icons use near-black ink against light button faces; disabled
icons remain muted and active controls retain their state colors.

The supplied 1920 x 1030 screenshot is the baseline, showing Project, Mixer,
Channel Settings, and floating Transport. The matching runtime screenshot of
this build is pending user capture, so no before/after parity claim is made.
The current session has no available native desktop capture surface. An enlarged
representative icon sheet showing Default, Hover, Active, and Disabled
treatments is available as
[`tabler-toolbar-specimen.svg`](tabler-toolbar-specimen.svg).

The source migration also covers the Mixer, track headers, Project inspector,
Channel Settings, shared Sample Editor/event toolbar, floating Transport, and
editing cursor overlays. No audio or editing behavior was changed.

## Open review items

- The live 1920 x 1030 after screenshot is still needed for direct before/after
  comparison; the user asked to review the finished result.
- Several disabled placeholder controls have provisional symbols: `clock-exclamation`
  for Delay Compensation and `music` for Project Root Key. Revisit those mappings
  if either control becomes active or the glyph reads ambiguously in the app.
- The scale check covers egui rendering at 100%, 125%, 150%, and 200%; actual
  monitor scaling and cursor hotspots remain a live desktop review item.
