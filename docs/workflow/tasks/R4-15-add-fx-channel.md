# R4-15 — Enable Add FX Channel throughout the UI

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 15; user decision 2026-11-02 (FX returns are shared sends, cycles rejected, existing behavior held for review).
- Desired observable outcome: Project > Add Track > FX Channel and the Project/track context Add menus open the existing track-add dialog for an FX channel. The dialog supports a count and mono/stereo configuration, offers an optional first insert from the scanned VST3 effect catalog, and creates each return through the same insert-below-selection command. FX channels appear in Project, Mixer and Inspector with ordinary fader/EQ/insert/send controls and no record, media or input controls. Deleting an FX return clears the sends that targeted it in the same undoable operation. Missing plug-in files reload as unavailable inserts rather than blocking project open.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - User-specified: multiple tracks may send to an FX return; cycles are rejected; changes to already implemented behavior need user review.
  - Observed: the existing add-track dialog already handles count, mono/stereo, instrument selection and one undoable creation gesture. `Project::insert_track_below_selection` is the shared creator. Channel Settings shows sends for every non-device channel and now lists FX returns. VST3 insert identities tolerate missing modules and report failures at load.
  - Unknown: desktop dialog layout at the added effect row, live add/delete interaction and audible insert review; step 16 owns the end-to-end review.
- Dependencies and decisions needed: R4-13 and R4-14 (delivered). No dependency addition.
- Code scope and architecture boundaries:
  - `project_controls` and the main Project > Add Track menu add `FX Channel` to the available rows; both still open `TrackAddDialog`.
  - `TrackAddDialog` gains an optional effect index; the FX arm filters `PluginCategory::Effect`, adds one insert to each created return and uses a taller dialog for that kind.
  - `opentrack-core::project::remove_track` clears send targets and disables those send slots only when the removed channel is an FX return. Group deletion behavior is unchanged.
  - No new engine path, no change to the arranger/mixer layout, no new dependency.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Add FX Channel from the main and context menus opens the same dialog; OK creates the requested count with FX names, optional effect insert, selection and dirty state.
  2. Creation and deletion are each one undoable history step; Undo restores the channel and its settings.
  3. Deleting an FX return sets every targeting send to No Bus and disables it; project routing validates.
  4. An FX return with a missing VST3 insert saves and reopens with the insert identity preserved; the runtime reports it unavailable.
  5. FX appears in Project/Mixer/Inspector as a stereo/mono bus with no media/input/record semantics; inventory drops the disabled row; full suites/formatting pass.
- Affected regressions: add-track dialog for other kinds, context menus, delete/history, send routing, VST3 insert loading, inventory counts.
- Essential gates: one creation command; atomic add/delete history; cleared dangling sends; no record/media UI for FX.
- Review owner and milestone stop: user review. Self-review only in this run; step 16 is the audible workflow review.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). No independent reviewer.
- Source identity: based on `a3e0f7e`; commit recorded in Git history.
- Changes and reasons: enabled the main and context FX rows, extended the shared dialog with an optional VST3 effect and FX-height layout, disabled record/monitor controls on FX strips, and made FX removal clear targeting sends while leaving group deletion semantics untouched. Packet prepared after the source edits for this slice; the scope above is the implemented scope.
- Commands/checks and actual results: focused add/delete history test 1 passed; FX OPR/missing-insert round-trip 1 passed; removal cleanup test 1 passed; core suite 156 passed; full app suite 530 passed, 9 ignored; inventory 271 leaves with 85 unavailable.
- Runtime environment and evidence paths: Windows x64 host; headless only. No desktop dialog/mixer review, plug-in load test with a real missing module, or listening.
- Ledger IDs updated; report command/result: README mixing/interchange counts and current.md updated.
- Review findings, rework, elapsed effort if available: self-review only; one dialog initializer missed the new field and was corrected. FX deletion cleanup is intentionally FX-only to avoid changing existing group behavior.
- Failures/unknowns/pending acceptance: live add-dialog layout, VST3 effect insertion/load with an actual plug-in, send ranking/visuals and audible return processing remain pending; the generic arm/monitor data fields remain for compatibility but record/monitor controls are disabled on FX strips.
- Next ready task: revision-4 step 16, review the FX workflow end to end.
