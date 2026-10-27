# Historical UI review notes

> This file summarizes earlier feature-specific UI reviews. Current product direction and acceptance criteria are in [workflow/current.md](workflow/current.md) and the [workflow guide](workflow/README.md). The screenshot archive has been removed; future screenshots supplied by the user are task inputs and are not archived by default.

## Working principle

Earlier feature batches used a reference workflow to inform specific UI choices. Their observations remain historical notes; future feature work is specified and reviewed around the intended OpenTrack workflow. Project-format interoperability is tracked separately by format and version.

## Evidence rules

- **Manual:** documented behavior with printed page number; not a runtime measurement.
- **Observed:** a recorded interaction and reference screenshot with environment and state.
- **Implemented:** a source path exists; this does not prove it works on the target PC.
- **Runtime verified:** the acceptance sequence was exercised in OpenTrack with recorded results.
- **Parity verified:** reference and OpenTrack sequences agree at the same display conditions, including persistence and audio where applicable.
- **Unknown:** missing or contradictory evidence. Keep this visible until resolved.

Screenshots supplied for an individual task may be used as design input. Do not archive them or derived comparisons by default.

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

Before editing product behavior, name the user-visible outcome, expected behavior, persistence, relevant failure cases, and acceptance result. During implementation, limit changes to that feature and necessary foundations. Each batch ends with appropriate checks and unresolved limitations. Use screenshots only when supplied for the task; screenshot comparison is not a default acceptance step.

CPR remains a project-interoperability feature. Verify support against real files and round trips for each named version, preserve unknown data where possible, and report limitations.

## Historical L1 checkpoint

The earlier L1 study used user-supplied Cubase screenshots to inventory menu and tooltip behavior. Its screenshot archive and prototype build were later removed. This note is retained as historical context; no capture sweep is active or required. The product behavior and feature verification in [the roadmap](implementation-roadmap.md) and [current workflow](workflow/current.md) take precedence.

## Historical M2 checkpoint

Earlier Project and Inspector work added persisted per-track pan and delay with playback processing. A previous review recorded save/reopen behavior and identified UI and pan-law questions; its screenshot attachments were removed. These historical details do not set current feature requirements.

## Historical Windows theme and menu review

Earlier theme work followed the user's request for Windows light/dark settings and the Windows look. The task-specific screenshots used then were removed; the delivered theme behavior is summarized below.

The implementation follows Windows theme events through eframe/winit, restores the system preference over persisted app theme overrides, uses installed Segoe UI, and derives custom-drawn surfaces/text from the active palette. Menus inherit that theme. OS window borders/title bars remain native. egui still draws the application controls; Windows accent color and high contrast remain separate work. Runtime Windows-setting acceptance was pending in this historical note.

Only `builds/x64/OpenTrack.exe` and `builds/x86/OpenTrack.exe` are current deliverables. `build.ps1` refreshes these paths, using one ignored `.build-cache` for intermediate files. Old `target` build trees, verification outputs, review builds and baseline archives are removed at the user's request. Historical build paths in older review notes are no longer available. x64 remains primary; x86 is an experimental compilation target with no old-OS or hardware acceptance claim.

Cleanup completed after the user closed the old review build. The helper validated and removed the remaining repository `target` tree. Only the two current executables in `builds/x64` and `builds/x86` are deliverables; `.build-cache` is the single hidden intermediate build directory. Both packaged PE architectures and hashes were verified against the completed release builds. All three theme tests and the formatting check pass; actual Windows-setting changes still need visual acceptance.

Acceptance: change Windows Settings > Personalization > Colors > app mode between Light and Dark while OpenTrack is open, inspect File and nested menus, Arrange, Mixer, Channel Settings and MIDI editor for matching theme and readable text, then restart to confirm persisted settings do not force a theme. Also exercise Save/Discard close handling. Pause at this review checkpoint before another substantial UI batch.

## Historical M3 polish review

An earlier Mixer/Channel Settings polish pass added fixed-position controls, footer names, taller racks, and labelled EQ axes. The historical [M3 review](../../_analysis-tools/reports/opentrack-comparisons/M3-polish-review.md) records behavior checks and gaps at that time. Compressor was excluded from that pass; this checkpoint is no longer an active scope gate.

## Historical Mixer review

The earlier Mixer pass adjusted fader/meter order, channel button stacks, sidebar layout, and the e glyph. Its [review notes](../../_analysis-tools/reports/opentrack-comparisons/M3-mixer-focus-review.md) retain the behavior audit and backend gaps. The old exclusive-Mixer scope and screenshot review checkpoint are historical only.

### Mixer feature audit

The earlier Mixer feature audit found that Listen, Monitor, automation R/W, input phase, and input gain stored UI state without audio-processing consumers; bus/send backends and several sidebar actions were also missing. Preserve this as an implementation finding: UI toggles alone do not establish feature behavior. The [text review](../../_analysis-tools/reports/opentrack-comparisons/M3-mixer-button-features.md) remains available; its screenshot attachments were removed.

## Historical Project review checkpoint

An earlier review moved from Mixer to a focused Project-window pass. Its implementation history is retained in the linked task reports; screenshot-based comparisons and capture gates are not current acceptance criteria.


## Project visual review checkpoint

An earlier Project pass replaced Inspector grouping with original vector controls, corrected the ruler/track boundary, and refined the flow-based toolbar. Mute and Record Enable UI synchronization were exercised. Historical visual gaps included lower navigation and toolbar alignment; the screenshot attachments were removed and those gaps do not set current acceptance.


## Live Project interaction evidence

An earlier Project interaction study recorded the right-edge track color strip, the separate below-tracks Track List context menu, and ruler-drag behavior. Screenshot attachments and capture manifests were removed; the [text study](reviews/project-interaction-study.md) retains the feature observations and open questions.



## Inspector and Project header audit

An earlier [Project layout audit](reviews/project-layout-audit.md) records observations about the left inset, parameter wells, label contrast, Inspector grouping, and toolbar organization. Its screenshot attachments were removed; the written findings remain historical design context.

## Historical desktop-capture limitation

Earlier desktop review was limited by window-enumeration problems. That tool setup note is historical and does not affect current feature acceptance.
## 2026-10-24 recovery and single-instance rule

Earlier review notes also recorded a desktop-tool recovery procedure. It is retained only as historical context; current desktop work follows the applicable task and user instructions.
