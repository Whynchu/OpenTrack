# Window list and Recent Projects

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: Window > Windows... lists the app's windows and can open, close and focus them; File > Recent Projects lists the last ten opened or saved projects and reopens them.
- Reference evidence: `WINDOW_NAMES`, `window_is_open`, `set_window_open`, `focus_window`, `show_windows_dialog`, `remember_recent_path`, `remember_recent_project` and the `window_list_tests` / `recent_project_tests` modules in `crates/opentrack-app/src/app.rs`.
- Dependencies and decisions: none.
- Code scope: one dialog, one persisted list, two one-line hooks on successful load and save.

## Window list

- Lists Project, Mixer, Mixer 2, Mixer 3, Channel Settings, Media and Spectrum Analyzer with a checkbox per row and a Focus button for open ones.
- Project, Mixer, Channel Settings and the extra mixers only open with a project; the Media window and analyzer do not need one.
- Closing the dialog is the same as any other window; it joins `modal_dialog_open`, so its keyboard state does not leak to the arrange.

## Recent Projects

- The newest ten paths are persisted as `recent-projects-v1` in application storage and shown by file stem, with the full path on hover.
- Opening or saving a project moves it to the front; an existing entry is moved rather than duplicated, and the list is truncated to ten.
- A missing or invalid recent path still goes through the ordinary open path, so it reports "Could not open project" and is not re-remembered.
- "Clear Recent Projects" empties the list; an empty list shows the existing "No Recent Projects" row.

## Verification

- `cargo test -p opentrack window_list_tests` -> 2 passed (open/close/focus for each window, project-bound refusal, every name drawn).
- `cargo test -p opentrack recent_project` -> 2 passed (dedupe/move-to-front/cap, app-level remember, failed open not remembered).
- Inventory regenerated: 277 leaves, **47 available, 161 state-dependent, 61 unavailable**, 8 informational.
- App suite 590 passed / 4 known device-dependent roll failures / 9 ignored (603 tests).

## Not claimed

No desktop click-through of the dialog or the recent list; persistence is covered by the storage key plus the list logic, not by a restart test.
