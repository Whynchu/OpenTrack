# Standalone plug-in browser

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: Audio > Plug-ins > Reverb/Dynamics/EQ/Filter/Distortion/Modulation/Other each open a browser filtered to that insert family, and choosing an effect appends it to the selected track's insert rack as one undo step.
- Reference evidence: `open_plugin_browser`, `plugin_browser_target`, `plugin_browser_plugins`, `insert_catalog_plugin`, `show_plugin_browser` and the browser tests in `plugin_catalog_tests` (`crates/opentrack-app/src/app.rs`); `catalog_plugins_for_category` / `vst3_insert_effect` for the catalog and insert-kind mapping.
- Dependencies: none. Reuses the scanned VST3 catalog, the Channel Settings category mapping and the existing VST3 runtime sync.

## Behaviour

- Each menu row opens one window titled with its family; the rows stay explicit `menu_command` calls so the generated inventory keeps tracking them.
- The window shows the target ("Target: <track> — new insert slot N") from the selected track, or "Select a track to insert into this rack" and disabled Insert buttons when nothing is selected.
- A search field filters by name or vendor within the family; an empty family says to scan plug-ins in Device Setup.
- Insert appends an `InsertEffect` with the catalog's class id, module path, name and vendor, enabled and not bypassed, and runs the ordinary history, plan-rebuild and modify path. The status names the track and slot.
- Refusals: no selected track, a class id that is no longer in the catalog, a track already holding 16 inserts, and a project that would fail validation. None of them edit the project.

## Verification

- `cargo test -p opentrack plugin_catalog_tests` -> 6 passed, including the new browser tests: family filtering and search, insert as slot 1 then slot 2 with the "Insert Plug-in" history label, no-selection and stale-catalog refusals, the 16-slot bound, and the window drawing its title, target and effect names.
- Inventory regenerated: 277 leaves, 50 available, **171 state-dependent, 48 unavailable**, 8 informational (was 55 unavailable).
- App suite 598 passed / 4 known device-dependent roll failures / 9 ignored (611 tests).

## Not claimed

No third-party plug-in was loaded in this environment, so the VST3 editor/audio path is not verified here; the test covers catalog selection and insert bookkeeping. The window layout needs a desktop pass.
