# OpenTrack roadmap revision 5: finish editing and interchange workflows

Status: prepared, 2026-11-02. Planning baseline: `9a5e7dd` on `main` (revision 4 complete plus the manual-review fixes). Prepared by the OpenCode runner; intended implementation runner: OpenCode using DeepSeek V4.1 Flash (record the actual model identifier reported by the runner). This document authorizes no claim that implementation, runtime review or user acceptance has happened.

## Starting evidence and scope

Revision 4 delivered steps 1-32 and is closed; its handoff is [revision-4-delivery](handoffs/revision-4-delivery.md). The manual review after the revision-4 build produced four findings, all fixed in `9a5e7dd`: extra mixers moved to native viewports (click-through), track-keyed widget ids made relative (duplicate id 7411), the standalone spectrum window gained its own grid and a state line, and Audio Mixdown now takes its range from the locators only, by user direction.

Measured at this baseline (rerun in step 1, not asserted here): the generated inventory reports 275 project-open leaves - 40 available, 145 state-dependent, 82 unavailable, 8 informational - and 147 of 152 bundled command IDs have a dispatch arm. The grep-based disabled-entry count in source is 58 unique labels. Suites: app 549 passed / 4 failed / 9 ignored, core 164, audio 170 / 3 ignored. The four failures are `transport_locate_tests` roll cases that fail identically on the pristine revision-4 checkpoint on this host because no audio output device is available to `schedule_roll`; they are a recorded baseline, not a regression. OPR is 37.

The remaining five undispatched command IDs are `devices.video`, `edit.open_score_editor`, `project.open_tempo_track_editor`, `transport.retrospective_record` and `transport.use_external_sync`. Two of them become reachable in this revision (tempo editor, retrospective record); the other three stay unavailable until their features exist.

Goal: close the highest-value gaps that have a workable contract - tempo/signature editing, retrospective capture, offline processing, cue-sheet import, archive backgrounding and the remaining timeline surfaces - and prepare honest decision records for the ones that need hardware, fixtures or a dependency choice. This is a 32-step queue, not a promise of 32 newly enabled menu rows.

## Execution contract for OpenCode

- Read AGENTS.md, docs/workflow/README.md, architecture-agreement.md and the implementation/review skills before edits. This revision is the active planning authority.
- Execute serially. Each step is one bounded packet; split it further when necessary while keeping its step number. Do not combine unrelated work. Keep new responsibilities in focused modules; `app.rs` composes.
- Before each step, create a packet from `tasks/TEMPLATE.md` with the behaviour, file ownership, dependencies, refusal/cancel semantics and acceptance checks defined before implementation. A dependent step is ready only when its foundation works.
- Preserve all existing files, including untracked recovery and bench directories. Never reset or clean the tree. Commit only the slice's changes.
- Use one command path for menus, shortcuts and gestures. Content edits get one labelled history operation; true no-ops get none. Validate the whole edit before publishing.
- File work runs away from audio callbacks and UI frames, with bounded jobs, cancellation, progress, stale-result rejection and cleanup. Originals and files needed by undo/discard survive. Never publish half a multi-file edit.
- New persisted fields need validation, defaults/migration and the native version policy, incrementing from the actual current version. Accompany schema edits with older-file and save/reopen checks.
- Run focused checks per slice and affected regressions. Do not weaken assertions or relabel product failures as harness failures. Record exact results and known-baseline failures.
- Build product changes through the platform build script. Keep one app instance and close it through its UI before replacing a build.
- Update the packet, README status, measured inventory and current.md after each slice. Keep current.md concise.
- Checkpoint each verified slice in a local commit; push only under publication authorization for this run.
- Runtime, listening, hardware and user acceptance stay separate. Never interpret inability to run the desktop as a pass. A detected correctness failure blocks dependent work.

## The next 32 steps

### A. Baseline and QA follow-through

1. **Reconcile inventory and QA baseline.** Re-run the generated inventory and shortcut audit, record exact counts and the four environment-failing roll tests, and reconcile README, current.md and the accessibility report against the source. Note the manual-review fixes and their new tests. Check: deterministic regeneration, no duplicate menu paths, empty-project shell unchanged, documented baseline failures.

