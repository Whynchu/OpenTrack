# OpenTrack roadmap revision 4: finish useful menu workflows

Status: prepared, 2026-11-02. Planning baseline: `12a01e0` on `main`, matching `origin/main` after a fast-forward pull. Prepared by Codex; intended implementation runner: OpenCode using DeepSeek V4.1 Flash (record the actual model identifier reported by the runner). This document authorizes no claim that implementation, runtime review or user acceptance has happened.

## Starting evidence and scope

Revision 3 continued through slice 56, beyond its original close-out. The latest README and handoff report 71 declared disabled menu leaves, 147/152 bound command IDs with dispatch, OPR 33, app 500 passed, core 146 passed and audio 166 passed. These are inherited results, not rerun during this planning pass. Older summaries in current.md, revision 3 and the accessibility report conflict with these later entries. Reconcile against source in step 1.

The remaining five bound IDs are devices.video, edit.open_score_editor, project.open_tempo_track_editor, transport.retrospective_record and transport.use_external_sync. None should receive a placeholder dispatch just to improve a count. The current coverage helper searches source text for match arms; it does not prove a command reaches working behavior.

The main-menu source still declares disabled pre/post-roll commands, MIDI Reset, Drum Map Setup/Open Drum Editor, Logical Presets, CC Automation Setup, FX/Folder/Marker/Ruler/Arranger track choices, offline plug-in processing, tempo import/export/editor, score layout, external sync, video and legacy interchange. TrackKind currently contains Audio, Group, Instrument and Midi only. Existing marker and region data can support real lanes without pretending they are audio tracks.

Goal: complete bounded musical workflows behind selected disabled entries, establish the foundations required by the larger ones, and improve confidence in recent media changes. This is a 32-step queue, not a promise of 32 new enabled menu rows or universal shortcut coverage. Count actual paths; enabling an equivalent context route must be recorded separately.

## Execution contract for OpenCode

- Read AGENTS.md, docs/workflow/README.md, architecture-agreement.md and the two repository implementation/review skills before edits. This revision is the active planning authority; old paused checkpoints remain historical.
- Execute serially. Each step is one bounded packet; split it further when necessary while retaining its step number. Do not combine unrelated work or reformat whole source files. Keep new responsibilities in focused Rust modules with explicit inputs/actions; app.rs composes them.
- Before each step, create a packet from tasks/TEMPLATE.md with the behavior below, exact file ownership, dependencies, refusal/cancel semantics, meaningful acceptance checks and review status. Define these before implementation. A dependent step is ready only when the required foundation works.
- Preserve all existing files, including untracked recovery-20261028-204244/ and stress-bench/. Never reset/clean the tree. Commit only changes belonging to the slice. Stop on unexpected overlapping work and reconcile it.
- Use one command path for menus, shortcuts and gestures. Content edits get one labelled history operation, true no-ops get none; transient view/selection state must not dirty content. Validate the entire edit before publishing it.
- File operations run away from audio callbacks and UI frames, with bounded jobs, cancellation, progress, stale-result rejection and cleanup. Original sources and files required by undo/discard must survive. Never publish half a multi-file edit.
- New persisted fields require validation, defaults/migration and the native version policy. Increment from the actual current format, never a version copied from this plan. Older-file and save/reopen checks accompany schema edits.
- Run focused checks for each slice and affected regressions. Do not weaken assertions or silently relabel product failures as harness failures. Record exact results and known baseline failures. At phase boundaries, run the relevant complete suites.
- Build product changes through the platform build script. On Windows retain x64 local ASIO and verify build-info.json; a no-ASIO cargo binary must not replace the user's executable. Documentation-only steps need no release rebuild. Keep one app instance and close through its UI before replacing a build.
- Update the packet, README status, measured inventory and current.md after each slice. Keep current.md concise: active revision, latest checkpoint, open issues and next ready step; move detailed historical prose to a linked handoff before replacing it.
- Checkpoint each verified slice in a local commit. Push only if the human has authorized publication for this run; authorization to pull is not authorization to push. Do not claim an independent reviewer or another model was used unless it actually was.
- Runtime, listening, hardware and user acceptance remain separate. If this host cannot perform them, record the exact pending procedure and continue only independent work. A detected correctness failure blocks dependent work. Two failed repetitions without new evidence require a new approach or a named blocker.

## The next 32 steps

### A. Establish a trustworthy baseline

1. **Reconcile the menu and command inventory.** Enumerate main and context menu leaves, including state-disabled versus unimplemented rows; record full path, shared command, gate, consumer and source snapshot. Add a reproducible inventory command/report and strengthen the shortcut audit with behavior checks for touched commands. Correct the accessibility report's stale claim of 43 missing shortcuts and misleading statement that all missing commands are available through menus. Check: deterministic counts, empty-project shell, generated menus and duplicate paths accounted for. Do not count disabled empty states as missing features.

