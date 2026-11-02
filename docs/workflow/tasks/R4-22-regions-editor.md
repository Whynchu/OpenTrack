# R4-22 — Expose Regions as editable ranges

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 22; user decisions (content edits undoable; region selection is not audio-event selection).
- Desired observable outcome: Project > Regions... opens a list/editor for every `Project.regions` entry, including hitpoint-created ranges. The user can select a region, locate to it, rename it, edit start/end beats, add a region (manually or from the locators) and delete it. Apply snaps edited ranges to the current grid, validates the whole draft, and commits one "Edit Regions" history step. Cancel discards drafts. Selecting a region never selects audio events; invalid/inverted ranges are refused without changing the project.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: regions have stable IDs; `add_region`, `rename_region`, `remove_region`, bounds validation and insert/delete-time adjustments already exist. There was no range-editing command or editor UI.
  - User-specified: regions are ranges, not audio events; content changes are undoable.
  - Unknown: desktop row/field interaction and visual review.
- Dependencies and decisions needed: none. Overlaps are allowed because regions are navigation ranges, not clips.
- Code scope and architecture boundaries:
  - Core: add validated `set_region_range` retaining identity and sort order.
  - App: new `app::region_editor` draft/editor; Project > Regions... menu; Apply performs candidate validation and one labelled history step. Selection and locate stay transient. No audio-plan or engine change.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Existing regions appear in the editor with name/start/end; hitpoint regions are not distinguished or dropped.
  2. Rename/resize/add/delete apply atomically with one history step; Undo restores prior regions.
  3. Invalid/inverted ranges and empty names are refused with the project unchanged.
  4. Selecting a region leaves the audio-event selection untouched.
  5. Overlaps validate; snap applies on Apply; full app/core suites, inventory and formatting pass.
- Affected regressions: region persistence, insert/delete time, marker/region list commands, undo.
- Essential gates: identity preserved; candidate validation; no event-selection coupling.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `51ffe34`; commit recorded in Git history.
- Changes and reasons: added validated region range editing, a transient region editor with select/locate/rename/resize/add/delete and one-step Apply, and the Project > Regions... entry. Overlaps remain valid; hitpoint regions are just entries in the same list.
- Commands/checks and actual results: core resize test 1 passed; app editor/undo test 1 passed; core suite 160 passed; full app suite 537 passed, 9 ignored; inventory 273 leaves with 84 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; the first editor test assumed unsnapped values while grid snapping was on, and now disables snap for that focused check.
- Failures/unknowns/pending acceptance: live editor layout, field interaction, snap feel and save/reopen review remain pending; no region lane graphics were added beyond the list editor.
- Next ready task: revision-4 step 23, additional ruler presentation.
