# L1 review: baseline and reference workflow

Date: 2026-10-22. Status: L1 reference and preferred-layout review checkpoint. The menu, Project-window and Channel-Settings capture is expanded; full workflow capture and user review remain open.

## Preserved baseline

`target/review-baselines/L0-2026-10-22/` contains the existing executable and a source archive made before this fidelity-plan change. The repository has no commits, so this archive is the immutable comparison point for the current pass; the normal working tree continues separately.

| Artifact | SHA-256 |
| --- | --- |
| opentrack.exe | 6530F685677DC4E7CBB01E966AB13B1A0548BA388DA16C67E0EAE2CD613020EF |
| source.zip | E34553F3F114AE2B7F78DAB67EED818EECE4EA7D999850378416DA926F362BE8 |

The baseline is an x64 development build. Prior compilation is not runtime or parity acceptance. No new product behavior was changed in this evidence pass.

## Findings that change implementation order

The captured reference uses an application workspace containing a Project child window and Mixer child window, with a separate Transport pane. OpenTrack currently uses a docked central view plus native secondary viewports. The correct window architecture needs evaluation before further visual refinements.

The source audit and manual comparison identify absent Inspector, single-track selection only, append-only track creation, menu-only track reorder, no per-track monitor button, simplified New Project, and a Channel Settings form that differs structurally from the documented default window. These are now explicit workflow gaps, not small styling tasks.

The Solo manual citation was independently checked and corrected from printed p.118 to p.119. Channel Settings citations pp.122-125 were confirmed.

Luna completed two bounded read-only assignments: source workflow audit at high effort and manual extraction at medium effort, followed by window-host option analysis. The parent reviewed the findings against actual captures. The host proposal is recorded in [workspace-host-decision.md](../workspace-host-decision.md); no framework or native-host change has been made.

## Review requested

The user supplied their preferred layout and a clean capture was archived as `../_analysis-tools/reports/workflow-L1/04-preferred-approved-0.png`, SHA-256 `daef9e9ab5121eedeb1421ef429769a0e5d8e05131a197829d92bb753b4e78cf`. Transport and Compressor have separate bounded captures and a provenance manifest. This establishes the layout target; it does not verify interaction sequences or OpenTrack parity.

Confirm that the installed Cubase 5 layout and basic track workflow are the right reference. Proposed next UI scope: application/Project window hierarchy, Inspector and Track List, track creation and selection, drag reorder/resize, and M/S/R/monitor flow. Channel Settings and Mixer reconstruction follow at L3; recording and workspace acceptance follow at L4.

Capture coverage and unresolved measurements are in [basic-track-workflow.md](../basic-track-workflow.md). The schedule and review gates are in [fidelity-plan.md](../fidelity-plan.md).

The expanded menu, Project-window, track-control and Channel-Settings observations are documented in [L1 menu and Project capture](L1-menu-and-project-capture.md). They extend the baseline evidence but do not establish full Cubase behavior or parity.
