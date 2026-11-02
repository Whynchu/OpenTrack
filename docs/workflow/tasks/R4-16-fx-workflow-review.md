# R4-16 — Review the FX workflow end to end

- Status: review_pending (source/headless)
- Scope version and requirement IDs: roadmap revision 4, step 16. User decision 2026-11-02: FX returns are shared sends with cycle rejection; do not change implemented behavior without review.
- Desired observable outcome: a user can create an FX return, add an insert, send two tracks to it, save/reopen it, render the result through the live and offline paths, and undo its removal without dangling routes. FX volume automation and Write are explicitly unsupported in this revision and remain disabled. A concrete listening procedure is recorded for the desktop pass.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown):
  - Observed source: FX kind/validation (R4-13), typed `FX:` sends and cycle checks (R4-14), menu/dialog creation and deletion cleanup (R4-15). The audio engine already treats every bus through `PlaybackTrackSettings.is_group`; the app maps FX to that flag and the offline renderer takes the same settings.
  - Observed automation policy: track-volume Read/Write is limited to Audio/Instrument tracks in Project, Mixer, Channel Settings and Inspector. FX channels now also fail project validation if automation flags or lanes are present, matching the "unsupported until designed" rule rather than implying Read/Write support.
  - Unknown: live desktop dialog/visual review, actual third-party VST3 effect loading, hardware audio and listening.
- Dependencies and decisions needed: R4-13–R4-15 delivered. Desktop/hardware availability is separate.
- Code scope and architecture boundaries:
  - Review only, plus tests and the explicit FX automation validation. No new engine path, no automation implementation, no change to group behavior.
  - Added an audio test proving two sends sum on one FX return with an insert and that live and offline render produce the same generated-signal output.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Create an FX return (menu/context) with an optional VST3 insert; add sends from two audio tracks; both receives appear in the send menu.
  2. Live plan and `OfflineRenderer` produce the same summed/inserted signal; test covers generated constants, pre-fader gain and a hosted insert.
  3. Save/reopen keeps the FX kind, send targets and missing-VST3 insert identity; routing validates on write.
  4. Removing the return clears and disables its send targets; the removal is undoable; group deletion and group route spellings remain unchanged.
  5. FX automation flags/lanes are rejected; Read/Write controls stay disabled. Cycle/dangling cases remain typed errors.
- Affected regressions: audio live/offline render, routing cycles, project validation/save, add/delete history, automation controls.
- Essential gates: graph invariant (two senders to one acyclic return) passes; unsupported automation is refused rather than enabled by analogy.
- Review owner and milestone stop: user review. This packet is self-review plus automated evidence only; no independent reviewer and no runtime acceptance are claimed.

## Listening procedure (pending desktop/hardware)

1. Build x64 with local ASIO as usual, open a disposable project copy.
2. Project > Add Track > FX Channel, count 1, stereo, choose an installed VST3 effect (for example a reverb) if available; otherwise create the return and add an effect later from Channel Settings.
3. Add two audio tracks with different material and route both sends to the return; enable both sends post-fader at -6 dB.
4. Play. Confirm the dry tracks are audible, the return adds only the wet effect, both sends contribute, and muting/bypassing the return insert changes only the wet path.
5. Solo/mute the return and one source; confirm no feedback, stuck audio or unexpected master fallback.
6. Save, close, reopen; confirm the FX return, its insert and both send destinations return. Remove the return and confirm the sends show No Bus; Undo restores them.
7. Export a short range and compare with live playback; listen for level/delay mismatch.
8. Confirm automation Read/Write cannot be enabled on the FX channel.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). Self-review only; no independent reviewer.
- Source identity: based on `28ae424`; commit recorded in Git history.
- Changes and reasons: added an end-to-end generated-signal audio test for two sends into one FX return with a hosted insert in both live and offline render; added explicit validation refusing FX automation flags/lanes so unsupported automation cannot be enabled by analogy. No behavior changes to existing group routing.
- Commands/checks and actual results: focused FX render test 1 passed; audio suite 168 passed, 3 ignored; core suite 156 passed; full app suite 530 passed, 9 ignored; formatting/diff checks run.
- Runtime environment and evidence paths: Windows x64 host, headless generated signals. No desktop, plug-in catalog, hardware or listening evidence.
- Ledger IDs updated; report command/result: current.md/README reflect the FX workflow and pending listening review.
- Review findings, rework, elapsed effort if available: graph invariant is satisfied for two senders to one acyclic return; live and offline output agree. The project UI still shows the generic fader/EQ/send rack, which is intended for a bus. Record/monitor are disabled on FX. Deferred: FX automation, nested FX routing design, hardware/VST3 compatibility.
- Failures/unknowns/pending acceptance: all runtime and listening steps above; third-party VST3 editor/loading behavior; desktop layout of the FX dialog.
- Next ready task: revision-4 step 17, folder hierarchy model.
