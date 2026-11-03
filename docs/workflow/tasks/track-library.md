# File > New/Open/Save Library: track-template library

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: File > Save Library stores the selected track's kind, insert rack and portable routing as a named template in application settings; File > New Library and File > Open Library open one window listing the templates, where each can be added to the project or deleted.
- Reference evidence: `LibraryTrack`, `sanitize_library`, `portable_input_route`, `portable_output_route`, `save_selected_track_to_library`, `add_library_track`, `show_track_library` and the tests in `crates/opentrack-app/src/app/track_library.rs`; storage key `track-library-v1`.
- Dependencies: none.

## Model

- OpenTrack keeps **one application-wide library** rather than a file per library; that is why New and Open both show the same window, which the window states.
- A template stores: name, track kind, audio channel width, the insert rack (up to 16) and input/output routing **only when it is portable**. Group and FX routes name project ids, so they are dropped on save and load instead of pointing at a channel that may not exist.
- Loading sanitises: trimmed non-empty unique names, ≤100 templates, ≤16 inserts, width clamped to 1–2, non-portable routes removed.
- Saving uses the track name, suffixed (`Lead Guitar 2`) when the library already has it. Adding to the project also makes the new track's name unique.
- Adding is one labelled **"Add Track from Library"** undo step through the ordinary history, plan rebuild and modify path.
- A stale index, an invalid result and a full library report without editing.

## Verification

- `cargo test -p opentrack track_library` -> 4 passed: save captures kind/inserts and drops a `Group:7` output while keeping `Stereo In`; adding restores inserts and `Stereo Out`, names the track uniquely and writes the undo step; sanitising trims/caps/de-duplicates and strips non-portable routes; the window lists the template, its insert count and `out Stereo Out`.
- Inventory regenerated: 277 leaves, **52 available**, 174 state-dependent, **43 unavailable**, 8 informational (was 46).
- App suite 612 passed / 4 known device-dependent roll failures / 9 ignored (625 tests).

## Not claimed

No desktop pass over the window. Insert state is copied, not re-validated per plug-in; a template saved from a work session with a third-party plug-in still depends on that plug-in being installed.
