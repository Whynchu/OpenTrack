# R4-20 — Review hierarchy across views

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 20; user decision 2026-11-02 (folders are organizational, collapse does not affect playback/recording).
- Desired observable outcome: Project, Mixer, Inspector and keyboard track navigation agree on folder identity. Folders have no Mixer strip and no channel Inspector controls. Track Up/Down and edge navigation skip rows hidden by a collapsed folder or group while leaving the hidden tracks selected/unchanged. Collapsing never deletes selection or mutes audio. Folder subtree drag/reorder and duplicate behave as R4-18. Deleting a folder keeps children and their sends; no data is lost on save/reopen.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: Mixer iterated every project track, so a folder would have appeared as a channel strip. Inspector rendered channel controls for every selected track. Keyboard navigation stepped every row even when its height was zero. Project already hides collapsed group children by zero row height.
  - Observed: Playback excludes folders by kind; track selection is stored by project index and remains valid when rows are hidden.
  - Unknown: live desktop visual/scroll behavior and user review.
- Dependencies and decisions needed: R4-17–R4-19 delivered. Hidden-track navigation policy chosen here: skip hidden rows for Up/Down/Home/End; do not clear an existing selection when it becomes hidden.
- Code scope and architecture boundaries:
  - Mixer excludes folders from strip layout and visible-width accounting.
  - Inspector shows folder name and an organizational notice instead of channel controls.
  - Track navigation works over visible rows only; no change to selection storage or audio plan.
  - No new persistence or schema change.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Mixer creates no strip for a folder and keeps other strips' selection indices correct.
  2. Selecting a folder shows an organizational notice, not fader/insert/record controls.
  3. With a collapsed folder, Up/Down/Home/End skip the hidden child but the child remains in the project and selection is not deleted.
  4. Folder subtree reorder/duplicate and save/reopen behave as before; hidden children stay in the audio/recording plan.
  5. Full app/core suites, inventory and formatting pass.
- Affected regressions: Mixer strip layout/selection, Inspector, keyboard navigation, folder collapse, save/reopen.
- Essential gates: identity agreement across views; hidden rows are view-only; no audio/selection loss.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `e49c326`; commit recorded in Git history.
- Changes and reasons: filtered folders from Mixer strips and width accounting, replaced folder Inspector controls with an organizational notice, and made track navigation operate on visible rows while retaining hidden selections.
- Commands/checks and actual results: focused navigation test 1 passed; full app suite 533 passed, 9 ignored after inventory regeneration; core suite previously 159 passed (no core behavior changed in this step); inventory 271 leaves, 84 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; no desktop view/scroll inspection.
- Failures/unknowns/pending acceptance: live Mixer absence of folder strips, Inspector notice, navigation feel, hidden selection presentation, subtree drag/reorder visuals and save/reopen review remain pending.
- Next ready task: revision-4 step 21, enable a real Marker lane.
