# Project edit history

OpenTrack- provides project-level undo and redo through **Edit > Undo/Redo**, Ctrl+Z, and Ctrl+Y. Ctrl+Shift+Z also redoes. A new project edit clears the redo stack. Opening or creating a project starts a fresh history.

## Current behavior

- A pointer gesture or keyboard command creates one project snapshot before the edit. Continuous controls such as a drag are grouped until the pointer is released.
- History keeps up to 32 snapshots and at most 128 MiB of serialized project state. A single snapshot is limited to 64 MiB. If a project exceeds that single-snapshot limit, the edit still works, but it is not recorded in history and the status line reports that undo history is unavailable.
- Undo and redo restore the project model and rebuild the audio playback plan. The MIDI note selection and active piano-roll drag are cleared.
- Returning to the last saved project snapshot clears the project's modified state.
- History is session state. It is not written into RWP files and is cleared when a project is opened or created.
- A recorded WAV is an external file. Undoing its clip removes the clip reference from the project but leaves the WAV on disk.

## OpenTrack- decisions and remaining work

The history stores serialized project snapshots to keep the first implementation bounded and easy to validate. It does not snapshot workspace layout, transport position, selected audio devices, decoded WAV cache, or external device/file side effects. Device reconnection, failed file operations, and crash recovery need separate transactional handling. The history behavior has unit coverage for project undo, redo, and redo invalidation; gesture behavior still needs manual UI acceptance.
