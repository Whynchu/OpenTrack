# Window and loading-screen layout audit

- Status: accepted for source and headless layout and geometry checks; desktop review pending
- Scope: review of the windows and loading surfaces added in the post-revision-5 work
- Desired observable outcome: each new window opens at a usable size, stays inside a 640×480 screen, keeps its painted content inside its own frame, and the loading screen is centered and reports progress.
- Reference evidence: `crates/opentrack-app/src/app/window_layout_tests.rs`; the window modules listed below.

## Surfaces checked

| Surface | Default / min size | Organisation |
| --- | --- | --- |
| Tempo Track | 340 min | two-column grid (tempo, beats/bar, beat unit), muted explanation |
| Event Colors | 360 min | palette row, colour edit + RGB readout, wrapped 16-slot swatches, action row |
| Direct Offline Processing | 460×320 | per-processor rows with one numeric control, reorder/remove, add/save/apply row |
| Windows list | 340 min | checkbox rows with a Focus button |
| Revert / Cleanup | 340 / 400 min | confirmation text with a short button row |
| Registration | 380 min | left-aligned informational text, no link |
| Plug-in Browser | 440 min | target line, search, scrollable effect rows with Insert |
| VST Connections | 560 min | **grids** for device buses (bus/width/device/users) and project buses (channel/kind/width/output/sends) |
| Macros | 520 min | macro selector + Create/Delete, step list, searchable command scroll |
| Track Library | 520 min | Save Selected Track row, template list with Add/Delete |
| Browse Sounds | 560 min | Choose Folder row, search, file scroll, selected-file format line, Import |
| Control Room | 480 min | Mixer/Overview tabs, listen picker, talkback, readout |
| Project Synchronization Setup | 440 min | **grid** for MIDI Output + Refresh and MMC device ID |
| Score | 600×400 | header counts, **wrapping** toolbar, horizontal+vertical scroll canvas |
| Score Layout / Settings | 320 / 300 min | two-column grids for bars/zoom/page and zoom |
| Video Window | 600×400 | Open row, status, **centered** image fitted to the available width/height |
| Loading Project | 360 min, 340 bar | **centered** file name, progress bar with a **centered** percentage, phase text |

## Geometry checks

`window_layout_tests` reads each window's area rect from egui memory after four
frames and asserts that it lies inside the screen (1 px tolerance). Every
painted text shape is clipped to its paint clip rect (so scroll-area rows that
are laid out but not visible do not count) and must lie inside the window.
Measured at **640×480** (identical at 1100×780 because the defaults fit both):

| Window | Rect | Size |
| --- | --- | --- |
| Tempo Track | (16,16)–(370,214) | 354×198 |
| Event Colors | (16,16)–(390,291) | 374×275 |
| Direct Offline Processing | (16,16)–(476,167) | 460×151 |
| Windows | (16,16)–(370,281) | 354×265 |
| Revert | (16,16)–(370,119) | 354×103 |
| Registration | (16,16)–(410,189) | 394×173 |
| Plug-in Browser | (16,16)–(470,148) | 454×132 |
| Clean Up Project Folder | (16,16)–(430,149) | 414×133 |
| VST Connections | (16,16)–(590,246) | 574×230 |
| Macros | (16,16)–(576,422) | 560×406 |
| Track Library | (16,16)–(576,151) | 560×135 |
| Browse Sounds | (16,16)–(616,197) | 600×181 |
| Control Room | (16,16)–(536,216) | 520×200 |
| Project Synchronization Setup | (16,16)–(470,238) | 454×222 |
| Score | (16,16)–(616,416) | 600×400 |
| Score Layout | (16,16)–(356,162) | 340×146 |
| Score Settings | (16,16)–(356,134) | 340×118 |
| Video Window (no file) | (16,16)–(616,146) | 600×130 |
| Video Window (1280×720 frame) | (16,16)–(616,404) | 600×388 |

The widest window is 616 px and the tallest 422 px, so all fit a 640×480 screen.

Centering:

| Check | Measured |
| --- | --- |
| Loading window at 1100×780 | rect (363,331)–(737,449), center (550,390) = screen center |
| Loading window at 640×480 | rect (133,181)–(507,299), center (320,240) = screen center |
| Loading window at 1920×1080 | rect (773,481)–(1147,599), center (960,540) = screen center |
| Loading text column center | within 2 px of the window center |
| Video frame image, 640×480 | rect (75.1,126)–(556.9,397), center_x 316 = window center_x 316; 482×271 inside a 600×388 window |

## Fixes made

- VST Connections used ad-hoc horizontal rows; both sections are now striped grids so columns line up.
- The Video Window sized the image to a fixed 320 px and left-aligned it; it now fits the available width **and height** and is horizontally centered.
- The loading window was centered but its labels were left-aligned and the bar had no percentage; it is now a centered column, and the percentage is painted **centered on the bar** (egui draws its own bar label at the left edge).
- The Windows list, Plug-in Browser, Track Library, Browse Sounds, Control Room, Video Window and Sync Setup gained explicit minimum widths so their columns do not collapse when the screen is small.
- Sync Setup's output picker and MMC device id moved into a two-column grid.
- **Score** defaulted to 820×420 and its single-row toolbar forced the window to 746 px wide at 640×480; the default is now **600×400** and the toolbar uses `horizontal_wrapped`, so it wraps instead of widening the window.
- **Video Window** defaulted to 660×420; it is now 600×400 so it opens fully on-screen.
- The geometry test counts the **painted (clipped)** part of a shape. Before that, the Macros command list reported a row 7 px below the frame even though the scroll area clipped it; the painted extent now ends at the scroll viewport.

## Verification

- `cargo test -p opentrack window_layout_tests` -> **3 passed**:
  - `the_new_dialogs_open_with_their_controls_at_desktop_and_small_sizes`: every window rendered at 1100×780 and 640×480 with its title and a representative control, its rect inside the screen, and all painted text inside the window.
  - `the_project_loading_screen_is_centered_and_reports_progress`: at 1100×780, 640×480 and 1920×1080 the window center equals the screen center within 1 px, the body column is centered within 2 px, and the text includes `Loading Project`, the file name, `65%` and the phase.
  - `the_video_image_is_centered_and_fitted_in_its_window`: a 1280×720 texture is painted, horizontally centered within 2 px of the window center, inside the window, and no larger than the window.
- `cargo test -p opentrack` -> 645 passed / 4 known device-dependent roll failures / 9 ignored (658 tests); `opentrack-core` 171; `opentrack-audio` 175 / 3 ignored; `cargo fmt --all --check` clean.
- Inventory unchanged at 277 leaves (63 available, 189 state-dependent, 17 unavailable, 8 informational; source line numbers refreshed).

## Not claimed

Headless rendering proves the layout code runs, the rects fit the screen, the painted text stays inside its window and the two centering cases hold at three sizes. Font metrics, theme contrast, DPI scaling and the visual result still need the desktop review. No meters were added where the app has none (audio import and archive import report through the status line, not a progress window).
