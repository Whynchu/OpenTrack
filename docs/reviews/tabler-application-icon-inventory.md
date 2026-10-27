# OpenTrack Tabler application icon inventory

Status: application-wide source migration complete; runtime visual review pending.

Upstream assets are pinned to Tabler Icons v3.49.0, tag commit `bbed884`.
Only verified SVGs are vendored in `crates/opentrack-app/assets/tabler/` with
the upstream MIT notice. Their rounded outline geometry uses 1.75-unit strokes
after the user's request for slightly stronger contrast. Transport uses
Tabler's filled conventional player glyphs. OpenTrack's supplied five-dot
application mark is kept separately at `crates/opentrack-app/assets/opentrack-icon.svg`.

## Toolbar

The existing toolbar order, groups, spacers, dropdown text, dimensions,
tooltips, actions, and shortcuts remain in `app.rs`. Full source inventory and
behavior mapping: [toolbar prototype inventory](tabler-toolbar-prototype.md).
The specimen sheet shows default, hover, active, and disabled states at
enlarged size.

## Remaining application controls

| Location | Control / verified function | Previous artwork | Replacement | State and notes | Implementation |
|---|---|---|---|---|---|
| Track header | Audio track type | Procedural waveform | `wave-sine` | Informational hover target | `project_controls::type_icon` |
| Track header | MIDI track type | Filled piano-key drawing | `piano` | Informational hover target | `project_controls::type_icon` |
| Track header | Instrument track type | Filled piano-key drawing | `keyboard` | Informational hover target | `project_controls::type_icon` |
| Track header | Monitor enable | Unicode circle glyph | `volume` | Existing toggle state and tooltip | `project_inspector::button`, `app.rs` |
| Track header | Take lanes visibility | Unicode row glyph | `layout-rows` | Existing toggle and supported-track disable state | `project_inspector::button`, `app.rs` |
| Track header | Mute, Solo, Record-enable, Effects | Letter labels M, S, R, E | Kept as text | Existing DAW conventions and behavior | `app.rs`, `project_inspector.rs` |
| Mixer | Phase inversion | Procedural crossed ring | `circle-off` | Toggle | `mixer::control` |
| Mixer / Channel Settings | Inserts | Procedural module symbol | `plug` | Existing section state; labels retained | `mixer::control` |
| Mixer / Channel Settings | Equalizer | Procedural diamond/equalizer | `chart-line` | Existing section state | `mixer::control` |
| Mixer / Channel Settings | Sends | Procedural send lane | `send` | Existing section state | `mixer::control` |
| Mixer | Monitoring | Filled speaker wedge | `volume` | Toggle; established monitor behavior | `mixer::control` |
| Mixer | Record-enable | Filled red disc | `player-record` / existing red disc | R control remains text; record state red | `mixer::control` |
| Mixer | Copy / paste channel settings | Duplicate rectangles | `copy`, `clipboard` | Existing commands | `mixer::control` |
| Mixer | Routing and channel targets | Procedural routing marks | `route`, `target` | Existing routing controls | `mixer::control` |
| Mixer sidebar | Undo reset, reveal all | Unicode arrow and dots | `arrow-back-up`, `eye` | Existing actions | `mixer.rs` |
| Channel Settings | Power toggles | Custom power ring | `power` | Existing enabled state and click action | `channel_settings.rs` |
| Channel Settings | Preset navigation / section disclosure | Filled triangles | `chevron-down`, `chevron-right` | Text labels and popup actions retained | `channel_settings.rs` |
| Channel Settings | Save layout preset | Procedural floppy drawing | `device-floppy` | Existing save action | `channel_settings.rs` |
| Project inspector sections | Grid, close, monitor, instrument, document, lanes, inserts, EQ, sends, channel controls | Unicode and custom path drawings | `grid-dots`, `x`, `volume`, `music`, `file-description`, `layout-rows`, `plug`, `chart-line`, `send`, `adjustments-horizontal` | Letter controls remain text | `project_inspector.rs` |
| Project toolbar dropdowns | Grid and color menus | Filled triangles | `chevron-down` | Existing text/value selection retained | `app.rs` |
| Sample Editor / event tools | Play, select, range, split, zoom and other shared tools | Mixed procedural icons | Same central semantic Tabler registry as toolbar | Existing actions and tooltips | `sample_editor.rs`, `audio_events.rs` |
| Editing cursor overlay | Split, draw, zoom | Segoe icon-font glyphs | Tabler scissors, pencil, zoom-in | Existing crosshair/hidden-cursor modes and hotspot retained; white outline improves dark/light canvas contrast | `system_tool_icons.rs` |
| Transport panel | Previous, next, cycle, stop, play, record | Local procedural graphics | Same central semantic Tabler registry | Existing states and controls | `transport_panel.rs` |
| Application startup | Brand splash | Old unrelated app mark / no dedicated splash | User-supplied `opentrack-splash.svg` | Shows briefly on normal launch; any click dismisses | `app.rs`, `icon_system.rs` |

## Architecture and state handling

`icon_system.rs` maps semantic IDs to pinned SVG bytes, rasterizes with the
existing egui/resvg stack, and caches textures by semantic ID, physical pixel
size, and tint. The shared toolbar button continues to own hover, pressed,
active, disabled, focus, and tooltip behavior. Enabled toolbar icons use near-black ink on light button faces; standard
outline width is 1.75. The
same renderer is used by non-toolbar controls and cursor overlays. The native
window mark and startup splash are rasterized from their supplied SVG sources
and cached by physical size. The native window icon is rasterized at 256px for
Windows to downsample into its title-bar and taskbar slots. The GitHub banner remains transparent and is
stored at `docs/images/opentrack-github-banner.svg`.

## Review limits

Source and release checks cover the supported display scales. The current
session cannot capture the running Windows desktop. A paired live screenshot
at the supplied 1920 x 1030 reference dimensions is still needed to verify actual
layout, cursor hotspot placement, and the revised menu row. The supplied
screenshots are the functional/layout reference; no concept mockup was used.
