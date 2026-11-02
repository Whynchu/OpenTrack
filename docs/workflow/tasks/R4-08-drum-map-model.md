# R4-08 — Drum-map model

- Status: review_pending (model/persistence only; no editor yet)
- Scope version and requirement IDs: roadmap revision 4, step 8
- Desired observable outcome: a project-owned drum map maps MIDI pitches to names and display order for MIDI/Instrument tracks. Empty is the identity presentation (pitch numbers), not an implied General MIDI naming. Duplicate pitches and bad names are refused; legacy projects load with an empty map.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified by the roadmap: bounded project-owned mapping; identity default; presentation-only; validate uniqueness, names and limits; stable note IDs remain the selection identity.
  - Observed: `Project` serialization is serde JSON with version migration; `FORMAT_VERSION` was 33.
  - Unknown: editor ordering direction (top/bottom) belongs to step 10's consumer.
- Dependencies and decisions needed: none. Sounding pitch is unchanged; no General MIDI names are shipped.
- Code scope and architecture boundaries: new `opentrack-core::drum_map` model; `Project.drum_map` with serde default and `validate`; OPR 33 -> 34 with a migration test. No app UI.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Model tests: identity, order-preserving round trip, duplicate pitch/empty/overlong/control-character names refused without replacing the map, malformed loaded rows reported, empty slice restores identity.
  2. Project test: round trip, legacy field absence -> empty map, duplicate loaded rows make `Project::validate` fail.
  3. Migration: the version-interval test passes with FORMAT_VERSION 34; full core and app suites pass.
- Affected regressions: project serialization/migration, project validation.
- Essential gates: older files load unchanged with identity presentation; malformed new data is rejected on save/validation; no note or clip pitch is changed.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `b47ee2e` plus this slice; commit recorded in Git history.
- Changes and reasons: added `DrumMap`/`DrumMapRow` (128 rows max, 48-character names), `Project.drum_map` defaulted/skipped when empty, validation, OPR 34 and docs.
- Commands/checks and actual results: core 152 passed; app 514 passed, 9 ignored; formatting/diff checks pass.
- Runtime environment and evidence paths: Windows x64 host; serde round-trip fixtures in core tests. No app launch.
- Review findings, rework, elapsed effort if available: none. Self-review only.
- Failures/unknowns/pending acceptance: no editor or drum-lane presentation exists yet (steps 9-10). No runtime save/reopen of a populated drum map was performed in the app.
- Next ready task: revision-4 step 9, enable Drum Map Setup.
