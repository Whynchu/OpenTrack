# Remove Empty Tracks

- Status: accepted for source and automated checks; desktop review pending
- Scope version and requirement IDs: post-revision-5 feature request (continue filling missing menu features)
- Desired observable outcome: Project > Remove Empty Tracks deletes media tracks that hold no events, MIDI parts or automation, in one undo step, without touching buses, folders, armed tracks or content.
- Reference evidence: `Project::remove_empty_tracks` and its test in `crates/opentrack-core/src/project.rs`; `OpenTrackApp::remove_empty_tracks` and its test in `app.rs`; the menu row in the Project menu.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: one core method built on the existing `remove_track` (which already re-parents folder and group children), one app command using the shared history pattern, and the menu row.
- Acceptance checks: empty audio and MIDI tracks are removed; a track with an event, a MIDI part or an automation lane is kept; an armed track is kept; groups and folders are kept and their children survive; the command is one labelled "Remove Empty Tracks" undo step; a second run reports instead of writing history; an invalid result is refused.
- Affected regressions: the Project menu row moves from unavailable to state-dependent; the disabled-row count falls from 69 to 68.
- Essential gates: undo, no content loss, folder/group integrity (`validate` after the edit).
- Review owner and milestone stop: self-review; the recording guard needs the desktop.

## Decisions

- **Buses and folders are never removed**, even when empty: they carry routing and organisation rather than content, and deleting an empty group would silently re-route its children.
- **An armed track is never removed**, because arming is a deliberate preparation for recording.
- A track with automation counts as non-empty, so automation is never discarded by a cleanup command.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Changes: `Project::remove_empty_tracks` (core), `OpenTrackApp::remove_empty_tracks` (undo + plan rebuild + status), menu wiring, core and app tests.
- Commands/checks and actual results: `cargo test -p opentrack-core empty_tracks_are_removed` -> passed; `cargo test -p opentrack empty_track_tests` -> passed; app suite 584 passed / 4 known device-dependent roll failures / 9 ignored; core 167; audio 171.
- Not claimed: the recording guard uses a live session that a headless test cannot construct, so that path rides on the desktop review.
