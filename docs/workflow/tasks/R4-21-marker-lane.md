# R4-21 — Enable a real Marker lane

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 21; user decision 2026-11-02 (view/selection state does not create project history; content edits are undoable).
- Desired observable outcome: an optional dedicated marker lane appears above the Project track rows, backed by `Project.markers` and sharing the existing marker list, slot commands and navigation. Clicking empty lane space adds a snapped marker; dragging a marker moves it with snap; right-click removes it; clicking a marker seeks to it; double-click/rename stays in the existing Markers window. A move is one undo step and Escape restores the original beat. The lane is a timeline presentation, not a track, and its visibility is persisted in workspace view state (off by default so existing row geometry is unchanged).
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: markers already have stable IDs, add/rename/move/remove core commands, slots, next/previous navigation and a list window. Project row geometry is offset through `row_offsets`; adding a lane height there keeps row hit testing consistent.
  - User-specified: the lane is a timeline presentation, not a fake audio channel; marker edits are undoable.
  - Unknown: desktop visual/scroll interaction and rename gesture preference.
- Dependencies and decisions needed: none. Default hidden preserves existing layout; Project > Show Marker Lane enables it.
- Code scope and architecture boundaries:
  - `ArrangeViewState`: `show_marker_lane` (serde default false), transient `marker_drag`; no project content schema change.
  - Project view: draw/label the lane, offset row geometry, reuse core marker commands and existing snap/bar geometry. Marker edits set content-modified and rely on the existing labelled undo frame; Escape restores the original beat and cancels the pending history step.
  - No engine or fake-track membership.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. With the lane enabled, markers draw at the shared bar/beat geometry and hit testing finds them within the flag width.
  2. Empty click adds a snapped marker with the next default name; right-click removes; clicking an existing marker seeks; drag moves with snap.
  3. A drag produces one undo step; Escape restores the original beat and leaves no history entry.
  4. Visibility defaults off for existing projects, persists through save/reopen, and row layout is unchanged while hidden.
  5. Core/app suites, inventory and formatting pass.
- Affected regressions: Project row geometry/hit testing, ruler/marker list, undo, workspace state.
- Essential gates: no fake track; marker edits undoable; Escape rollback; layout unchanged by default.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `9e760c7`; commit recorded in Git history.
- Changes and reasons: added an optional marker lane, persisted visibility, shared marker hit testing, add/move/remove/seek interactions, row-height offsetting, and Escape/history cancellation. Rename remains in the existing Markers window. Visibility defaults off after fixed-coordinate regression tests showed the layout shift; this preserves existing behavior until the user enables the lane.
- Commands/checks and actual results: focused marker-lane/hit tests 2 passed; full app suite 536 passed, 9 ignored; inventory 272 leaves with 84 unavailable. Core suite unchanged from 159 passed.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; the first implementation defaulted the lane on, which shifted existing Project row coordinates and failed four pointer tests. Defaulting it off preserved existing behavior while keeping the feature available.
- Failures/unknowns/pending acceptance: live lane visuals, scroll/row alignment, drag/hit feel, Escape and save/reopen remain for desktop review; rename is not a lane gesture.
- Next ready task: revision-4 step 22, expose regions as editable ranges.
