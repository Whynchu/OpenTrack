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
15. **Extract routing ownership.** Move route editing/validation coordination into a dedicated application module as the send path establishes a stable boundary.
16. **Build general typed buses.** Extend track/group/input/output routing only after send semantics and existing device routing are stable; preserve current simple track routing.

### Automation playback and editing

17. **Specify automation data and gestures.** Define lane identity, parameter targets, time units, draw/edit gestures, playback/read/write modes, touch/latch semantics if needed, undo and migration.
18. **Implement automation playback first.** Add persistent events and sample-accurate or documented block-boundary evaluation with no callback allocation, blocking or UI access.
19. **Add automation editing and recording.** Connect lanes and controls through shared commands, with bounded event publication and deterministic cancellation/recovery.
20. **Verify automation.** Test persistence, undo, playback boundaries and recorded control movement; listen in the running app and document CPU behavior.

### Recording, stability and completion

21. **Review mixer integration.** Check Mixer layout, Channel Settings, bus behavior, project migration, save/reopen and legacy projects. Treat runtime listening and user review as separate gates.
22. **Expand physical device routing.** Specify and implement supported input/output channel maps while preserving current stereo behavior and old project defaults.
23. **Improve recording reliability.** Recover incomplete takes with clear collision and restore/discard decisions; validate timing and persistence against real devices.
24. **Harden current plug-in workflows.** Audit VST3 scan/load/state/editor behavior and improve failure isolation based on evidence; document limits. New formats require a demonstrated user need.
25. **Complete a high-value edit family.** Use the command audit to implement split, duplicate, mute or range actions with shared selection, undo/redo and persistence.
26. **Complete Add Track choices.** Define supported channel/speaker behavior and instrument selection or label unsupported choices clearly; trace each selected option to track creation and processing.
27. **Unify command execution.** Connect menu, shortcut and editor entry points for a bounded command family to one operation; retain stable shortcut IDs.
28. **Build regression around shared identity.** Verify Project, Mixer, Inspector and editors agree on track/event selection for touched workflows.
29. **Complete recording resilience follow-up.** Close remaining abnormal-exit recovery defects from real-device review and verify restore/discard decisions.
30. **Make one editing policy real.** Choose a user-valued Preferences setting only when its consumer is specified. Persist it and prove it changes relevant Project/editor behavior.
31. **Choose project interchange.** Scope a named CPR version adapter against real fixtures, preserve unknown structures where possible and report unsupported data. Do not claim support without round trips.
32. **Consolidate and re-plan.** Reconcile the UI inventory, task packets and architecture agreement; measure whether the app boundaries reduced coupling; use evidence and user review to select the next roadmap revision.

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
