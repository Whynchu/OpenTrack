# Menu wiring batch: surfaces that already existed

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work (user direction: keep filling the missing features)
- Desired observable outcome: six menu rows that named a feature OpenTrack already had now open it, instead of showing as disabled.
- Reference evidence: `open_event_history`, `open_spectrum_analyzer`, `open_media_pool_window`, `open_midi_device_manager`, `begin_save_as_template` and the `wired_menu_row_tests` module in `crates/opentrack-app/src/app.rs`; the regenerated inventory.
- Dependencies and decisions: none.
- Code scope: menu rows plus one small method each; no new dialogs and no schema change.

## Rows wired

| Row | Surface it now opens |
| --- | --- |
| Edit > History... | the existing undo-history window (`undo_history_open`) |
| Audio > Spectrum Analyzer | the standalone analyzer (`spectrum_open`) |
| Media > Open Pool Window (Ctrl+P) | the Media window, focused (`focus_tab(PanelTab::Media)`) |
| Media > Import Audio CD... | the existing CUE-sheet import |
| Devices > MIDI Device Manager | Device Setup on the MIDI port page |
| File > Save as Template... | the existing Save as Template dialog with the project name |

Each row calls a small method rather than setting fields inline, so the wiring is testable without simulating pointer events.

## Verification

- `cargo test -p opentrack the_wired_rows` -> passed: every method reaches its surface, and the two project-only actions report instead of opening when no project is loaded.
- Inventory regenerated: 275 leaves, **44 available, 161 state-dependent, 62 unavailable**, 8 informational (was 41/158/68).
- App suite 586 passed / 4 known device-dependent roll failures / 9 ignored (599 tests); core 167; audio 172.
- README disabled-row count updated from 69 to 62.

## Not claimed

No desktop click-through of the six rows; the test exercises the methods the rows call, not the popup itself.
