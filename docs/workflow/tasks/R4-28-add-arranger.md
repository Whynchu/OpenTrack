# R4-28 — Enable Add Arranger and review the complete path

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 28; user decision (arranger edits undoable, preview playback-only).
- Desired observable outcome: Project > Add Track > Arranger and the Project/track context Add menus open the single arranger editor; no fake ordinary track row is created. Sections, repeats, sequence Apply, Undo, save/reopen and Preview Arranger share the same arranger data and sequence transformation. The offline renderer consumes the same ordered sections/repeats. If live preview had been impossible, the entry would stay disabled; the source path now exists and only runtime listening remains unverified.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed: R4-24 contract, R4-25 editor, R4-26 offline transformer and R4-27 preview exist and use the same section/repeat data.
  - User-specified: no fake timeline track; preview playback-only; edits undoable.
  - Unknown: actual desktop/listening behavior. No audio device or desktop session was opened in this run, so this packet claims source/headless only.
- Dependencies and decisions needed: R4-24–R4-27 delivered.
- Code scope and architecture boundaries:
  - Main and context menu entries call `open_arranger_editor` via one app action; no `Track`/`TrackKind` is added.
  - Save/reopen test covers arranger sections/sequence; the offline renderer and preview use the same expansion.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Add Arranger opens the editor from main and context menus; no ordinary track appears.
  2. Apply is one undo step; Undo restores; save/reopen preserves sections and sequence.
  3. Preview and offline render both use the same sequence; preview remains playback-only.
  4. Sequence playback/runtime listening and rapid-stop/seek behavior are recorded as pending, not claimed.
  5. Full suites, inventory and formatting pass.
- Affected regressions: menus, project history, arranger persistence, preview/transport.
- Essential gates: no fake track; same sequence data across edit/save/preview/render; no runtime claim without evidence.
- Review owner and milestone stop: user review; self-review only.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No reviewer.
- Source identity: based on `00d336f`; commit recorded in Git history.
- Changes and reasons: enabled the main/context Arranger entries to open the existing editor, added one app action, and confirmed arranger OPR round-trip with the same sequence data. No fake track was introduced.
- Commands/checks and actual results: arranger OPR round-trip test passed; full app suite 541 passed, 9 ignored; core 164/audio 171 from preceding steps; inventory 274 leaves with 83 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless only.
- Ledger IDs updated; report command/result: README/current.md updated; inventory regenerated.
- Review findings, rework, elapsed effort if available: self-review only. Live sequence playback and export of the sequence are not runtime-verified; no listening evidence is claimed.
- Failures/unknowns/pending acceptance: desktop editor/preview interaction, rapid stop/seek, MIDI hang checks and audible export comparison remain pending.
- Next ready task: revision-4 step 29, specify and export OpenTrack Track Archive v1.