2. **Runtime review of the review fixes.** Provide a short checklist for the user: drag Mixer 2 over the arrange and click through it; open both extra mixers together and confirm the console stays free of "Second use of a widget"; read the Spectrum Analyzer status line during playback; open Audio Mixdown and confirm the locator range. Record the observed results and fix any confirmed defect as its own commit. Check: each item has observed evidence or stays explicitly pending; a failed item blocks the related chain.

3. **Export and delivery contract review.** Verify that mixdown, stems, bounce-in-place and arranger export each derive their range and length from a defined source (locators, selection or sequence), that dry-run refusal paths are clear, and that no path silently exports a different range than the user sees. Fix confirmed divergence; report findings even when none are found. Check: locator edits reach every export path, stems remain per-track, cancelled exports leave no published file.

### B. Tempo and time signature

4. **Define the tempo/signature contract.** Persist one project tempo event at beat 0 plus a time signature (numerator 1-16, denominator 1/2/4/8/16) and the metronome accent pattern, bounded and validated, with an OPR bump and migration from the current version. Multi-tempo maps are explicitly out of scope and must be refused rather than flattened. Check: round trip, legacy defaults, invalid values rejected, existing beat/sample conversions unchanged for the default tempo.

5. **Enable the Tempo Track Editor.** A focused editor window edits tempo, signature and accents through the contract, applying as one labelled history operation with a true no-op when nothing changed, and refusing values the engine cannot honour. Reuse the existing metronome accent model rather than inventing a second one. Check: invalid input, undo/redo, switching transport during edit, save/reopen, ruler and metronome agreement.

6. **Audit beat/sample conversion under a constant tempo.** Add regression checks across recording placement, locator snapping, automation timing, MIDI import/export and offline render at several tempo and sample-rate combinations. Fix any conversion that assumes a hard-coded tempo or rate. Check: generated-signal timing tolerance recorded, no behaviour change at 120 BPM/48 kHz, mismatched-rate media still refused or conformed explicitly.

7. **Tempo track import and export.** Add a documented, versioned text/JSON exchange for tempo, signature and accents with a round trip and explicit refusal for unsupported constructs. Enable File > Import/Export > Tempo Track for this format only; do not claim Cubase tempo-track compatibility. Check: malformed files, version mismatch, partial writes, and a reopen that preserves the imported values.

8. **Wire the tempo editor command.** Connect `project.open_tempo_track_editor` (Ctrl+T) and the Tempo Track menu entries to the editor from step 5, removing them from the disabled inventory only when the editor works. Check: menu and shortcut reach the same editor, focus guards, empty-project behaviour.

### C. MIDI workflow completion

9. **Enable the transpose dialog.** Back a bounded Transpose dialog with the existing MIDI transform: semitone range, selected notes or whole part, one labelled history step, refusal when no part is selected, and a true no-op when the value is zero. Check: selection scope, octave edges, undo, non-target CC preservation and save/reopen.

10. **Define retrospective capture.** Specify a bounded in-memory ring buffer for incoming MIDI (for example 60 seconds or 8192 events, whichever comes first) with monotonic timestamps, a documented memory ceiling, no persistence and no disk writing, and a defined behaviour when the transport is playing or recording. No privacy claim without a written policy. Check: bounds, wraparound, clock alignment with the audio position, clearing on project close.

11. **Enable Retrospective Record.** Materialize the buffer as a MIDI part on the armed destination when the command runs, one labelled history step, with a clear reason when no input device, no armed track or no recent events exist. Do not fabricate notes for silent input. Check: exact note timings against a synthetic input stream, empty buffer, armed/track selection, undo and project-close clearing.

12. **Close the virtual-keyboard review items.** Finish the Alt+K feature against runtime evidence: held-note release on close, monitor gating with and without hardware MIDI input, interaction with MIDI Reset, and the audible result through an armed instrument track. Record listening evidence separately; do not mark it passed from tests. Check: no stuck notes, no monitor left enabled after close, focus and repeat presses.

### D. Offline processing

13. **Define the per-event offline chain.** Persist an ordered chain of built-in processors (gain, normalize, reverse, remove DC offset, stereo flip, resample, time stretch, envelope) on an audio event, bounded in length, validated by processor and parameters, with an OPR bump and migration. The chain describes processing intent; it does not change playback until applied. Check: unknown processor refusal, duplicate/limit rules, legacy events unchanged, save/reopen.

