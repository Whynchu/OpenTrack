# OpenTrack feature and architecture roadmap: 32 steps

Status: active roadmap, 2026-11-01. The user confirmed the CPU/memory meter task is done and authorized completion of this roadmap. Steps are tracked below; listing a feature is not itself implementation or acceptance.

## Direction

Use incremental architecture work tied to the next real feature. Do not wait until every planned feature is built to address `app.rs`, and do not attempt a large upfront rewrite. After the active CPU/memory task is reviewed and accepted, establish one small feature boundary, deliver a vertical slice through it, then repeat. This keeps each extraction small enough to compare against current behavior and gives later features a cleaner place to live.

Keep `app.rs` as the composition point for the eframe application: startup, top-level state ownership and frame orchestration should remain easy to find. Move cohesive responsibilities into modules, and reduce broad mutable access by making feature inputs/actions explicit over time. File moves that leave every helper reaching into all of `AppData` are temporary steps, not the architectural goal. Preserve the current crate structure and avoid framework rewrites, generic service layers and new dependencies without demonstrated need.

## Product direction from the competitor comparison

The user's preferred editing and mixing workflow is the product constraint. Do not copy SoundCraft's Pro Tools-style Edit/Mix window model or its visual design. Compare claims against OpenTrack's source and README; close useful workflow gaps with working behavior and evidence, not by matching a competitor's checklist.

Lead with verified OpenTrack strengths: 32-band static/dynamic EQ, spectrum and tonal-reference analysis, up to 24 insert and send rack slots per channel, third-party VST3 effect hosting, the Sample Editor's Pitch & Time workflow, WASAPI with optional local ASIO, and audio/MIDI capture. Keep the send caveat visible: track and group send processing is implemented and exercised offline, while live hardware review and device-bus sends remain open.

Near-term gap priority after the first architecture seam:

1. Audio mixdown/export and Standard MIDI File import/export, which make completed projects portable and deliverable.
2. Audible send processing and typed routing, extending the existing 24-slot send UI into real signal flow.
3. Automation playback and user editing/writing.
4. Recording recovery, real-device reliability, and broader physical channel routing.
5. Improve the current VST3 workflow and built-in processors based on compatibility and stability evidence; do not chase CLAP/AU parity by name alone.
6. Consider CLI/MCP control and broader deployment after OpenTrack's own editing commands and core user workflows are stable. Kubuntu Linux is a user-verified platform; expand validation to more Linux distributions and other operating systems based on demand.

The exact order of export, MIDI interchange, and send processing can change after tracing dependencies, but all remain ahead of lower-value menu polish and broad speculative rewrites. Score editing, video, surround, and competitor-specific window conventions are not commitments in this roadmap.

## Gate before this roadmap starts

The current user-directed CPU/memory meter task T071 was accepted by the user. The existing working tree has T071-related app/audio UI changes and other audit documents; preserve those exact changes and distinguish user acceptance from independent runtime evidence. This roadmap begins from the recorded baseline and dirty state, not from an assumed clean branch.

## Sequence

## Progress checkpoint

- **Step 1 complete by user confirmation:** T071 accepted by the user. No independent runtime evidence is claimed; see `docs/workflow/tasks/T071-transport-cpu-memory-meters.md`.
- **Step 2 recorded:** current audit baseline is HEAD `7b898c68f2272c5c7be9ef9d83f763072ae25424`; preserve the dirty/untracked audit and recovery files identified by `git status` when resuming.
- **Step 3 source audit complete with documented limits:** menus, keyboard commands, windows and selectors are linked from `docs/reviews/window-selector-coverage.md`. 152 shortcut IDs map to 44 implemented, 6 partial and 102 fallback actions. This is source evidence; running-app verification remains separate.
- **Step 4 first slice selected:** T072 makes the MIDI editor Snap checkbox affect positional grid snapping and persist as an application preference. Its interaction review is pending.
- **First app boundary (step 6) implemented:** MIDI preference defaults, persistence, validation, and Snap quantization now live in `midi_editor_settings.rs`; `OpenTrackApp` owns one settings value. Three focused tests and the existing MIDI editor render test pass. Windows x64 local-ASIO release compilation passed. Staging was blocked because an OpenTrack process held `builds/x64/OpenTrack.exe`; the new candidate is recorded in T073 and the existing package was left untouched.
- **Priority update from user:** README now describes OpenTrack's distinctive current capabilities and names the consequential gaps. The gap-first ordering above supersedes the earlier provisional candidate order; retain the user's preferred workflow rather than copying SoundCraft's UI model.
- **Platform and license update:** the user confirms OpenTrack was developed with Kubuntu Linux support and has been built and verified there; Windows x64 remains primary. Original workspace crates now identify as Apache-2.0 and the root `LICENSE` is present, with third-party notices retained.
- **Standard MIDI File progress:** T075 defines a source-safe format-0/1 note-and-controller contract. Core parsing/writing and File menu worker integration are implemented; eight focused checks, including format-1 import and an independent export decoder, and the full 31-test core suite pass. Independent DAW files and running-app acceptance remain.
- T071's `system_stats.rs` also establishes a small OS-statistics module boundary, though it does not yet reduce the broad mutable `AppData` surface in `app.rs`.

