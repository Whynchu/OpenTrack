# T001 — Transport specification and baseline

Status: complete. Scope: transport-v1, all ledger requirement IDs. Preferred owner: Astra/Sol.

Outcome: completed source/evidence audit; T002 is ready, with later transport questions explicitly left for their own packets. This is an audit, not approval to redesign the application.

Read current.md, architecture-agreement.md, Project interaction study and checklist. Use existing captures/manifests first. The user's 2026-10-24 directions are the specification for normal drag, release snap, directional range selection, colors, and adaptive labels. They are not automatically measured Cubase evidence.

Inspect show_project, transport clock/seek/cycle consumers, core range state, grid selection, persistence and dirty tracking. Record precise source locations and actual implementation status per ledger requirement. Existing code may already fulfill some requirements; do not rewrite working behavior without evidence.

Questions: which snap settings govern ruler release, tie behavior, and clamping; which normal ruler hit zone excludes range drawing and scrollbar; how zoom anchor and seek update together; whether skip is enabled by Cycle and what playback does at each endpoint; whether range endpoints snap; how playback follow differs from drag-edge scroll. Resolve only questions needed for the next packet. Existing user-reported drag scrolling is a regression obligation, not proof for the current build.

Acceptance: each requirement has an assessed source status with evidence, or an explicit unresolved reason; T002 contains concrete grid semantics, code seams, and fixtures; broader unknowns have named follow-up packets. Do not set baseline_complete unless every implementation row is assessed. No whole-DAW percentage. Update ledger/report and current.md. Live observations require the current computer-use workflow and preserved reference project. Do not repeat the full Cubase sweep.

## Results

- Grid audit correction after the user's failed T002 check: the toolbar Grid Type is `Bar`; `1/16` is the separate MIDI Quantize Type. The initial audit incorrectly chose the latter for ruler snap. Current ruler snap uses `beats_per_bar` quarter-note beats. Both displayed dropdown choices remain hard-coded. See T002/T003 for the correction and pending user acceptance.
- Click and horizontal seek map continuously to ruler pointer; snapping was not applied. T002 applies the enabled fixed grid on click/release. Scrollbar and ruler share viewport content coordinates. Captured drags continue outside the starting point as long as the primary button remains down.
- Normal ruler gestures lock to horizontal seek or vertical zoom after a 3 px threshold. Mixed axes do not operate together. Horizontal edge scrolling has a 32 px zone, quadratic edge strength and elapsed-time speed capped at 720 px/s; viewport clamping, pointer remap, and 16 ms repaints are present. The user previously reported edge dragging works; this is a regression obligation.
- Middle drag source pans both axes. Labels currently appear every other bar at all zoom levels, so adaptive density is absent. A 256-bar runway supports navigation beyond bar 50.
- The app persists cycle locator positions and passes the enabled cycle range into audio-plan construction; a no-engine transport fallback wraps. The audio engine path needs playback-boundary and listening acceptance. No skip model, range drawing, or skip execution path was found. `autoscroll_enabled` has a control and persistence but no playback-follow consumer.
- Source locations: `app.rs::show_project`, `OpenTrackApp::rebuild_audio_plan`, `OpenTrackApp::advance_transport`; `project_file.rs::ArrangeViewState`; `Project::set_loop_range`; audio playback-plan cycle input. See ledger notes for exact findings.
- T002 is ready with snap resolution, disabled behavior (leave unsnapped), nonnegative clamp, and positive midpoint rounds forward. Whole click/drag-release acceptance remains runtime-pending.
