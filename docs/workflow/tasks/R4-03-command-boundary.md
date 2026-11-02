# R4-03 — Establish the command boundary for revision 4

- Status: review_pending
- Scope version and requirement IDs: roadmap revision 4, step 3
- Desired observable outcome: the selection boundary used by the existing transport locate/play commands and by upcoming pre/post-roll commands is one pure, tested function; menu and key dispatch reach the same result; `app.rs` remains composition.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: `selected_event_span` in `crates/opentrack-app/src/app.rs` duplicated the audio/MIDI boundary logic; `play_from_selection_edge`, `play_until_selection_edge`, `locate_selection_start/end`, `locators_to_selection` and `play_selection_range` all consume it.
  - Observed: `TransportLocate::PlaySelection` (menu) and `transport.play_selection_range` (Alt+Space) both call `play_selection_range`.
  - Unknown: runtime desktop interaction; pre/post-roll commands do not exist yet.
- Dependencies and decisions needed: none. This step does not add roll lengths, preferences or scheduling; steps 4-6 do.
- Code scope and architecture boundaries: new app module `transport_commands.rs` owns `SelectionSpan` and `selected_event_span`; the app method delegates. No new dependency, schema change or undo semantics.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Pure tests cover empty selection, audio span at project tempo, mixed audio+MIDI part span and a stale selection.
  2. An app test runs the menu action (`TransportLocate::PlaySelection`) and the Alt+Space dispatch on equivalent projects and asserts identical locators and status; cycle is enabled by both.
  3. Existing selection/transport tests and the full app suite pass; formatting/diff checks pass.
- Affected regressions: transport locate/play-from/play-until, locators-to-selection, loop selection.
- Essential gates: selection resolution is content-free; no project dirty/history change from resolving a span. Focus guards remain unchanged because dispatch is untouched.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: `09994d7` plus this slice; commit recorded in Git history.
- Changes and reasons: moved the audio-event/MIDI-part span calculation to `transport_commands.rs` as a pure function taking the project, selected audio IDs and selected MIDI part. Callers keep the same tuple API. Added four pure tests and one menu/key equivalence test.
- Commands/checks and actual results: focused transport tests pass (4 pure + 1 menu/key equivalence); `cargo test -p opentrack --bin opentrack` — 502 passed, 9 ignored, 0 failed; generated inventory regenerated; `cargo fmt --all -- --check` and `git diff --check` pass. Runtime review pending because no app was launched.
- Runtime environment and evidence paths: Windows x64 host; generated `EXAMPLE`-style fixture projects in tests, no external media.
- Review findings, rework, elapsed effort if available: the initial stale-selection test used a zero-length clip, which `add_audio_clip` rejects; replaced with an unknown ID, which is the real stale-selection case. Self-review only.
- Failures/unknowns/pending acceptance: runtime menu/keyboard behavior, focus guards on a live text field, and the roll commands themselves are pending steps 4-6.
- Next ready task: revision-4 step 4, bounded pre/post-roll transport contract.
