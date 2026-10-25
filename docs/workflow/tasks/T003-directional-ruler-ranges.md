# T003 — Draw loop/skip ranges and correct the visible Bar snap

User feedback: the user reported "it loops fine" and then "skip also worked" for the delivered T003 build. This confirms their functional loop/skip check; it does not specify an audible-media fixture, exact boundary timing, save/reopen, or full visual parity. Their requested appearance refinements are implemented in T004. Snap was not explicitly confirmed in that feedback.

- Status: review_pending; source/build checked; desktop behavior and user acceptance pending.
- Actual implementer: GPT-6.1 Sol. Independent visual review: pending.
- Scope: transport-v1; loop-range, skip-range, range-endpoint-snap, cycle-audio, skip-audio; corrective T002 snap-release work.
- Authority: user requested implementation with OpenTrack kept closed, then a handoff for their check. This supersedes the previous packet's desktop-review prerequisite.
- Evidence: recorded user intent in workflow/README.md; existing toolbar source says Grid Type `Bar`, with separate Quantize Type `1/16`. Exact reference range endpoint snapping, skip audio semantics and pencil shape remain unmeasured.

## Implemented behavior

- The top 7 logical pixels of the timeline ruler form a range-drawing strip with an original pencil cursor. The remaining ruler handles normal seeking/zoom. Right-click still uses the ruler menu.
- Left-to-right draws one purple loop range; right-to-left draws one orange skip range. Preview follows the pointer, including captured release below the ruler. Range drawing preserves the playhead, supports edge scrolling, and commits once on release. Escape cancels. Redrawing identical bounds/mode does not dirty the project.
- Snap enabled uses the visible Bar grid for the transport cursor and range endpoints. Snap disabled leaves fractional positions. The separate `1/16` quantize field does not govern this ruler grid.
- Locators and direction are persistent core project data. One range edit participates in the existing Undo/Redo history. OPR v16 records the mode; old supported files migrate with Loop as the default. The saved EXAMPLE was not opened or modified.
- Cycle enables/disables execution. Loop wraps end to start; Skip jumps across the half-open interval [start, end). The audio callback applies boundaries per output frame, including within a buffer. The no-engine clock follows the same mode. This skip convention is an explicit implementation choice pending reference/user review, not a measured reference claim.
- Transport locator displays preserve drawing direction in Skip mode. Ranges are dimmed when Cycle is disabled. The timeline runway grows with navigation/range bounds.

## Checks and limits

- Existing x64 build checked before product edits. Final x64 release build succeeded with no OpenTrack process running. The application was never opened during this slice.
- Core tests: 8 passed. Audio tests: 25 passed, including per-buffer loop/skip boundary checks.
- App tests: 28 passed; the existing `theme::tests::drawn_surfaces_and_text_follow_both_themes` failed at theme.rs:213. This is the same unrelated theme failure recorded before this slice.
- New headless UI tests drive the actual `show_project` input path: continuous seek then Bar snap; Snap off release; both range directions; preview without project mutation; captured release outside the ruler; identical redraw; Escape; Undo/Redo. These tests create no native application window.
- New OPR test writes/reads a disposable temporary file and checks range/mode persistence plus legacy v15 default/migration. Focused persistence test passed again after the format increment.
- Formatting passed. Runtime listening, Windows pointer behavior, screenshots, no-change close, and visual/reference parity remain pending. No runtime or parity score is awarded for these headless checks.
- Mixed scrub/zoom still locks to one axis. Adaptive ruler labeling and playback follow remain separate gaps. Dropdown grid/quantize choices remain fixed.

## Build identity

- EXE SHA-256: `39846420C9B1FABEC70651448FE78DBC06BEF63478F990B460747C1F6915B4B4`.
- Base commit: `cbd9835`, plus the current uncommitted transport changes.
- `app.rs`: `6AEA5BC16274331E8FFB4DF12EBCA1C9EBE0753CE66874BAD3269C5E52AF9A1B`.
- `timeline.rs`: `033A05E24A9A9BD29DDDD6F0F0CEC7D35B6E8A1BE2326986E1E27F84CBD637C7`.
- `project_file.rs`: `94C9956EB106406FE88C467C3607A2CA9C996B89D3DD2D90D085A97E71227144`.
- Core `project.rs`: `DE84304607D308E8E1400488536BCB7F87223D5BF19D0356EC3E49DDAC9CA33C`.
- Audio `lib.rs`: `EB4F3EA3AF2C5E3929FA11183764372CFCD281B09C58797E105D3C2D4AB0940C`.

## User acceptance steps

1. Open `builds/x64/OpenTrack.exe` and load EXAMPLE. Enable Snap. Click/drag in the lower ruler area, then release between bar lines: the marker should land on a bar boundary. Disable Snap and repeat: it should remain at the pointer's fractional position.
2. Move to the very top edge of the ruler: the pencil should appear. Drag left to right over several bars: a purple range should preview and remain on release, without moving the playhead.
3. Enable Cycle and start playback before the region's end: playback/cursor should return to its start. Disable Cycle: playback should pass through.
4. Draw right to left: the region should turn orange. With Cycle enabled, playback should jump from its earlier boundary to its later boundary. Turn Cycle off and verify ordinary playback.
5. Undo/Redo the range; cancel a new drag with Escape. Try both edge holds and release outside the ruler. Use Save As to a disposable OPR for save/reopen checks.

Stop here for user feedback. Do not launch the application to perform these checks on the user's behalf under the current instruction.
