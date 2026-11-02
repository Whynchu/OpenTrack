# R4-18 — Enable Add Folder and hierarchy commands

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 18; user decision 2026-11-02 (folders are organizational).
- Desired observable outcome: Add Folder is available in the Project > Add Track menu and the Project/track context Add menus. Track context menus can move the clicked row (or the current multi-selection when it contains that row) into any existing folder, or remove it from its folder. Moving or removing resolves to one core command with one undo step; refused moves leave the project exactly as it was. Duplicating a folder copies its subtree and rewrites the new parent IDs. Reordering a folder carries its subtree contiguously.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: folders are organizational and must not alter playback or recording.
  - Observed: group assignment already clones a candidate, validates it, and commits only on success. The generic history layer records pointer/key gestures as one step. `move_tracks_to` already reorders and interprets group membership; it needed folder-parent traversal for contiguous subtrees. Folder depth/cycle validation exists from R4-17.
  - Unknown: folder menu interaction and indentation/collapse presentation; steps 19–20 own rendering and cross-view review.
- Dependencies and decisions needed: R4-17 delivered. No dependency addition.
- Code scope and architecture boundaries:
  - Core: `assign_tracks_to_folder`, `remove_tracks_from_folder`; extend `duplicate_tracks` and `move_tracks_to` to traverse `parent_folder_id`.
  - App: enable Folder in both Add Track menus; add Move to Folder / Remove from Folder context actions; expand a context row to the current track selection; one command, one history step. No DSP/routing changes.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Add Folder from main and context menus creates a folder row with the chosen count; no channel/DSP fields.
  2. Move selected tracks into a nested folder: subtree stays contiguous, IDs/routing unchanged, project validates; a folder-into-own-descendant attempt is refused and the project is byte-equal to before.
  3. Remove from Folder clears only selected roots; unselected children remain.
  4. Reordering a folder moves its descendants with it; duplicating a folder copies the subtree with remapped parent IDs.
  5. Move into folder is one undo step; Undo removes the parent link and Redo restores it.
- Affected regressions: track ordering, group assignment/duplication, folder validation, context menus, history.
- Essential gates: refusal rollback; stable identity; subtree continuity; no folder DSP.
- Review owner and milestone stop: user review. Self-review only; no runtime acceptance claimed.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `ebaa1b6`; commit recorded in Git history.
- Changes and reasons: added atomic folder assignment/removal, folder-aware subtree traversal for reorder and duplicate, enabled Folder in both Add Track menus, and added context Move to Folder / Remove from Folder actions that use the current track multi-selection. Packet prepared after the source edits; scope matches the implemented diff.
- Commands/checks and actual results: core folder assignment test 1 passed; app undo/redo folder test 1 passed; core suite 159 passed; full app suite 532 passed, 9 ignored; inventory 271 leaves with 84 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only; the initial reorder assertion used a no-op destination and the duplicate assertion selected the wrong new folder; both tests were corrected.
- Failures/unknowns/pending acceptance: live context-menu interaction, pointer drag into a folder (step 19/20), indentation, collapse and hidden-track behavior remain pending.
- Next ready task: revision-4 step 19, render and fold folders.