14. **Render an offline chain safely.** Apply a chain to a staged file away from the callback, publish only a complete result, cancel on request, reject stale results and clean up partial files. Reuse `opentrack_audio::audio_process`; do not duplicate DSP. Check: failed second processor leaves the event and file untouched, cancel, concurrent edit during render, undo/discard lifetimes.

15. **Enable Direct Offline Processing.** Provide the per-event chain UI (add, remove, reorder, inspect parameters, apply, revert) with one labelled history step per applied render and clear refusal for unsupported combinations (for example a chain on a locked event). Check: menu, selection scope, undo/redo, reopen, and that a chain is never applied twice.

16. **Enable Add Plug-in and Remove Plug-in for built-ins only.** Scope the two realtime-processing entries to adding and removing the built-in processors from the chain. Third-party VST3 in this stack stays unavailable with a stated reason; hosting remains insert-based. Check: entry availability, refusal message, no duplicate processors where unsupported, chain persistence.

17. **Accept the offline processing path.** Exercise a multi-event, multi-processor run with one failure, cancellation and a locked event; confirm no partial commit, correct undo and reopen, and matching offline/live output for a generated signal within a recorded tolerance. Check: measurements recorded, listening pending if no device.

### E. Interchange and portability

18. **Enable Audio CD import through cue sheets.** Implement a bounded `.cue` parser (file, track, index, title, performer) that imports a referenced WAV as separate events per track with a report of unsupported commands. Refuse malformed sheets, missing audio, overlapping indices and out-of-range times; do not claim red-book image support. Check: authored fixtures, track boundaries, one history step, undo and reopen.

19. **Background the Track Archive import.** Move the synchronous import into a bounded job with progress, cancellation, stale-result rejection and no partial project publication. Keep the validated manifest/path containment checks. Check: cancel mid-copy, huge package refusal, project edited during import, reopen.

20. **Accept the archive round trip.** Export a project with folders, FX returns and mixed media, import into a nonempty project, and verify IDs, hierarchy, routing and media deduplication against recorded expectations. Check: external route report, missing-module report, one history step, no source overwrite.

21. **Record the interchange decision.** *User decision (2026-11-02):* compatibility with other project file formats (CPR, OMF and similar) begins only once OpenTrack has full feature support for the content those formats carry; until then no adapter, converter, alias or placeholder dispatch is written. Keep the entries unavailable with that reason recorded and referenced from the remaining menu rows, so a future adapter starts from a known feature baseline rather than mapping incomplete concepts. Check: the decision is stated once, the rows stay honestly unavailable, and nothing is claimed about Cubase compatibility.

### F. Timeline surfaces

22. **Decide and wire the Add Track lane entries.** Map `Marker` and `Ruler` to the existing marker lane and secondary ruler where that is an honest equivalence, and keep `Tempo`, `Signature`, `Transpose` and `Video` honest: wire Tempo/Signature to the new editor, Transpose to the dialog, and leave Video unavailable with a reason. Do not create fake audio track kinds. Check: each entry either performs the named action or states why it cannot, with the inventory regenerated.

23. **Polish the Regions editor.** Add multi-select, keyboard navigation and delete-with-confirmation while keeping region selection distinct from event selection. Check: overlap handling, minimum length, snap on commit, cancelled drags, undo and insert/delete-time behaviour.

24. **Close the arranger preview review items.** Verify stop, seek, cycle refusal and MIDI release during playback-only preview; fix confirmed defects. Record listening evidence separately. Check: rapid stop/seek, project close, sequence item boundaries, no hanging notes.

25. **Review the secondary ruler.** Confirm format coverage (bars, seconds, samples, 30/60 fps timecode), timecode offset handling, zoom/scroll sharing and workspace persistence at several sample rates and zooms. Fix divergence from the primary ruler. Check: alignment tests at 44.1 and 96 kHz, workspace round trip.

### G. Devices, sync and video scoping

26. **Evaluate MMC.** Determine whether OpenTrack has (or should gain) a MIDI output path suitable for MMC master or slave, and record the decision with protocol, message set and failure behaviour. Implement only the bounded subset the existing path can carry; otherwise keep the entry unavailable with the reason recorded. Check: no placeholder dispatch, documented device requirement.

27. **Record the external-sync decision.** Document supported clock sources (MIDI clock, MTC, word clock), latency handling and the hardware needed to verify, then either scope a bounded implementation or keep the feature unavailable. No implementation without a verification path. Check: decision record linked from the remaining command row.