### Foundation and first seams

1. **Close T071 review.** Complete by user confirmation. Record no independent runtime observation; see the T071 task packet.
2. **Freeze a handoff baseline.** Record commit, dirty files, build/backend, known failures and executable identity. Keep user work intact; use a clean checkpoint/worktree only when the current state permits it safely.
3. **Complete source audit.** Complete from current source with documented runtime limits; audit registers are linked from `window-selector-coverage.md`.
4. **Choose first user workflows.** Complete: priorities are the user's core editing/mixing flow, audio export, MIDI file exchange, sends, automation and reliability; SoundCraft's UI convention is not a design target.
5. **Define the app boundary.** Complete for the first seam in T073: MIDI preferences are feature-owned; `OpenTrackApp` retains composition and passes state into the editor.
6. **Extract one focused app module.** Complete for T073 in `midi_editor_settings.rs`.
7. **Verify the extraction.** Three focused tests and the existing MIDI editor recovery render test pass; release compilation passed. Package replacement is pending because the app process holds the executable.
8. **Write the first feature contract.** Complete in T072; positional snapping, persistence, independent note-length grid, and acceptance evidence are specified.

### Deliver the first prioritized workflows

9. **Review the MIDI Snap workflow.** T072 source behavior and automated checks are complete; runtime gestures and restart persistence remain pending until native desktop interaction is available.
10. **Deliver audio mixdown/export.** T074 source implementation includes an isolated `OfflineRenderer`, no-device plan snapshots, independent VST3 worker instances, range/rate/encoding dialog, progress/cancellation and atomic WAV replacement. Six focused renderer/writer checks and local-ASIO compilation pass; runtime review and playback parity remain open.
11. **Deliver Standard MIDI File exchange.** T075 source implementation imports format-0/1 PPQ notes and CC into new tracks and exports project notes/CC with tempo and meter. Eight focused core tests, the full 31-test core suite and Windows x64 local-ASIO release build pass; independent DAW fixtures and undo/menu runtime review remain.
12. **Specify and model sends.** Implemented: [T076](tasks/T076-send-routing-model.md) records destination, pre/post position, level/pan, bypass, cycle prevention, persistence and migration.
13. **Implement and connect sends.** Source implementation and automated checks complete for track and group sends; live and hardware review remain open. See [T076](tasks/T076-send-routing-model.md).
14. **Verify sends.** Check gain, routing, silence, bypass and edge cases offline, then listen and review real-time performance in the application.
15. **Extract routing ownership.** Initial boundary established in `routing.rs`: send resolution, group-cycle decisions and active-send state are shared there. Broader route command ownership remains for later typed-bus work.
16. **Build general typed buses.** Extend track/group/input/output routing only after send semantics and existing device routing are stable; preserve current simple track routing.

### Automation playback and editing

17. **Specify automation data and gestures.** Complete for the initial track-volume scope in [T078](tasks/T078-volume-automation.md); bus and non-volume targets remain outside the contract.
18. **Implement automation playback first.** Complete in source: read-enabled track-volume curves are evaluated per sample in both engine paths and offline bounce using a bounded cached cursor. Live and hardware validation remains open.
19. **Add automation editing and recording.** Track-volume curves support snapped point insertion, drag movement and right-click removal through bounded core commands and project history. Read playback is implemented. Write capture and gesture cancellation remain open; until then, all Write controls are disabled and labeled unavailable. Bus automation and desktop review remain open.
20. **Verify automation.** Test persistence, undo, playback boundaries and recorded control movement; listen in the running app and document CPU behavior.

Revision checkpoint (2026-11-01): this revision delivered step 23/29 (take recovery and recovery autosave), 25/27/28 (edit family, command unification, selection regression), 26 (Add Track instrument catalog), 22 (typed validated routing) and 32 (consolidation). The app test suite is 289 passed / 0 failed, core 61 and audio 145. Step 31 remains blocked on real Cubase fixtures; runtime acceptance for T074/T075/T076/T078 and the recovery prompts is the largest open surface.

