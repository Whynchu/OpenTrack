# R4-24 — Add the arranger data contract

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 24; user decision 2026-11-02 (arranger edits undoable; preview playback-only; no fake track).
- Desired observable outcome: the project can persist named arranger sections (half-open beat ranges) and an ordered play sequence of stable section IDs with positive repeats. Bounds are enforced (128 sections, 1024 sequence items, 1024 expanded plays), references must resolve, deletion removes the section and its references atomically, and expanded duration is bounded. Editing the arranger never moves or copies source clips. Add Arranger stays disabled until the editor and audible preview exist.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: sections are ranges; sequence references stable IDs; edits do not rearrange source clips; preview is playback-only.
  - Observed: `Project` already persists markers/regions with stable IDs and serde defaults; project validation is the single gate for loaded/saved content.
  - Unknown: editor/preview interaction in steps 25–28.
- Dependencies and decisions needed: none. Recursion is prohibited by construction: the sequence references sections only and sections cannot contain another arranger.
- Code scope and architecture boundaries:
  - New `opentrack-core::arranger` module with `Arranger`, `ArrangerSection`, `ArrangerSequenceItem`, bounds and commands (add/rename/resize/remove section, append/repeat/reorder/remove sequence items, expanded duration, validate).
  - `Project` gains a serde-default `arranger` field; OPR bumped to 37. No clip or timeline mutation.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Sections keep stable IDs and sorted start order; rename/resize works; add refuses empty names/invalid ranges/over-capacity.
  2. Sequence append/repeat/reorder/remove preserves references; unknown section or zero repeats is refused; expanded plays and duration are bounded.
  3. Removing a section atomically drops its sequence references.
  4. Arranger edits leave source clips byte-equal; validate accepts a valid arranger and rejects malformed/dangling data.
  5. Older documents default to an empty arranger; OPR 37 round-trips; full suites/formatting pass.
- Affected regressions: project validation, OPR migration, save/reopen, future arranger editor/preview.
- Essential gates: no source rearrangement; bounded/reference-safe; atomic removal.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `d2cc50d`; commit recorded in Git history.
- Changes and reasons: added the arranger module, project field and OPR 37, with section/sequence commands, bounds, reference checks and expanded-duration limits. Add Arranger remains disabled; no editor/preview was added.
- Commands/checks and actual results: arranger tests 4 passed; core suite 164 passed; full app suite 538 passed, 9 ignored; formatting/diff checks run.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README native version and current.md updated.
- Review findings, rework, elapsed effort if available: self-review only; the default `next_section_id` initially prevented any section creation and was corrected to 1.
- Failures/unknowns/pending acceptance: editor, sequence UI, offline render and audible preview remain steps 25–28.
- Next ready task: revision-4 step 25, implement the arranger editor and commands.
