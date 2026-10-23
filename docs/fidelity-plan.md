# Fidelity delivery plan

## Working principle

Observe a complete Cubase 5 workflow, document its visible states and behavior, reproduce it in original Rust code and graphics, then compare it before expanding it. The full product goal remains unchanged, including CPR investigation and optional legacy plug-in hosting.

## Evidence rules

- **Manual:** documented behavior with printed page number; not a runtime measurement.
- **Observed:** a recorded interaction and reference screenshot with environment and state.
- **Implemented:** a source path exists; this does not prove it works on the target PC.
- **Runtime verified:** the acceptance sequence was exercised in OpenTrack with recorded results.
- **Parity verified:** reference and OpenTrack sequences agree at the same display conditions, including persistence and audio where applicable.
- **Unknown:** missing or contradictory evidence. Keep this visible until resolved.

Reference screenshots are analysis material in `../_analysis-tools/reports/workflow-L1/`, not product graphics. Store source citations and relative capture paths in the workflow specification. Record logical/physical display dimensions and scaling separately; do not infer DPI from image size.

## Review landmarks

| Landmark | Deliverable | User involvement | Exit condition |
| --- | --- | --- | --- |
| L1: reference and baseline | Preserved baseline build/source, basic-track workflow specification, captures and mismatch list | Confirm the reference layout and priority order | Core reference states captured; unknowns identified; user accepts the first UI scope |
| L2: Project window and track flow | Review build matching Project structure, Inspector, track creation/selection/rename/drag order/resize and track controls | Try the basic track sequence and flag layout/interaction mismatches | Visual comparison and workflow results recorded; user accepts or supplies corrections |
| L3: Channel Settings and Mixer | Review build with reference-driven window reuse/focus, channel selection, EQ and insert interaction | Compare settings flow and window setup | Corresponding states and parameter persistence agree; remaining DSP differences explicit |
| L4: recording through recall | Audio/MIDI recording, playback, native save/reopen and workspace recall acceptance | Exercise real audio/MIDI devices and preferred workspace | Target-PC results recorded; known limitations accepted for the milestone |
| L5+: expanded workflows | Event editing, routing, automation, MIDI tools and plug-in workflows in complete sequences | Review at the end of each substantial workflow | Same observation/specification/implementation/comparison cycle |

Pause at each landmark before the next substantial UI batch. A review pause preserves the full objective and current artifacts. Resume from user feedback, not from an assumed approval.

## Batch definition

Before editing product behavior, name the workflow step IDs, reference captures/manual pages, current mismatch, and acceptance result needed. During implementation, limit changes to those steps and necessary foundations. Each batch ends with a named build, appropriate checks, comparison notes, and unresolved mismatches. Keep fresh graphics while matching structure, placement, density, hierarchy, focus, shortcuts, and window behavior.

Do not expand feature breadth to avoid a difficult fidelity mismatch. CPR and legacy hosting remain separate evidence-backed investigation tracks; they must not silently redefine the first workflow's acceptance criteria or imply CPR compatibility.

## Current position

The user accepted the expanded L1 reference capture as a sound basis and asked for a systematic tooltip sweep before implementation. The ordered capture ledger is [tooltip-sweep-log.md](reviews/tooltip-sweep-log.md). A separate Rust helper under `_analysis-tools/cubase-hover-mcp/` adds the hover-only MCP action. The user reported that its original `SetCursorPos` movement did not move the visible cursor reliably. A corrected `SendInput` build now verifies cursor position, and the saved Codex config points at it; the currently loaded MCP server still holds the old binary and must be reloaded before capture resumes. The user clarified that top-level menu labels have no tooltips, so the ordered tooltip sweep begins at the Project window power-symbol button. Automated movement and tooltip visibility remain unverified. The original prototype was archived under `target/review-baselines/L0-2026-10-22/`; that build archive was removed when the user requested deletion of all old build folders. Reference captures and source documentation remain outside build output. No product UI changes had been made during that capture pass. See [basic-track-workflow.md](basic-track-workflow.md) and [the L1 capture review](reviews/L1-menu-and-project-capture.md) for the specification, evidence and remaining unknowns.

## Active Windows theme and menu fidelity batch

The user rejected the forced steel-blue appearance and requested the Windows app light/dark setting and Windows look. This supersedes the prior fixed palette direction. The layout reference remains the L1 menu inventory, `10-file-menu`, `13-edit-menu` through `43-about-cubase`, and approved arrangement `04-preferred-approved-0.png` in `../_analysis-tools/reports/workflow-L1/`.

The implementation follows Windows theme events through eframe/winit, restores the system preference over persisted app theme overrides, uses installed Segoe UI, and derives custom-drawn surfaces/text from the active palette. Menus inherit that same theme. OS window borders/title bars remain native. egui still draws the application controls; full native widget fidelity, Windows accent color, and high contrast remain separate work. Targeted theme tests cover both palettes and live light/dark/light event transitions. Runtime Windows-settings and screenshot comparison remain pending.

Only `builds/x64/OpenTrack.exe` and `builds/x86/OpenTrack.exe` are current deliverables. `build.ps1` refreshes these paths, using one ignored `.build-cache` for intermediate files. Old `target` build trees, verification outputs, review builds and baseline archives are removed at the user's request. Historical build paths in older review notes are no longer available. x64 remains primary; x86 is an experimental compilation target with no old-OS or hardware acceptance claim.

Cleanup completed after the user closed the old review build. The helper validated and removed the remaining repository `target` tree. Only the two current executables in `builds/x64` and `builds/x86` are deliverables; `.build-cache` is the single hidden intermediate build directory. Both packaged PE architectures and hashes were verified against the completed release builds. All three theme tests and the formatting check pass; actual Windows-setting changes still need visual acceptance.

Acceptance: change Windows Settings > Personalization > Colors > app mode between Light and Dark while OpenTrack is open, inspect File and nested menus, Arrange, Mixer, Channel Settings and MIDI editor for matching theme and readable text, then restart to confirm persisted settings do not force a theme. Also exercise Save/Discard close handling. Pause at this review checkpoint before another substantial UI batch.
