# R4-23 — Enable additional Ruler presentation

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 23; user decisions (view state does not create content edits; hold changes to existing behavior).
- Desired observable outcome: the Project ruler context menu can enable a secondary ruler row using an existing time format (Bars+Beats, Seconds, Timecode, Samples or 60 fps) while keeping the primary format. The secondary choice is workspace view state, shares the existing zoom/scroll, and uses the same label conversion, including the project timecode offset and sample rate. With no secondary format selected, the ruler height and all existing geometry are unchanged.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: `ruler_label` already converts bars/seconds/timecode/samples with tempo, sample rate and timecode offset; ruler hit testing seeks from the shared grid geometry.
  - User-specified: persist the view setting in workspace state; do not add tempo/signature changes.
  - Unknown: desktop visual alignment at zoom levels.
- Dependencies and decisions needed: none. Default secondary `None` preserves the old single-row ruler.
- Code scope and architecture boundaries:
  - `ArrangeViewState.secondary_ruler_format: Option<u8>` with serde default, normalization and round-trip tests.
  - Ruler context menu adds a Secondary Ruler submenu. When active, the ruler grows to two rows and draws primary/secondary labels at the same stride/positions; ruler seeking still uses the whole ruler rect.
  - No project schema/content change, no tempo/signature work.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Selecting a secondary format draws a second row; None restores the original 22 px ruler and row positions.
  2. Labels use the same conversion for both rows; samples follow project rate, timecode honors offset.
  3. The setting persists through save/reopen and defaults off for older projects.
  4. Ruler seeking and shared zoom/scroll remain unchanged; full app/core suites, inventory and formatting pass.
- Affected regressions: ruler geometry, row offsets, time label conversion, workspace persistence.
- Essential gates: old layout unchanged by default; view state not content dirty.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `d7623dd`; commit recorded in Git history.
- Changes and reasons: added a persisted optional secondary ruler format, a context-menu selector, two-row label drawing and longer ruler only when enabled.
- Commands/checks and actual results: secondary-format persistence test passed; extended timecode/sample label test passed; full app suite 537 passed, 9 ignored after inventory regeneration; inventory 273 leaves with 84 unavailable. Core suite unchanged (160 passed).
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; no visual zoom/alignment review.
- Failures/unknowns/pending acceptance: live two-row visual alignment, context-menu interaction and save/reopen review remain pending.
- Next ready task: revision-4 step 24, arranger data contract.
