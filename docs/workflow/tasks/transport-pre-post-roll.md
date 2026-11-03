# Use Pre-roll and Use Post-roll

- Status: accepted for source and automated checks; audible review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: the two Transport rows are persisted checkboxes that make plain playback start before the left locator and stop after the right locator, without changing the explicit roll commands or "play from selection".
- Reference evidence: `use_pre_roll`, `use_post_roll`, `play_with_roll` and `transport_roll_tests` in `crates/opentrack-app/src/app.rs`; the roll lengths in `transport_commands`.
- Dependencies: none (reuses the existing roll lengths and `play_until`).

## Behaviour

- Both preferences default off, are stored under `transport-use-pre-roll` / `transport-use-post-roll`, and are shown as checkable Transport rows with a status line naming the current roll length.
- **Pre-roll:** if the cursor is inside the locator range, plain Play starts `pre-roll beats` before the left locator, clamped at beat 0.
- **Post-roll:** plain Play in the same range sets a stop target `post-roll beats` after the right locator through the existing `play_until` check, so stopping at the target and manual-stop clearing behave as they already do.
- Only the plain Play paths use this (the `transport.play` command, the Play toolbar button). "Play from selection", the roll commands and recording keep their own position and stop targets.

## Verification

- `cargo test -p opentrack transport_roll_tests` -> passed: off by default, pre-roll moves to 4 - 2 = beat 2 and post-roll stops at 8 + 3 = beat 11, a cursor after the locators gets neither, and a long pre-roll clamps at beat 0.
- Inventory regenerated: 277 leaves, **49 available, 164 state-dependent, 56 unavailable**, 8 informational.
- App suite 595 passed / 4 known device-dependent roll failures / 9 ignored (608 tests).

## Not claimed

No listening check of the lead-in or the stop point; the transport also needs an audio device to actually play, which this host does not provide.