28. **Evaluate a video decoder dependency.** Compare candidate decoding paths for licence, size, build complexity, x64 Windows support and frame-accurate sync needs against the repository dependency rule. Record the recommendation; implement nothing until the user approves a dependency. Check: written comparison, explicit recommendation, no silent dependency addition.

### H. Platform, quality and close-out

29. **Performance and diagnostics pass.** Measure arrange draw and meter/analyzer polling costs on a project with many tracks and events, identify one measurable hotspot, and fix it without changing behaviour. Record before/after numbers and method. Check: frame-time measurement, no functional regression, release build comparison.

30. **Accessibility and keyboard pass.** Dispatch the commands whose features now exist (tempo editor, retrospective record), audit focus guards on the new editors, and record which remaining commands are blocked by missing features rather than wiring. Check: inventory regenerated, no fake dispatch arms.

31. **Revision acceptance.** Run the affected full suites, formatting and diff checks, the platform release build and old-OPR migration checks on real saved files; exercise the new menu paths, cancellation and failure paths. Record source identity, build hash and exact results; separate source, automated, runtime and user acceptance. Check: recorded counts, pending desktop/hardware items listed.

32. **Consolidate and prepare the next queue.** Regenerate the inventory, reconcile README/current/task packets and the native-format document, measure module growth, and report delivered, partial, blocked and runtime-pending work separately. List the remaining score, video, external sync, MMC, Control Room and legacy interchange gaps with their fixtures or decisions. Do not append unbounded post-close slices.

## Decisions needed from the user

- **Video:** approve or reject a decoder dependency after step 28's comparison. No video work proceeds without that decision.
- **CPR/OMF:** decided 2026-11-02 - compatibility with other project formats waits for full feature support, so no fixtures or adapter work is needed now. Nothing about those formats is claimed in the product or the docs.
- **External sync / MMC:** provide a device and a clock source for verification, or keep both features unavailable with the recorded reason.
- **Control Room and score engraving:** these need product design before planning; they are not in this queue.
- **Windows x64:** the local-ASIO deliverable needs a Windows host for build and runtime verification.

## Dependencies and pause rules

Steps 1-3 precede feature edits. Tempo 4-8 are a chain; 5 and 8 depend on 4, and 7 depends on 4. Retrospective 10-11 are a chain. Offline processing 13-17 are a chain and must not start before step 3's export review confirms the range/delivery contract. Interchange 18-21 are independent of each other except 19-20. Steps 22-25 depend on the earlier features they touch. Steps 26-28 are decision records and can run in any order; only step 26 may implement code.

A schema, timing or graph decision beyond these contracts requires a written decision in the packet before dependent changes. If no independent reviewer is available, name the gap and stop the dependent high-risk chain. Two failed repetitions without new evidence need a new approach or a named blocker.

## Explicitly deferred in revision 5

Score engraving and layout, Control Room monitoring, MMC implementation without a MIDI output path, external sync without hardware, video without an approved decoder, real CPR/OMF adapters without fixtures, and multi-tempo maps. These omissions are deliberate: keep the relevant entries honestly unavailable and report them rather than deleting labels or adding aliases to improve a count.

## Revision 5 close-out (2026-11-02)

All 31 implementable steps were executed, and step 2 is the user's runtime review, which remains open rather than claimed. Steps 1 and 3-32 delivered code, tests or decision records; step 2's checklist is in R5-01's handoff and in the delivery handoff.

Final measurements: inventory 275 project-open leaves (41 available, 157 state-dependent, 69 unavailable, 8 informational); 149/152 command IDs dispatched; OPR 38; app 579 passed / 4 known device-dependent failures / 9 ignored, core 166, audio 171 / 3 ignored; release build `48e524cc...`.

Delivered: baseline inventory, export contract review, tempo/signature contract and editor, tempo exchange, conversion audit, transpose verification, retrospective capture and command, virtual-keyboard close-out, offline chain model/render/UI/menus/acceptance, cue-sheet Audio CD import, background and accepted Track Archive import, add-track lane mapping, regions polish, arranger preview review, secondary ruler review, MMC/sync/video decision records, backgrounded chain render, dialog keyboard ownership and the acceptance pass.

Remaining work is user-blocked or decision-blocked: the runtime review, audible checks, Windows/ASIO and VST3 verification, the two open export decisions (stems and arranger export), and the larger features named in the decision records.
