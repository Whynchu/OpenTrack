# Window and loading-screen layout audit

- Status: accepted for source and headless layout checks; desktop review pending
- Scope: review of the windows and loading surfaces added in the post-revision-5 work
- Desired observable outcome: each new window opens at a usable size, keeps its controls aligned and visible on a small screen, and the loading screen is centered and reports progress.
- Reference evidence: `crates/opentrack-app/src/app/window_layout_tests.rs`; the window modules listed below.

## Surfaces checked

| Surface | Size / min width | Organisation |
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
| Score | 820×420 | header counts, toolbar, horizontal+vertical scroll canvas |
| Score Layout / Settings | 320 / 300 min | two-column grids for bars/zoom/page and zoom |
| Video Window | 480 min | Open row, status, **centered** image fitted to the available width/height |
| Loading Project | 360 min, 340 bar | **centered** file name, progress bar with percentage, phase text |

## Fixes made

- VST Connections used ad-hoc horizontal rows; both sections are now striped grids so columns line up.
- The Video Window sized the image to a fixed 320 px and left-aligned it; it now fits the available height and is horizontally centered.
- The loading window was centered but the label text was left-aligned and the bar had no percentage; it now uses a centered column with a `NN%` progress text.
- The Windows list, Plug-in Browser, Track Library, Browse Sounds, Control Room, Video Window and Sync Setup gained explicit minimum widths so their columns do not collapse when the screen is small.
- Sync Setup's output picker and MMC device id moved into a two-column grid.

## Verification

- `cargo test -p opentrack window_layout_tests` -> 2 passed: every new window renders at **1100×780 and 640×480** with its title and a representative control present, and the loading screen shows `Loading Project`, the file name, `65%` and its phase.
- `cargo test -p opentrack` -> 644 passed / 4 known device-dependent roll failures / 9 ignored (657 tests); inventory unchanged at 277 leaves (63 available, 189 state-dependent, 17 unavailable, 8 informational).

## Not claimed

Headless rendering proves layout code runs and key labels are produced at both sizes; pixel alignment, clipping at unusual DPI and the actual look still need the desktop review. No meters were added where the app has none (audio import and archive import report through the status line, not a progress window).