Automation checkpoint (2026-11-01): Write controls are disabled until capture is implemented; Read controls are limited to supported Audio and Instrument tracks. Arrange background and track context menus reveal stored valid volume curves. Focused source tests and app compilation pass. Point-drag cancellation, fader following/manual override, Write capture, loop-boundary writing, desktop/audio review and the known baseline arrange snap failure remain open.

### Recording, stability and completion

21. **Review mixer integration.** Check Mixer layout, Channel Settings, bus behavior, project migration, save/reopen and legacy projects. Treat runtime listening and user review as separate gates.
22. **Expand physical device routing.** Routing is now a typed, validated model (`opentrack_core::routing`): `InputRoute` (No Bus, Stereo In, mono input channel N) and `OutputRoute` (Stereo Out, Left only, Right only, No Bus, Group) parse the persisted spellings, keep them byte-compatible with existing projects, and are the only vocabulary the send rack, cycle check, live-monitor tap and playback plan use. The routing menus and the Add Track input picker are generated from `input_destinations`/`output_destinations`, so a menu can only offer a route the engine can play; the Add Track picker is limited to the channels the probed input device reports. `Project::validate_routing` reports unknown routes, dangling or self group targets, feedback loops and mono inputs wider than the capture bus, at load; a save refuses them. Basic track creation, recording and VST insert/load were verified by the user on 2026-11-01 with a Mackie interface and the laptop's built-in devices. Remaining: multi-channel physical routes and speaker layouts wider than stereo are not implemented, because both the capture bus and the output bus are stereo end to end; the model can express them and validation rejects them explicitly instead of folding them into the master.
23. **Improve recording reliability.** A recording journal is written before the recorder creates its files and cleared only after each take belongs to the project, so an abnormal exit leaves a recoverable record. On the next open of that project, a prompt lists the unfinished takes and each is restored (WAV header rebuilt from the real file length, clip placed at its recorded beat, one undo step) or discarded (moved to `Metadata/Recording/Discarded`, never destroyed). Ordinary recording against real devices was verified by the user on 2026-11-01 (Mackie interface and laptop built-ins); a real power-loss or kill during recording still needs a device review.
24. **Harden current plug-in workflows.** Failure isolation is now visible and actionable: a failed insert is quarantined by class (no re-instantiation on every plan rebuild), a failed state restore keeps the instance running, the Channel Settings row marks it with a red `!` and its reason on hover and retries on click, and opening a project with failures says how many did not load while the rest of the project plays. Scanning is now isolated per module: each module's factory enumeration runs in its own child process of the scanner with a bounded budget, so a hanging module is killed and reported `timed_out` and a crashing one is reported `crashed` with its exit status and stderr, instead of stalling or killing the scan. Plug-in Information lists every rejected module with its status, path and reason. Limits are documented in `vst3-status.md`: worker isolation is not a sandbox, hosted plug-ins still run in the application process, job-object containment is pending, a timed-out or crashed module is now quarantined in the persisted catalog and skipped by later scans until the user retries it in Plug-in Information, and the native editor is Windows-only. Adding and loading VST3 inserts was verified by the user on 2026-11-01; live listening review of a failed-then-retried insert and cross-machine plug-in coverage remain open. New formats require a demonstrated user need.
25. **Complete a high-value edit family.** Selected audio events support Split at Cursor (including a selected group segment, split through the group so members keep the shared boundary), Move to Cursor, Crossfade, Duplicate, Mute/Unmute and Select All/None; Range adds Select in Loop and Set Left/Right Locator from the selection. Every one is a single operation shared by the Edit menu, the reference key commands and project history. Remaining: Repeat/Fill Loop, Crop, Invert, Lock and event grouping (binding events so they move together), plus runtime review.
26. **Complete Add Track choices.** Count is bounded by remaining project capacity; mono/stereo and mono Left/Right input channel are source-connected. Add Instrument Track now lists the scanned VST3 instruments and loads the chosen one as the new track's first insert, so it is saved and reloaded with the project; an empty catalog says where to scan. Unavailable speaker layouts and preset browsing are still labeled. Runtime review of loading an instrument from this dialog remains open.
27. **Unify command execution.** Split at Cursor, Move to Cursor, Crossfade, Duplicate, Mute/Unmute Events, Select All/None, Select in Loop and Set Left/Right Locator each use one operation across the Edit menu and the reference key commands (Ctrl+D, Shift+M, Shift+U, Alt+M, Ctrl+A, Ctrl+Shift+A). Split at Cursor now splits a selected group segment through the group so every member keeps the shared boundary. Audio events carry a persisted `muted` flag: a muted event keeps its position, trim, fades and gain but produces no audio and draws dimmed with an `M` marker. Repeat..., Fill Loop, Range > Crop and Select > Invert are implemented with bounded core commands (`repeat_audio_clips`, `fill_loop_with_audio_clips`, `crop_audio_clips_to_range`) and their own menu entries; Repeat opens a count dialog bounded by `MAX_REPEAT_COUNT`. Crop trims the events crossing the loop and leaves events entirely outside it alone. Select > Events under Cursor and Move to > Left/Right Locator are also implemented. Event Lock/Unlock and grouping are implemented: a locked event keeps playing but refuses move, trim, split and crop (core refuses at the command level, and the arrange press selects instead of starting a gesture); events sharing a `group` move together, because `move_audio_clips` expands a grouped edit to its members, and clicking any member selects the whole group. Locked events draw an `L` and a dashed inset outline, grouped events a `G`. Lock/Unlock (Ctrl+Shift+L / Ctrl+Shift+U) and Group/Ungroup (Ctrl+G / Ctrl+U) share the Edit menu and the reference keymap. Still unavailable: Convert to Real Copy, the Project Logical Editor, Automation follows Events, Auto Select Events under Cursor and Enlarge Selected Track.
28. **Build regression around shared identity.** `selection_identity_tests` drives a real arrange click and asserts the clicked event becomes the selection and that `Project::selected_track` moves to that event's track, which is the field the Mixer, Channel Settings and Inspector all read. Touched workflows are covered; MIDI part and note selection are not yet in that regression.
29. **Complete recording resilience follow-up.** Take recovery from an abnormal exit is implemented and covered by tests, including header repair, loop/partial-frame trimming, placement on the original or a replacement track, and discard that keeps the file. Project recovery autosave now covers unsaved edits: while a project has unsaved changes a validated copy is written to `Metadata/Recovery/autosave.opr` at most every 20 seconds (never during recording), removed on a completed save or an explicit discard, and offered as "Recover Unsaved Changes" when the project is opened again. This also covers the window close eframe will not prompt on. Remaining: close defects found by a real kill-during-recording and kill-with-unsaved-edits review and confirm both prompts on hardware.
30. **Make one editing policy real.** Preferences > General now owns the recovery autosave: an on/off switch and a 5–300 s interval. Both persist through the existing `metering-preferences-v1` storage, sanitize to that range, and drive `maybe_autosave`, so turning it off writes no copy and a shorter interval writes one sooner (proved by an app test). Older preference files without the fields keep recovery on at 20 s. The remaining Preferences pages stay labeled planned.
31. **Choose project interchange.** Scope a named CPR version adapter against real fixtures, preserve unknown structures where possible and report unsupported data. Do not claim support without round trips.
32. **Consolidate and re-plan.** Done for this revision: [consolidation-2026-11-01.md](consolidation-2026-11-01.md) reconciles all 32 steps with their evidence, measures the UI inventory (68 wired menu commands, 176 declared-but-disabled entries, 52 of 152 keymap ids dispatched) and the coupling change across the 7 commits, and orders the next revision. Its conclusion is that source is ahead of evidence: runtime acceptance passes lead, then Repeat/Crop/Invert, then Linux VST3 editors.

