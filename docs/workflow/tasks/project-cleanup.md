# File > Cleanup: reversible project-folder cleanup

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: File > Cleanup scans the managed project's `Audio` folder, lists files no pool entry or clip refers to, and moves them to a dated trash folder inside the project after confirmation, reporting counts. Nothing is deleted.
- Reference evidence: `referenced_media`, `scan_project_audio`, `execute_cleanup`, `request_project_cleanup`, `execute_project_cleanup`, `show_cleanup_prompt` and `project_cleanup` tests in `crates/opentrack-app/src/app/project_cleanup.rs`.
- Dependencies: none.

## Behaviour

- Refuses without a project, without a managed project folder, or without a saved project path.
- References are the media pool plus every clip's source path, compared in canonical form; only files under `Audio/` are considered, so the project file, `Metadata/` (state, presets, recovery) and anything outside the media folder are never touched.
- A plan is computed first and the command reports "Cleanup ready: N file(s), X MB"; nothing moves until the confirmation dialog's **Clean Up**.
- Files keep their path relative to the project root under `Metadata/Cleanup Trash/<YYYY-MM-DD>/`, so restoring one is a plain move back. A second cleanup on the same day uses `… <date> 2`.
- The date is derived from the system clock with a civil-from-days conversion (no new dependency); a failed move stops the pass and reports how many files already moved.
- An empty plan reports "No unreferenced files" instead of opening a dialog.

## Verification

- `cargo test -p opentrack project_cleanup` -> 4 passed: only the unreferenced audio file moves while referenced media, the project file and metadata stay; the file's relative path is preserved and moving it back restores the scan; a second cleanup uses a new trash folder; 1970-01-01, 1970-01-02 and 2026-09-21 date labels; unsaved/unmanaged/no-project refusals; the command plans, waits for confirmation, moves, and reports an empty second pass.
- Inventory regenerated: 277 leaves, 50 available, **172 state-dependent, 47 unavailable**, 8 informational.
- App suite 602 passed / 4 known device-dependent roll failures / 9 ignored (615 tests).

## Not claimed

No desktop interaction with the dialog; the tests call the same plan/move functions the dialog uses.
