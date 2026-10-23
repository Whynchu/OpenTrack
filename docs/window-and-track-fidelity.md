# Window and track fidelity

This note separates observed reference behavior, current OpenTrack- behavior, and parity work still needed. It is not a claim of one-to-one compatibility.

## Evidence available

- The local Cubase 5 Operation Manual, Customizing chapter (printed pages 469-470), says workspaces store window size, position, content, zoom, and track height. It also describes locked layouts, project workspaces, global presets, and workspace shortcuts. See [workspace-reference.md](workspace-reference.md).
- The same manual (printed pages 35-36) documents independent track-height resizing, Ctrl-drag to resize all tracks, relative vertical zoom, and track controls adapting to smaller heights. The Windows key-command table (printed page 486) lists Alt+Down, Alt+Up/Ctrl+Up, and Z/Ctrl+Down for the three track zoom commands.
- Cubase users report that Zoom Tracks Exclusive enlarges the selected track while shrinking the others, and that invoking it again restores the normal track view. This behavior is useful corroboration, but it still needs a controlled check against the installed Cubase 5 build: [Steinberg forum discussion](https://forums.steinberg.net/t/zoom-just-the-selected-track-key-command/89547).
- REA has been used to inventory the installed Cubase tree and record bounded file evidence outside the product repository. The inventory is saved under `../_analysis-tools/reports/cubase-install.evidence.json`.
- REA's current install inspection does not decode Cubase project internals. No Ghidra, Hopper, or IDA provider is configured for native binary analysis, and CPR's inner records remain unmapped.
- Initial Project/Mixer/Transport screenshots were captured on 2026-10-22 after Windows computer use became available. The full controlled interaction sequence and matched OpenTrack comparisons remain outstanding. See [basic-track-workflow.md](basic-track-workflow.md) for capture provenance and gaps.

## OpenTrack- behavior implemented so far

- The arrange view lets the user select a track and toggle Mute, Solo, Solo Defeat, and Record Enable in its header. The `e` control opens Channel Settings for that track.
- Track rows can be resized independently from their lower edge; Ctrl-drag resizes all tracks together. Compact rows hide the track-type caption and keep Mute, Solo/Solo Defeat, Record Enable, and Channel Settings controls available. Alt+Down/Alt+Up and Ctrl+Up change track heights while retaining their relative sizes. Z/Ctrl+Down toggles the selected track into an exclusive tall-row view and back to the saved heights.
- Per-track heights and the exclusive-zoom restore state are saved with the active workspace and restored when switching workspaces or reopening the project.
- Double-clicking a track name opens Rename Track. The Track menu can also rename, move the selected track up/down, add each implemented track kind, or request track removal. Removal confirms the track name, is disabled during recording, and can be undone.
- Mixer, Channel Settings, MediaBay, and MIDI Editor are separate top-level windows. Window open state and geometry are stored in the active project workspace. Global workspace presets store main-window state only, following the manual's project/global distinction.
- Channel Settings selects among project tracks, creates a track when opened in an empty project, and edits the selected track's built-in instrument choice, four-band EQ, and insert chain. Inserts can be added, bypassed, reordered, removed, and adjusted.
- Mute, Solo, and Solo Defeat affect the audio playback plan. Ctrl-clicking Solo clears other soloed tracks; Alt-click toggles Solo Defeat, which keeps that channel audible while other tracks are soloed. The manual describes both behaviors on printed page 119. Record Enable is consumed by the existing audio/MIDI recording paths, which currently have input-routing limits described in the [roadmap](implementation-roadmap.md).
- Visuals and application code are original. The current panels are not Cubase assets and their exact look has not been measured against the reference.
- The zoom step sizes and exclusive-row height are implementation choices; exact pixel values and the Cubase 5 toggle behavior have not been measured on screen.

## Capture plan for accurate replacement

For each reference workflow, record screen resolution, Windows display scale, application bounds, active project/workspace, open windows, and exact interaction sequence. Save before-and-after screenshots and note keyboard shortcuts and project-file changes. Begin with:

1. New project, add Audio/MIDI/Instrument tracks, rename, reorder, remove and undo, select tracks, and inspect header controls.
2. Open Mixer and Channel Settings from the menu, toolbar, keyboard command, and track header. Record whether each command reuses, focuses, or creates a window.
3. Toggle Mute, Solo, and Record Enable on different track kinds while playing or recording. Capture how simultaneous solos and armed tracks behave.
4. Open EQ and inserts, change parameters, reorder or bypass an insert, save, close, reopen, and compare restored state.
5. Move and resize each window, save the workspace, switch workspaces and projects, change display scale or monitor, and confirm restoration.

Use those captures to drive incremental Rust changes and a repeatable visual comparison at the same resolution and scale. Keep each measurement labeled as observed, inferred, or unknown. Do not claim exact parity while captures or behavior comparisons are missing.