2. **Review the recent media pipeline before extending it.** Inspect slices 41-56 for locked clips, multi-selection partial failure, source identity/range/channel conversion, source-to-pool mapping, undo/discard file lifetime and stale worker results. Audit pool_record_folder against absolute/UNC/drive-relative paths, traversal, Windows separator/ADS forms and existing junction/symlink escapes on the active host; ensure actual destinations stay in the intended project directory. Exercise generated WAVs and a failed second item. Fix confirmed defects as separately described commits; report findings even if none are found. Check: no original overwrite, no partial committed project, rejected paths create nothing, undo/reopen retain usable media.

3. **Establish the command boundary for this revision.** Trace selection, project history and transport scheduling used by the upcoming features. Extract only the cohesive command/state needed for steps 4-6; keep app.rs as composition. Record source ownership and add behavioral regression checks for the moved commands. Check: menu and key dispatch produce the same result, focus guards remain effective, no broad refactor or dependency addition.

### B. Finish pre/post-roll and basic MIDI control

4. **Implement a bounded pre/post-roll transport contract.** Use persisted application preferences for nonnegative roll lengths in beats (0-64, default 4). Pre-roll starts before a target and ends at it; post-roll starts at a target and ends after it. Clamp at project start, reject missing selections, use ordinary engine transport/stop scheduling, and cancel the scheduled target on manual stop/seek, project close or conflicting playback command. Specify cycle and recording interaction; refuse active recording and conflicting cycle until supported. Check targets within an audio block, stop/cancel and beat-to-sample conversion. Do not implement a UI-only countdown.

5. **Enable Pre-roll to Selection Start/End.** Wire both disabled entries to step 4, using the relevant existing selection resolver with documented audio/MIDI scope. Provide labelled roll-length controls and a reason when the selection cannot resolve. Check missing/zero-length selection, near-zero target, both boundaries, focus and actual engine completion. Transport changes must not create project undo steps.

6. **Enable Post-roll from Selection Start/End.** Wire both entries to the same scheduler, seeking to the named boundary and stopping after the configured interval. Check manual interruption, empty selection, end-of-project handling and independent pre/post settings. Review all four entries together; keep hardware listening pending if unavailable.

7. **Implement MIDI Reset.** Release held live notes and sustain on the active MIDI destination, including virtual keyboard state; send supported All Notes Off/All Sound Off/Reset Controllers through the existing bounded MIDI path. Preserve recorded project notes/CC and unrelated destinations. Check held-note, pedal, no-device, close/reopen and repeated reset cases; listen for stuck notes when a device is available.

### C. Drum editing and useful MIDI tools

8. **Add the drum-map model.** Define a bounded project-owned mapping of MIDI pitches to names and display order for MIDI/Instrument tracks. Default is identity: this first version changes presentation, not sounding pitch. Validate pitch uniqueness, names and limits; selection refers to stable note IDs. Check round trip, malformed data and legacy defaults. No silent GM naming claim without a supplied mapping.

9. **Enable Drum Map Setup.** Create, rename, reorder and remove mapping rows through validated commands; Apply commits one history entry and Cancel changes nothing. Explain identity playback mapping in the UI. Check duplicate pitches, empty/default map, switching tracks while editing, undo/redo and save/reopen.

10. **Enable Open Drum Editor.** Draw mapped pitch rows using the existing MIDI part/note model and shared editing commands. Support add, select, move in time, velocity edit and delete; preserve note duration and channel, and show unmapped notes so data never disappears. Check focus guards, grid/snap, multi-selection, Escape cancellation, empty parts and shared selection with the Key/List editors. Keep changes within a focused editor module.

11. **Enable bounded Logical Presets.** Ship a small explicitly named set: select by pitch range, select by velocity range and set selected-note MIDI channel. Show the matched scope and apply through existing selection/transform commands; no executable scripting engine. Content transforms are atomic, selection-only presets leave dirty state unchanged. Check no match, all/selected notes, channel limits, locks where supported, undo and non-target CC preservation.

12. **Enable CC Automation Setup.** Scope this entry to configuring visible/editable MIDI controller lanes, not VST parameter automation. Reuse the existing CC editing/playback path; choose CC 0-127, label the lane and persist the intended view policy in the proper owner. Check selecting a lane does not invent controller data, existing CC remains visible, edits survive save/reopen and engine/MIDI export preserve values. If the current editor cannot consume multiple lanes, implement that consumer before enabling the entry.

### D. FX channels as real signal destinations

