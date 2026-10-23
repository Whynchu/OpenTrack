# Luna continuation checklist

Use this short rubric to continue OpenTrack safely and produce reviewable results. Read AGENTS.md first; this guide supplements it.

## 1. Establish the current slice

- Work in `G:\dev\Opentrack\OpenTrack-`, not its parent folder.
- Read `docs/fidelity-plan.md`, `docs/basic-track-workflow.md`, `docs/window-and-track-fidelity.md`, `docs/reviews/L1-menu-and-project-capture.md`, and `docs/reviews/tooltip-sweep-log.md`. Read the relevant L2-L4 notes linked by the plan.
- Read `docs/reviews/remote-review-workflow.md` and the external `M2-review-report.md` before resuming desktop work.
- Check the actual working tree and recent commits. Preserve existing changes. Do not reset or pull over unfinished work merely to start from main.
- Name one bounded slice: reference IDs, visible mismatch, source location, and acceptance actions. Finish that slice before expanding scope.
- Resolve the latest user feedback against the saved review report. Do not infer milestone approval from a successful build or a general positive comment.

## 2. Use the working desktop hook

Use the installed Computer Use skill and its `@oai/sky` package through persistent `mcp__node_repl__js`. Read that skill's guidance, API, and confirmation rules before automation. Discover the tool through `ALL_TOOLS` if it is deferred. An empty browser `cua_repl` app inventory does not establish that SKY is unavailable.

Initialize once per Node session:

```js
if (!globalThis.sky) {
  const { sky } = await import('@oai/sky');
  globalThis.sky = sky;
}
globalThis.windows = await sky.list_windows();
nodeRepl.write(JSON.stringify(windows, null, 2));
```

Select exactly one window from returned objects by executable path and title. Then use `sky.get_window({ id, app })`. Do not invent handles or reuse old IDs after a dialog is recreated.

Use this loop:

1. Observe with `sky.get_window_state({ window, include_screenshot: true, include_text: true })` when both pixels and controls are needed.
2. Inspect the screenshot/tree before acting. Store the returned state and its `window`.
3. Perform exactly one action using that observation.
4. Refresh and inspect the result before the next action.

For coordinate clicks, choose the screenshot matching the intended window dimensions and pass its `id` as `screenshotId`. A 16 x 16 icon image can precede the actual window image; never assume `screenshots[0]` is correct. Coordinates must come from the observed image. Use `sky.click`, `sky.press_key`, and other documented SKY APIs; do not substitute custom Windows automation.

### Close an old build before replacing it

- Refresh `sky.list_windows()` and select the exact running OpenTrack executable. Multiple builds can share a title.
- Bring a blocking native file picker forward with `sky.activate_window`, then observe that picker. Cancel an Open dialog before closing its owner.
- Close OpenTrack through its visible close button or `Alt_L+F4`.
- If the in-app **Unsaved project changes** guard appears, inspect Save / Discard / Cancel. Discard only with current user authorization; otherwise preserve the work.
- The user explicitly authorized discarding the scratch session on 2026-10-23. That session was closed and a fresh window list contained no OpenTrack windows. This is historical authorization for that scratch session, not permission to discard future work.
- Refresh after an action has settled. An immediate list can still contain a closing window; a second fresh observation may show it gone. Do not conclude that a click only hovered from one immediate frame, and do not queue repeated clicks.
- Verify closure with a fresh window list before building or exporting over the executable.

If a modal is missing from the main capture, list windows and target its separately returned window. If activation/state capture fails, refresh selection and retry once as the skill directs. Report the exact error if recovery fails. For a cached-element error, reobserve; do not reuse the rejected index. Stop on an explicit user stop or a locked desktop.

## 3. Build the current x64 application

From the repository, run:

```powershell
.\build.ps1 -Architecture x64
```

The deliverable is `builds\x64\OpenTrack.exe`; `.build-cache` holds intermediates. Launch that exact executable through SKY and select its newly returned window. Do not launch an obsolete cached build. x64 is primary; do not expand to x86 unless the slice requires it.

If the environment is read-only, use the supported approval/escalation path for authorized writes and builds. Do not claim an edit was saved when a tool rejected it. Do not change global Git trust settings to bypass an ownership error; inspect the repository context and use a narrowly scoped command if appropriate.

## 4. Compare against the evidence

- Existing references: `G:\dev\Opentrack\_analysis-tools\reports\workflow-L1\`.
- Approved overall arrangement: `04-preferred-approved-0.png`.
- Treat `*-0.png` as primary, subject to explicit manifest/review exceptions, including separate Transport-menu and Compressor-editor captures.
- Use existing captures. Do not open Cubase for this task; the user will handle new reference collection when needed.
- Match display resolution, scaling, window arrangement, and control state. Record physical/logical dimensions and scaling separately; image size alone does not prove DPI.
- Capture OpenTrack before and after each slice. Save a side-by-side comparison or overlay, preserving meaningful dimensions and labeling any crops/resizing.
- Keep code and graphics original and in Rust. Preserve Windows light/dark following, shortcuts, enabled states, submenus, focus, and window reuse.
- Top-level menu labels have no tooltips. The tooltip sweep starts at the Project power icon, then follows the log left to right and top to bottom. Match observed text, hover target, position, and timing. Do not invent tooltips.
- Conflicting or missing evidence remains **Unknown**. Record it and raise it at the next review checkpoint.

Save captures, manifests, and comparison reports outside Git:

`G:\dev\Opentrack\_analysis-tools\reports\opentrack-comparisons\`

Use a coverage table:

| Reference ID | Control / tooltip | Expected position / size | Action / shortcut | Source location | Current capture | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Capture ID | Observed control | Measured facts or Unknown | Observed behavior or Unknown | Rust file / symbol | External filename | Missing / Implemented / Runtime verified / Parity verified / Unknown |

Keep observed facts distinct from inference. A build proves compilation, not visual or functional parity. Exercise the named acceptance actions, rebuild after changes, recapture, and correct visible mismatches before claiming the slice complete.

## 5. Deliver a review checkpoint

Show the comparison images inline in the conversation so the user can review them on a phone. A local file path alone is insufficient. Forward image content through the supported tool image output; do not print base64 payloads. Follow the Computer Use skill's capture/staging rules when saving artifacts.

Report briefly: what changed, what was exercised, what the comparison shows, and what remains unknown. Include the capture IDs and report path. Stop before the next major milestone and wait for the user's review decision:

- M1: menus, toolbar, power icon/tooltips, separate Transport panel.
- M2 / L2: Project, Inspector, track flow and controls/tooltips.
- M3 / L3: Mixer, Channel Settings, EQ and inserts.
- M4 / L4: recording, playback and workspace recall.

## Resume facts recorded on 2026-10-23

- All active OpenTrack windows were closed through SKY; the final fresh window list was empty.
- Inspector level, pan, and delay controls have earlier review artifacts in the external comparison folder.
- The current Add Audio source requests `Vec2::new(338.0, 128.0)` in `crates/workstation-app/src/app.rs`. A successful build was recorded, but its final dialog dimensions still require a fresh live comparison. The older comparison used a 338 x 160 OpenTrack crop against a 338 x 162 reference; do not present it as verification of the latest build.
- Read the external `M2-review-report.md` and latest user approvals before deciding whether M2 is accepted and M3 may begin.

## Done means all five

1. Named evidence and acceptance steps.
2. Bounded original Rust implementation.
3. Successful current build and relevant runtime checks.
4. Saved current capture, comparison, conditions, and coverage status.
5. Inline screenshots delivered and the milestone review gate respected.
