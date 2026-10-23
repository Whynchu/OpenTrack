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

## Current position — M2 in progress

The user accepted the previous M1 review and asked to proceed to L2. The x64 Project and Inspector build now contains original level, pan and track-delay symbols and controls. Per-track pan and delay save in RWP; their playback processing is implemented, but reference pan law and audio parity remain unknown. Opening the earlier RWP, editing these controls, saving and reopening were exercised live through Sky. Current source and screenshot details are in the [M2 review report](../../_analysis-tools/reports/opentrack-comparisons/M2-review-report.md). M2 is incomplete: Inspector spacing and typography, Add Track dialog sizing, enabled states, tooltip hover presentation and other window/track checks remain. Native desktop access works through `node_repl` and `@oai/sky`; use the [remote review workflow](reviews/remote-review-workflow.md). Stop at this M2 review checkpoint; wait for screenshot feedback before M3.

## Active Windows theme and menu fidelity batch

The user rejected the forced steel-blue appearance and requested the Windows app light/dark setting and Windows look. This supersedes the prior fixed palette direction. The layout reference remains the L1 menu inventory, `10-file-menu`, `13-edit-menu` through `43-about-cubase`, and approved arrangement `04-preferred-approved-0.png` in `../_analysis-tools/reports/workflow-L1/`.

The implementation follows Windows theme events through eframe/winit, restores the system preference over persisted app theme overrides, uses installed Segoe UI, and derives custom-drawn surfaces/text from the active palette. Menus inherit that same theme. OS window borders/title bars remain native. egui still draws the application controls; full native widget fidelity, Windows accent color, and high contrast remain separate work. Targeted theme tests cover both palettes and live light/dark/light event transitions. Runtime Windows-settings and screenshot comparison remain pending.

Only `builds/x64/OpenTrack.exe` and `builds/x86/OpenTrack.exe` are current deliverables. `build.ps1` refreshes these paths, using one ignored `.build-cache` for intermediate files. Old `target` build trees, verification outputs, review builds and baseline archives are removed at the user's request. Historical build paths in older review notes are no longer available. x64 remains primary; x86 is an experimental compilation target with no old-OS or hardware acceptance claim.

Cleanup completed after the user closed the old review build. The helper validated and removed the remaining repository `target` tree. Only the two current executables in `builds/x64` and `builds/x86` are deliverables; `.build-cache` is the single hidden intermediate build directory. Both packaged PE architectures and hashes were verified against the completed release builds. All three theme tests and the formatting check pass; actual Windows-setting changes still need visual acceptance.

Acceptance: change Windows Settings > Personalization > Colors > app mode between Light and Dark while OpenTrack is open, inspect File and nested menus, Arrange, Mixer, Channel Settings and MIDI editor for matching theme and readable text, then restart to confirm persisted settings do not force a theme. Also exercise Save/Discard close handling. Pause at this review checkpoint before another substantial UI batch.

## Current position — M3 polish review

User requested polish of the existing pieces, excluding Compressor. The current x64 build has fixed-position Mixer track and Channel Settings controls, footer names, taller racks, and original labelled EQ axes. Live Sky checks exercised F3, channel e and Mute on/off; reference comparisons and remaining gaps are in [M3 polish review](../../_analysis-tools/reports/opentrack-comparisons/M3-polish-review.md). M3 remains in progress: bus strips, routing typography, fader scale, exact bounds, tooltip presentation and floating Transport still need work. Compressor code/DSP were excluded. Stop for screenshot review before the next substantial batch; L4 is untouched.

## Current position — exclusive Mixer fidelity

The user narrowed work exclusively to Mixer. Its measured renderer is now isolated in crates/workstation-app/src/mixer.rs, with corrected fader/meter order, full channel button stacks and two-column sidebar. Live Mixer bounds match(5,579),1881x449. The user accepted the visual direction and requested the e glyph be raised slightly; the final build moves it up1.5px. [Mixer review](../../_analysis-tools/reports/opentrack-comparisons/M3-mixer-focus-review.md) and its97-observation coverage table separate working controls from disabled backend gaps. Channel Settings, Compressor and Transport were excluded; L4 remains untouched. Pause for the updated glyph screenshot review. Keep subsequent work exclusive to Mixer unless the user changes scope.

### Mixer optical centering and feature audit

The latest user correction raises the Mixer e glyph another1px (total offset-2.5px), selected using captured ink bounds. The rebuilt live capture and horizontal comparisons are recorded in [Mixer feature audit](../../_analysis-tools/reports/opentrack-comparisons/M3-mixer-button-features.md). Source audit clarifies that Listen, Monitor, automation R/W, input phase and input gain currently store UI state without audio processing consumers; bus/send backends and several sidebar actions are also missing. Prior UI-toggle checks do not establish those features as implemented. This pass adds no backend functionality and changes no other panels. Pause for the requested screenshot/feature review; scope remains exclusively Mixer.

## Current position — Mixer checkpoint accepted; Project visual pass authorized

The user accepted the Mixer checkpoint and explicitly requested commit/push, then a focused Project-window comparison covering the timeline grid and surrounding layout. This supersedes the exclusive Mixer scope above. Preserve Mixer and Compressor during the Project pass. Use approved L1 frame 04-preferred-approved-0.png and the existing Project tooltip captures; record differences and display conditions outside Git, rebuild and capture the bounded visual slice, then stop for screenshot review. No audio/backend parity or L4 completion is implied.