## Parallel work and integration

Use up to three implementation/research workers and one integration owner when the session allows. Parallelize only packets with disjoint file ownership and agreed interfaces. The integrator owns `app.rs`, shared project schema and audio graph changes unless a lane has an explicit temporary handoff. Keep tightly coupled menu/command work serial. Workers return a focused diff, source snapshot, checks, known limitations and a concrete integration note; the integrator reviews and verifies the combined result.

Prefer worktrees from a recorded baseline when the current user tree can be preserved cleanly. Never reset, clean or overwrite uncommitted work to make parallelism convenient. If isolation cannot be done safely, serialize the conflicting files and parallelize read-only review or independent packet preparation.

## Gates for every feature step

- Name the behavior before implementation; list source status separately from tested behavior.
- Identify state ownership, downstream consumer, persistence, undo/redo, cancellation and failure semantics.
- Keep project schema changes backward compatible for supported OPR/OTP/RWP inputs.
- For audio, protect callback limits and verify generated signal before real-time listening.
- Preserve WASAPI and the user's local ASIO build option.
- Close the running app through its UI before rebuilds; keep only one instance open.
- Record automated, runtime, hardware/listening and user-acceptance results separately.
- Update the task packet, inventory and current-work handoff; stop for the user's review at the stated feature gate.

## Ordering rationale

The first extraction follows the T071 review and source audit, so it is grounded in a completed current task and actual coupling evidence. Later extractions happen beside the features that need them. That lets OpenTrack keep gaining usable workflows while `app.rs` gradually becomes a coordinator, rather than paying for a large speculative rewrite or allowing the monolith to grow untouched.
