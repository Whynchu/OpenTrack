# R4-25 — Implement the arranger editor and commands

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 25; user decision 2026-11-02 (arranger edits undoable; preview is playback-only).
- Desired observable outcome: an arranger editor creates sections from the locator range or an existing region, renames/resizes them, appends selected sections to the play sequence, changes repeats, reorders and removes sequence items. Source and sequence positions are shown separately; ordinary timeline playback is preserved. Apply validates the whole draft and commits one labelled history step; Cancel changes nothing. An empty sequence is allowed and reports that the ordinary timeline remains the playback source. No Add Arranger menu is enabled yet.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: R4-24 supplies bounded sections/sequence, stable IDs, reference validation and expanded duration. Region/locator ranges are available from `Project.regions` and the loop locators.
  - User-specified: edits undoable; preview playback-only; source clips not rearranged.
  - Unknown: live editor layout/interaction; audible preview is step 27 and menu enablement step 28.
- Dependencies and decisions needed: R4-24 delivered. Preview absent, so the editor has no active-preview/source-edit conflict yet.
- Code scope and architecture boundaries:
  - New `app::arranger_editor` draft/window; Apply clones the project, replaces `arranger`, validates, and commits one "Edit Arranger" step. No clip/audio-plan mutation.
  - Section and sequence rows display source beat ranges and cumulative sequence positions; selection is transient.
  - No Add Arranger menu until step 28.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Add from Locators / Add from Region creates sections; rename/resize edits the draft.
  2. Append/repeats/reorder/remove operate on stable section IDs; an empty sequence is accepted with an explanatory label.
  3. Apply makes one undo step; Undo restores the prior arranger; Cancel discards.
  4. Missing references/invalid ranges are refused with the project byte-equal to before.
  5. Source clips and ordinary timeline playback are unchanged; full suites/formatting pass.
- Affected regressions: project validation/save, history, regions/locators, future preview.
- Essential gates: atomic apply; no source rearrangement; reference-safe.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `56686a0`; commit recorded in Git history.
- Changes and reasons: added the arranger editor with locator/region section creation, section editing, sequence append/repeat/reorder/remove, source versus sequence positions, empty-sequence messaging and one-step Apply.
- Commands/checks and actual results: focused arranger editor test 1 passed (apply/undo/source-clip preservation/missing-reference refusal); full app suite 538 passed, 9 ignored after inventory regeneration; core suite 164 passed previously (no core change in this step).
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; the sequence row needed a precomputed section map to avoid borrowing the draft mutably and immutably at once.
- Failures/unknowns/pending acceptance: live editor layout, selection changes, cancellation, and save/reopen review remain pending; active-preview interaction belongs to steps 26–27.
- Next ready task: revision-4 step 26, render an arranger sequence offline.
