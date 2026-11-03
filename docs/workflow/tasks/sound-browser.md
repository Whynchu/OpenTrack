# Project > Add Track > Browse Sounds

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: Browse Sounds opens a window over one persisted folder, lists its WAV files, shows the selected file's format, and imports it through the ordinary WAV import path onto the selected audio track.
- Reference evidence: `scan_sound_folder`, `sound_file_info`, `rescan_sound_folder`, `import_selected_sound` and the tests in `crates/opentrack-app/src/app/sound_browser.rs`; `import_audio_path` (factored from the dialog import); storage key `sound-browser-folder-v1`.
- Dependencies: none.

## Behaviour

- One persisted folder (`sound-browser-folder-v1`), chosen with **Choose Folder...**; the window lists `.wav`/`.wave` files from it and its subfolders (depth 4, at most 2,000 entries, sorted by name) with a file-name search.
- Selecting a file shows its format: duration, sample rate, channels, bit depth and size, read with the existing WAV metadata reader. **No audio audition** — the window says the preview is the format, and nothing pretends to play.
- **Import Selected** runs the same pipeline as File > Import > Audio: it copies the file into the project's pool folder when it lives outside the project, opens it, places a clip at the cursor on the selected audio track (or a new audio track), registers it in the media pool and rebuilds the plan. The dialog import and the browser now share `import_audio_path`.
- Refusals come from the import path: no project, an import already running, or an unmanaged project folder. Unreadable WAVs are listed but report their metadata error instead of a fake preview.

## Verification

- `cargo test -p opentrack sound_browser` -> 5 passed: the scan (WAV only, nested, sorted, filtered, empty folder), format readout for a real WAV and an error for a non-WAV, opening/refusing without a ready project, the window drawing the title/file/`48000 Hz`, and an end-to-end import that copies an outside WAV into `Audio/Imports`, places one clip on the selected track and adds it to the media pool.
- Inventory regenerated: 277 leaves, **53 available**, 174 state-dependent, **42 unavailable**, 8 informational.
- App suite 617 passed / 4 known device-dependent roll failures / 9 ignored (630 tests).

## Not claimed

No desktop review of the window, and no audio audition (by design). MP3/FLAC/AIFF are not listed because the importer only reads WAV.