13. **Define and persist FX-channel identity.** Introduce a distinct FX channel kind with no audio/MIDI events, normal insert chain and fader. Reuse group DSP where semantically valid but retain distinct type/labels and validation. Audit every TrackKind match, duplication, deletion, history, presets and legacy loading. Check migration and that invalid clip content is rejected rather than silently dropped.

14. **Route sends to FX channels.** Extend typed destinations and graph preparation, sharing group feedback/cycle/latency handling. Preserve the existing routing spelling compatibility and refuse dangling or cyclic targets. Check pre/post sends, bypass, gain, dry plus wet sum, feedback refusal and both engine paths plus offline export using generated signals.

15. **Enable Add FX Channel throughout the UI.** Main and context menus create the same channel through one command; support count/name and optional existing-catalog VST3 selection. Show the FX channel in Mixer/Inspector with compatible routing controls; unavailable catalog entries stay unavailable. Check add/delete/undo, send-target removal, loading a missing plug-in and save/reopen.

16. **Review the FX workflow end to end.** Exercise create FX, insert effect, send from two tracks, automate supported volume only if the engine supports that target, save/reopen, bounce/export and undo removal. Establish supported automation behavior explicitly; never enable unsupported Read/Write by analogy. Run routing/audio regressions and produce a concrete listening procedure. A failed graph invariant blocks dependent routing work.

### E. Folder tracks with coherent selection and history

17. **Add the folder hierarchy model.** Use stable parent IDs and bounded nesting (maximum 8 levels); folders hold no media or DSP. Reject cycles, dangling parents and children assigned to nonfolders. Define deletion as removing the folder while retaining/reparenting children unless the user explicitly selects a delete-contents action. Check migration, hierarchy validation and no audio graph membership for folders.

18. **Enable Add Folder and hierarchy commands.** Add folder, move selected tracks into/out of a folder and reorder a contiguous subtree through single atomic commands. Preserve stable IDs, routing and event selection. Check mixed selections, nested moves, refusal rollback, duplicate behavior and undo/redo.

19. **Render and fold folders.** Show indentation, folder headers and per-folder collapse in Project; extend Expand/Collapse All to folders without losing existing group behavior. Hidden child tracks continue to play and record; visibility belongs to view/workspace state unless existing policy requires otherwise. Check scroll/row hit testing, automation lane heights and restoration after reopen/workspace change.

20. **Review hierarchy across views.** Ensure Project, Mixer, Inspector, keyboard navigation and selection commands agree on identity when children are hidden. Define hidden-track navigation consistently; collapsing must not delete selection or mute audio. Check subtree drag/reorder, deletion with child sends, history and no data loss on save/reopen.

### F. Expose existing markers and regions; build the arranger foundation

21. **Enable a real Marker lane.** Provide a dedicated timeline lane backed by Project.markers, sharing the existing marker list and slot commands. Create, move, rename and delete with snap, one history operation per gesture, Escape rollback and stable identity. Treat the lane as a timeline presentation rather than a fake audio channel. Check marker slots, insert/cut time, ruler alignment, workspace visibility and save/reopen.

22. **Expose Regions as editable ranges.** Add a region lane/list using Project.regions: select, locate, rename, move/resize and delete, preserving hitpoint-created regions. Validate bounds and operation scope; region selection must not masquerade as audio-event selection. Check overlaps, minimum length, snap, cancelled drag, undo and insert/delete-time behavior.

23. **Enable additional Ruler presentation.** Add a configurable secondary ruler lane using the existing time-format/conversion functions. Persist view settings in workspace state, share zoom/scroll and honor timecode offset. Do not add musical tempo/signature changes in this slice. Check bars/seconds/samples/timecode alignment at multiple sample rates and zooms, seek behavior and workspace round trip.

24. **Add the arranger data contract.** Define named arranger sections and an ordered play sequence referencing stable section IDs, bounded to 128 sections and 1024 sequence items/repeats in total. Sections are half-open beat ranges; repeats are positive integers. No destructive rearrangement of source clips on edit. Validate deletion/reference policies, recursion prohibition, migration and maximum expanded duration. Prepare commands in a feature module; leave Add Arranger disabled until a usable editor and audible preview exist.

25. **Implement the arranger editor and commands.** Create sections from a region/locator range, rename and resize them, append/reorder/remove sequence items and change repeats. All content changes are atomic and undoable. Display source versus sequence position explicitly and preserve ordinary timeline mode. Check missing references, empty sequence, cancellation, selection changes and reopen. Source editing during active preview must stop/rebuild it or be refused clearly.

