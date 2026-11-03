# Edit > Macros

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: Edit > Macros opens one window where named macros hold an ordered list of fixed command IDs and run them through the ordinary dispatch path.
- Reference evidence: `macros::Macro`, `sanitize_macros`, `replayable_command`, `create_macro`, `add_macro_step`, `run_macro`, `show_macros` and the tests in `crates/opentrack-app/src/app/macros.rs`; the persisted key `command-macros-v1`.
- Dependencies: none. Reuses the command catalog and `dispatch_reference_command`.

## Behaviour

- Macros are stored in application settings as `command-macros-v1` (`name` + ordered command IDs). Loading sanitises them: trimmed non-empty names, at most 32 macros, 32 steps each, 64 characters per name, and the first of any duplicate name wins.
- One Edit menu entry opens the window: a macro selector, Create/Delete, the selected macro's step list with move-up/move-down/remove, a Run button, and a searchable command list to append steps.
- Steps are chosen from the keymap catalog, so a macro can only contain commands the app actually dispatches. Marker-slot commands (`transport.set_marker_*`, `to_marker_*`, `recall_cycle_marker_*`) are excluded because they read the digit from the physical chord; the add and run paths both refuse them.
- Running calls `dispatch_reference_command` for each step in order with a neutral chord, then reports `Ran macro '<name>' (N command(s))`. An empty macro and a run without a project report instead of pretending.
- Create refuses blank and duplicate names; the step list is bounded to 32.

## Verification

- `cargo test -p opentrack macros` -> 3 passed: sanitising (trim, caps, duplicate names, step cap) and the replayable-command rule; create/add/refuse/reorder/remove plus running a one-step macro that flips the auto-select flag through dispatch; and the window drawing its title, macro name and `1. Cycle` step.
- Inventory regenerated: 277 leaves, 50 available, **173 state-dependent, 46 unavailable**, 8 informational.
- App suite 608 passed / 4 known device-dependent roll failures / 9 ignored (621 tests).

## Not claimed

No desktop pass over the window, and macros are not recorded from the user's actions — steps are added from the command list by design. Parameters are not captured, so a macro repeats commands, not a specific edit value.
