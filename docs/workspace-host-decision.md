# Workspace host evaluation

Status: proposal for L1 review; no architecture selected or implemented in this pass.

## Requirements from the preferred reference capture

Project, Mixer and Channel Settings appear as independent, overlapping windows inside the application workspace. Transport and the Compressor editor are separate floating surfaces. Preserve geometry, z-order, activation/focus, close/reopen behavior, keyboard routing, and workspace recall. Confirm containment, minimize/maximize, monitor and DPI behavior through interactions before claiming equivalent window semantics.

The current shell cannot represent this arrangement faithfully: Project is a dock tab, Mixer can appear in dock and native hosts, settings use native top-level viewports, and Transport is a fixed bottom panel. Tuning spacing in those containers would retain the wrong window model.

## Options to evaluate

| Option | Fit | Required proof / risk |
| --- | --- | --- |
| Eframe root with internal workspace windows, native floating surfaces for Transport/plug-in editors | Can render contained, overlapping Project/Mixer/settings using existing Rust/egui content without a new dependency | Must prove clipping, focus, keyboard routing, activation order, resizing, minimize/maximize and recall. Simulated chrome alone is insufficient; reject if observable requirements cannot be met |
| Native Win32 MDI workspace with floating editor/Transport windows | Closest candidate to the observed Windows child-window structure | Requires evaluation of eframe rendering/input ownership or a shell refactor. Existing windows-sys dependency supplies an OS boundary, not a complete integration solution |
| Current dock/native mixture | Keeps current prototype intact | Fails the observed container and Transport arrangement; not an accepted fidelity destination |

## Proposed first implementation boundary

Extract a presentation registry keyed by stable window ID, with host role independent from preset-copy class. Track coordinate space, parent, open state, bounds, z-order and minimized/maximized state explicitly. Keep audio/project state and content renderers separate from host choice. Avoid duplicating Mixer content across unrelated hosts.

After remaining reference focus/containment interactions are captured, evaluate both viable host options against the same acceptance sequence. A small host proof may precede the full UI rebuild, but it must demonstrate the requested behavior rather than justify a reduced target. Do not choose a framework solely because it is easier to compile.

Source seams: `app.rs` PanelTab/DockArea, NativeWindowState/show_viewport_immediate wrappers, fixed Transport panel; `workspaces.rs` WindowSnapshot and WindowClass; `project_file.rs` window snapshot serialization. WindowClass currently controls preset copy behavior and should not be overloaded to mean native/child hosting.
