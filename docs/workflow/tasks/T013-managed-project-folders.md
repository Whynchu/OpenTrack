# T013 — Managed project folders and contained media

Date: 2026-10-24. Status: source implementation complete for the first pass; x64 release build passes and user runtime review is pending.

## User requirements

- Creating a project asks for a project name and parent folder, then creates a dedicated folder with its `.opr` file and organized subfolders.
- OpenTrack starts with no project: show the menu bar and a blank work area until the user creates or opens a project.
- Older projects without managed-folder metadata prompt the user to use their current folder or choose/create another folder. Existing project contents must not be overwritten silently.
- Directory selection uses the native Windows folder dialog, including an option to choose a parent and create a new project folder.
- Recording writes takes inside the project automatically; Record must not be the point at which the user chooses a destination.
- New project files use `.opr`. The main window title follows the `.opr` filename, such as `Project- EXAMPLE`.
- External WAV imports should be copied into the project; audio already within the project stays in place.
- An unchanged project should close without a save prompt.

## Source implementation

- OpenTrack starts with only the menu bar and a blank work area; the title is `OpenTrack-` and no Project, Mixer, transport, or status panel is shown.
- File → New first opens the native Windows folder dialog to choose where the project folder will be created, then shows **Create Project Folder** for the name and final parent. OpenTrack creates `<parent>/<ProjectName>/<ProjectName>.opr`, `Audio/Recordings`, `Audio/Imports`, and `Metadata`; it writes the initial project immediately. Existing project-folder names are rejected rather than reused. Canceling leaves no project open.
- Opening a native file without both the managed-folder marker and expected directory structure prompts **Set Up Project Folder**. Options are **Use Current Folder**, **Create Beside Project**, **Create New Folder** (choose a parent with the Windows folder dialog), and **Choose Existing Folder** (select the destination in the Windows folder dialog). Assigning a different location writes a new `.opr` and preserves the source file. Choosing the same folder for an old `.opr` explicitly upgrades that file in place. Old `.otp`/`.rwp` sources are preserved and receive a new `.opr` in the selected folder. Canceling the prompt unloads the project.
- The managed-folder marker is versioned in OPR version 18; earlier native files default to unmanaged and remain readable. New saves use the OpenTrack format marker; the reader accepts the previous native marker for backward compatibility.
- Record creates numbered takes such as `Audio/Recordings/Take 001.wav`, checking for collisions. Multi-track records create separate track-named WAVs from the same take number. No save dialog is shown when recording starts.
- Import WAV copies external files into `Audio/Imports` on a worker thread using a temporary filename and atomic rename. Sources already under the project folder remain in place. The UI prevents project switching or closing while this copy is active.
- Save, import, and recording are blocked until a folder is established. The app title updates to `Project- <filename stem>` after the project path changes. Closing without unsaved edits continues to skip the save prompt.

## Limits and unknowns

- `Metadata` is created and reserved; this slice still stores the `.opr` document at the project root and has not moved caches or generated metadata into that subfolder.
- Legacy project media that was already linked to files elsewhere is not copied during migration. Those clips retain their existing paths; new imports are contained.
- Media paths in the project are currently absolute. Moving a managed project folder after importing/recording may require relinking; portable relative-path relocation is not implemented.
- The new/legacy dialogs, filename title, write protection, reopen behavior, automatic take naming, and unchanged-close behavior have not been exercised in the desktop app. The user's saved EXAMPLE.opr was not opened or modified.

## User acceptance steps

1. Launch OpenTrack and confirm only the menu bar appears, no project or child panels are open, and the title is `OpenTrack-`.
2. Choose File → New. Confirm the native Windows folder dialog asks for the parent location. Create a project and check its folder, title, and initial `.opr`; close unchanged and confirm there is no save prompt.
3. Open a disposable copy of a pre-version-18 or unmanaged `.opr`. Verify the setup prompt offers current-folder, new-folder, and existing-folder paths. Choosing a different folder should leave the source unchanged and produce a new `.opr` plus the directory structure.
4. Import an external WAV and confirm a copy appears under `Audio/Imports`; reimport a same-named WAV and verify the existing file is not overwritten.
5. Arm an audio track and record a short take. Confirm `Audio/Recordings/Take 001.wav` is created, then make another take and confirm it increments. Check multi-track names if using multiple armed tracks.
6. Close and reopen the new `.opr`; check its title, track media references, and workspace state.

## Source/build identity

- Build command: `.\build.ps1 -Architecture x64`.
- Process check before build: OpenTrack absent; no instance was launched.
- Formatting and compile check: `cargo fmt --all` and `cargo check --workspace --target x86_64-pc-windows-msvc` pass. No tests were run.
- Release artifact: `builds/x64/OpenTrack.exe`.
- Release SHA-256: `50632E097BE3518FF49379575B40FA4C87EED2A549AADA414DE3DAC16C36FB13`.
- Source SHA-256: `app.rs` `470D8E9948D8428C540E2EAD334E4249829A8AD0DF648983E11D7ADA2BA81E47`; `project_file.rs` `B6347D91EF91AB8E346B5340D7D0DB075DB0F5F1C77827CD544C90FAA9A2F732`.
- T013 ledger/report: [project-folder ledger](../project-folder-ledger.json), [metrics](../project-folder-metrics.md).