26. **Render an arranger sequence offline.** Prepare an immutable sequence playback plan off the callback using existing graph/render paths; preserve section-local audio offsets, MIDI notes/CC and volume automation across repeats. Clip note lifetimes to section boundaries and define discontinuity/reset policy. Start with the current constant-tempo model. Check exact length, ordering, repeated offsets, silence, boundary notes and generated-signal output; refuse unsupported constructs instead of flattening them silently.

27. **Connect audible arranger preview.** Share the prepared plan and boundary policy with live transport, with bounded callback behavior and an explicit source/sequence position display. First version is playback-only; refuse recording/punch and conflicting cycle mode during preview. Stop/cancel returns cleanly to ordinary timeline playback with no hanging MIDI notes. Check block boundaries, seeking between items, rapid stop, project close and offline/live scheduling agreement.

28. **Enable Add Arranger and review the complete path.** Main/context entries create or reveal the single arranger lane/editor; no fake ordinary track. Exercise sections, repeats, sequence playback, undo, save/reopen and export using the same sequence choice. If live preview cannot meet step 27, retain the relevant disabled entry and report the exact blocker; the offline renderer alone does not fulfill this step. Record listening/runtime evidence separately.

### G. Portable track packages and close-out

29. **Specify and export OpenTrack Track Archive v1.** Scope File > Export Track Archive to OpenTrack's own documented package, with an explicit signature/version; do not claim Cubase Track Archive or OMF compatibility. Use a directory package with a manifest and copied media to avoid a new archive dependency. Include selected tracks, referenced media, supported hierarchy/routing, MIDI and plug-in state; explicitly report unsupported/external dependencies and route links outside the selection. Stage/copy away from the callback and publish only a complete package. Check collisions, shared-source deduplication, cancellation and source preservation.

30. **Enable Import Track Archive with a real round trip.** Validate manifest/version, bounded sizes, relative paths and actual file containment before copying. Allocate new stable IDs and remap every internal reference; external routing requires explicit resolution or disconnected status. Apply one atomic history operation after preparation. Check import into a nonempty project, duplicate IDs/names, malformed/path-escape packages, failed copies, undo/redo/discard file lifetime and fresh-process save/reopen. Enable the entry only for the implemented OpenTrack format with clear file-dialog wording.

31. **Run the revision acceptance and compatibility pass.** Run affected full suites, formatting/diff checks and the platform release build; verify local ASIO on the Windows deliverable. Exercise new menu paths, keyboard focus, history, old OPR migration, reopened projects and cancellation/failure paths. Record source/build hash, exact test commands/results and pending desktop/hardware steps. Review the actual diffs; self-review must be labelled self-review. Do not report user acceptance or a separate reviewer that did not happen.

32. **Consolidate and prepare the next queue.** Regenerate menu/shortcut inventory from the final source, reconcile README/current/task packets and measure the added module boundaries/app.rs growth. Report delivered, partial, blocked and runtime-pending work separately, with next actions. List the remaining score, tempo/signature, transpose, video, external sync, retrospective record, Control Room/MMC, offline plug-in stack, legacy interchange and platform gaps. Record which need fixtures, devices or design decisions. Update the runner handoff; do not append unbounded post-close slices to this revision.

## Dependencies and pause rules

Steps 1-3 precede feature edits. Transport 4-6, MIDI 8-10, FX 13-16, folders 17-20, arranger 24-28 and archives 29-30 are ordered chains. MIDI 7/11/12 and marker/region/ruler 21-23 can proceed if an unrelated chain is blocked. Step 29 must state whether unfinished new track kinds are excluded. Steps 31-32 always report the actual outcome, including incomplete steps.

A schema, timing or graph decision beyond these contracts requires a written decision in the packet and architecture review before dependent changes. If no independent reviewer is available, name that gap and stop the dependent high-risk chain; continue a ready independent step. User listening checkpoints resolve audible/interaction questions, not routine implementation choices. Never interpret inability to run the desktop as a pass.

## Explicitly deferred in revision 4

Tempo/signature maps and their import/export/editor need one beat/sample conversion contract across recording, automation, DSP, MIDI export and rendering; an editor-only tempo curve is insufficient. External sync/MMC needs protocol, clock source and device fixtures. Retrospective capture needs bounded buffering, timestamp/recording/privacy behavior and a deliberate user workflow. Score engraving/video/Control Room are separate products or engine workflows. OMF/CPR compatibility needs real named-version fixtures; track archive v1 above is an OpenTrack format only. Offline plug-in processing needs its own ordered, persisted, cancellable processing-stack contract. First-party VST3 effects/instruments remain individually designed with the user.

These omissions are deliberate scope boundaries. Keep the relevant entries honestly unavailable and report them; do not delete labels or add aliases just to hit a count.
