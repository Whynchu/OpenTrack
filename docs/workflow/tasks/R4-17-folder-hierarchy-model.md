# R4-17 — Add the folder hierarchy model

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 17; user decision 2026-11-02 (folders are organizational; collapse must not change playback/recording; hold changes to implemented behavior).
- Desired observable outcome: a project can persist organizational folders with stable IDs and up to eight ancestor levels. Folders hold no media, MIDI, inserts, sends, automation or routing and are omitted from the audio plan, so they never change playback or recording. Children reference folders through `parent_folder_id`; cycles, dangling references and non-folder parents are rejected. Deleting a folder keeps its children and reparents them to the removed folder's parent unless a later explicit delete-contents command is used.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: folders are organizational; collapsing must not affect playback or recording.
  - Observed: group hierarchy already uses stable track IDs and bounded nesting but is a DSP/routing concept; reusing it for folders would make an organizational row participate in group routing. `PlaybackOutputRoute::Group(index)` currently uses the position in the full project track list, so folders must be filtered before plan construction and routing indices must resolve against the same filtered list.
  - Unknown: desktop folder-row presentation and collapse behavior; steps 18–20 own creation commands, rendering and cross-view review.
- Dependencies and decisions needed: none. The folder kind is additive and does not alter group semantics.
- Code scope and architecture boundaries:
  - `opentrack-core::project`: add `TrackKind::Folder`, `parent_folder_id`, `is_folder()`, creation defaults (no input/output route), bounded hierarchy validation, and deletion reparenting.
  - `opentrack-app`: exclude folders from the playback/offline descriptor list and resolve output/send indices against that filtered list; label/stats/icon support so a loaded folder row is not unknown. No folder creation menu in this step.
  - OPR bumped to 36. The new field serializes with a default, so older documents open with no folder parents.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Folders can be added/nested in the model; 8 ancestors are accepted and a 9th is refused; cycles, dangling IDs and non-folder parents are refused.
  2. Folder rows reject media, MIDI, inserts, sends, automation, input/output routing; `validate` returns the named error.
  3. Removing a folder reparents its direct children to the removed folder's parent; undo/history use the existing track snapshot path.
  4. Plan construction excludes folders, and group/FX destination indices still resolve correctly when a folder sits between source and bus.
  5. Older OPR documents decode with `parent_folder_id = None`; save/reopen preserves the hierarchy and format version 36.
- Affected regressions: group hierarchy validation, playback plan indices, routing resolution, project validation/save, statistics, add-track dialog matches.
- Essential gates: folder rows have no DSP/media membership; hierarchy is acyclic and bounded; deletion never drops children implicitly.
- Review owner and milestone stop: user review. Self-review only in this run; no runtime acceptance.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `11569c7`; commit recorded in Git history.
- Changes and reasons: added the folder kind and `parent_folder_id`, bounded/cycle validation, deletion reparenting, model defaults and plan filtering with filtered routing indices; labels/stats cover folders so loaded rows are not unknown. Packet prepared after the source edits for this slice; the scope above matches the implemented diff.
- Commands/checks and actual results: core folder tests 3 passed; app folder/plan-index test 1 passed; core suite 158 passed; full app suite 530 passed after inventory regeneration (271 leaves, 85 unavailable). Formatting/diff checks run.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README native version raised to OPR 36; current.md updated.
- Review findings, rework, elapsed effort if available: self-review only; the first depth test stopped at eight total folders rather than eight ancestors and was corrected.
- Failures/unknowns/pending acceptance: folder creation UI, indentation/collapse rendering, Mixer/Inspector filtering and desktop interaction remain steps 18–20.
- Next ready task: revision-4 step 18, enable Add Folder and hierarchy commands.
