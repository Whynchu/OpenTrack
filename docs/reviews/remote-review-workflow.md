# Remote OpenTrack review through Sky

The working Windows desktop entry point is the Computer Use skill's `@oai/sky` package in the persistent `node_repl` tool. The browser `cua_repl` wrapper can return `apps: []` while Sky still has working desktop access. That browser inventory is not evidence that native Computer Use is unavailable.

Read the installed Computer Use `SKILL.md`, its guidance, API and confirmation rules before app automation. Discover `mcp__node_repl__js` through the tool catalog, then initialize:

```js
if (!globalThis.sky) {
  const { sky } = await import("@oai/sky");
  globalThis.sky = sky;
}
globalThis.windows = await sky.list_windows();
nodeRepl.write(JSON.stringify(windows, null, 2));
```

Select exactly one returned main window by its executable path and title. Old OpenTrack builds can have identical titles. Bind with `sky.get_window`, activate, and observe with `sky.get_window_state`. Use a fresh screenshot or accessibility observation before each state-derived action and refresh after it. Use the supported Sky APIs for mouse, keyboard, dialogs, launch, and capture.

Some observations include a 16×16 icon screenshot before the main screenshot. Choose the screenshot whose dimensions correspond to the intended window; do not assume `screenshots[0]` is the main image. Use that screenshot's ID for coordinate input. Keep raw capture dimensions and origin in the manifest, including any discrepancy from the reference frame.

Before rebuilding, close the exact running executable being replaced. Multiple old build-cache and packaged executables were running on 2026-10-23. Closing those windows through Sky resolved the Cargo removal and package-copy locks. The user explicitly confirmed those old scratch sessions were disposable and authorized closing them all. Future unsaved work must be assessed from the current session's authorization.

Store review artifacts outside Git in `../../_analysis-tools/reports/opentrack-comparisons/`. The Node session may only write to its staging folder, `nodeRepl.tmpDir`; write the requested screenshot/manifest artifacts there, then use an authorized filesystem copy into the report directory. Saving an artifact is distinct from recapturing or re-emitting screenshots solely for inspection.

The live 2026-10-23 pass verified native window selection, title-bar close, unsaved-state handling, launch, Project menu actions, track creation, RWP save/reopen, record enable, and Project-child drag. Hover timing/placement and the remaining M2 acceptance checks are still unverified. The Cubase-specific hover adapter is not an OpenTrack control tool. Use the existing reference captures; Cubase was not launched.
