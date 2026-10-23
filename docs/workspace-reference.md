# Workspace behavior reference

The local Operation Manual's Customizing chapter (printed pages 469-470) describes these behaviors:

- A workspace records window size, position, and content. The examples also call out zoom and track height.
- Changes to an unlocked active workspace are stored automatically as windows are opened, closed, moved, resized, or adjusted.
- A locked workspace is recalled in its saved form the next time it is activated, even if the current screen layout was changed while it was active.
- Workspaces belong to a project. Workspace presets are global and can be copied between the global preset list and a project's workspaces.
- Global presets store main-window state only; project-specific windows are excluded.
- The `Auto Instantiate Presets` preference turns global presets into workspaces when a project is created or opened.
- Workspace shortcuts 1-9 address list order by default. With `Use IDs`, each workspace can be assigned an ID 1-9, and shortcuts address that ID instead.

The Rust model in `workstation-core::workspaces` records these rules without tying saved state to a GUI framework. The app stores up to nine named per-project workspaces, recalls locked layouts, and exposes the `W` organizer for naming, reordering, recall locks, and shortcut IDs. Alt+Numpad1..9 selects by list order unless `Use IDs` is enabled; Alt+Numpad0 locks or unlocks; Ctrl+Numpad0 creates a workspace. The organizer can save the active layout as a global or project preset, copy global presets into the project, and create a workspace from either list. Global preset names and the auto-instantiate preference persist in application settings; a project workspace records which global preset created it, preventing duplicates on reopen.

OpenTrack- records the main window and these project-specific native windows: Mixer, Channel Settings, MediaBay, and MIDI Editor. Their open state, outer position and size, and client-area size are restored from the active project's workspace. Older snapshots without client dimensions fall back to the saved outer size. This is an implementation choice based on the manual's window and layout behavior; the manual does not establish pixel measurements for these OpenTrack- windows. Cubase reference screenshots, exact window roles, multi-monitor behavior, and DPI-aware comparisons remain outstanding. Monitor IDs are currently not recorded, so restoration across changed monitor arrangements is not yet verified.
