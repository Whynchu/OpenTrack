# Revert and Start Record at Left Locator

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: File > Revert reloads the saved project only after confirming that unsaved edits are discarded; Transport > Start Record at Left Locator moves the transport to the left locator and starts recording there.
- Reference evidence: `request_revert_project`, `revert_project_now`, `show_revert_prompt`, `start_record_at_left_locator` and `revert_and_record_locator_tests` in `crates/opentrack-app/src/app.rs`.
- Dependencies: none.
- Code scope: one confirmation dialog, one transport command, two menu rows.

## Revert

- Without a project or without a saved project path the command reports instead of guessing.
- An unmodified project reloads immediately; a modified project shows "Discard the unsaved changes and reload <name> from disk?" with Revert/Cancel, and nothing is reloaded until Revert is chosen.
- Confirming goes straight to the ordinary background load (`begin_project_load`), so the unsaved-changes prompt is not shown twice; the load clears history and marks the project saved, as any open does.
- The prompt joins `modal_dialog_open`, so its keyboard does not leak to the arrange.

## Start Record at Left Locator

- Refuses while recording, seeks to the left locator, then runs the ordinary record toggle, so an unarmed or device-less project reports through the existing messages.
- The left locator is clamped at beat 0.

## Verification

- `cargo test -p opentrack revert_and_record_locator` -> 2 passed: the confirmation gate (edits kept until confirm, background load started on confirm), the no-project report, and the transport moving to the left locator.
- Inventory regenerated: 277 leaves, 47 available, **163 state-dependent, 59 unavailable**, 8 informational.
- App suite 592 passed / 4 known device-dependent roll failures / 9 ignored (605 tests).

## Not claimed

The audible result of recording at the locator needs a device; the revert prompt layout needs the desktop.
