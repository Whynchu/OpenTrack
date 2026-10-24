# Current assignment

Updated: 2026-10-24. This is the single current-status entry point.

## Active stage

Workflow setup is implemented and checked; see [setup review](setup-review.md) for results and limits. The next product slice is Project transport within the ongoing M2 review. Complete the transport specification and source baseline, then the snap-on-release pilot. Subsequent transport packets wait for that pilot's review; another substantial milestone waits for user feedback.

The user accepted the Mixer milestone and later reported drag-edge scrolling working. Preserve both as regression obligations. Channel Settings/EQ work exists, including value-filled draggable parameter bars; the last handoff reported a successful build but did not establish live acceptance of those bars.

## Repository/build evidence

The user authorized committing and pushing the current development state on 2026-10-24, including M2/Channel Settings work, the workflow setup, and a public README with their supplied EXAMPLE screenshot. This is a development checkpoint, not M2 acceptance. Preserve existing work. Accepted Mixer baseline commit: ac758cc, already present on origin/main when fetched for publication.

Last reported x64 hash: `C3F4AA02721B7035A29D79322CC44F048565AE4CFD8A0D26DD2872D1651DFAFB`. This is historical handoff evidence, not a freshly verified current binary. Deliverable: `builds/x64/OpenTrack.exe`. Inspect process/build state before the next app build; keep one instance and close it when done.

## Next work

1. Execute [T001](tasks/T001-transport-baseline.md): bounded specification/source audit, ledger baseline, and snap pilot preparation.
2. Execute [T002](tasks/T002-snap-release.md) once T001 establishes its code seams and grid semantics.
3. Prepare bounded packets for mixed scrub/zoom, directional loop/skip ranges and actual playback, adaptive ruler density, and both scrolling modes. Measure unresolved skip behavior before claiming reference parity.
4. Review the transport slice with the user before expanding M2 or another milestone.

## Read on demand

- User intent, roles, metrics: [workflow](README.md).
- Architecture target: [agreement](architecture-agreement.md).
- Existing Project gaps: [checklist](../reviews/project-fix-checklist.md), [interaction study](../reviews/project-interaction-study.md), [layout audit](../reviews/project-layout-audit.md).
- Existing Channel Settings evidence: [study](../reviews/channel-settings-study.md).
- Desktop procedure: [remote review](../reviews/remote-review-workflow.md) plus the current computer-use skill/tool documentation.
- Earlier evidence index: [fidelity plan](../fidelity-plan.md). Its multiple current-position sections are historical.

## Desktop and persistence

The user authorized targeted Cubase investigation later in the conversation, superseding the earlier ban. Current focus is already collected evidence plus named unknowns, not a repeat sweep. The last user-prepared Cubase state was zoomed out with cycle playing. Live desktop availability/activity must be rechecked; do not assume this historical state still holds. Preserve EXAMPLE.opr; use a disposable copy for destructive acceptance. The user authorized saving on close, and requires unchanged projects to close without a save prompt.
