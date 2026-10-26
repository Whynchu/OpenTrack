# T032 — Copy and paste audio events in the Project timeline

- Status: implementation/build complete; live interaction review pending.
- Implementer: GPT-6.1 Sol; no subagents or independent review.
- User request: copy and paste audio events from the Project timeline with keyboard shortcuts.
- Shortcut mapping: the checked-in reference keymap defines Ctrl+C as Copy and Ctrl+V as Paste. Ctrl+P remains Project > Pool (Media Pool).
- Copy stores selected audio event data and source paths in a transient in-app clipboard; it does not copy WAV bytes or dirty the project. Paste places the copied group at the shared transport cursor, retains relative beat spacing and track-row spacing, and preserves source offsets, duration, fades and event gain. It generates new event IDs, retains source references in Media Pool, and requires matching audio channel widths for destination lanes. After paste, inserted events become selected. Undo/Redo use project history. Clipboard clears on successful project open or project reset.
- Acceptance: select one event and Ctrl+C/Ctrl+V at another cursor position; repeat with a multi-event selection spanning tracks; verify copied content and placement, source identity, waveforms/playback, new IDs, channel-width rejection, copied-event selection, undo/redo, and no file duplication. Check Edit menu enablement. Confirm Ctrl+P still opens Pool and typing in text fields does not trigger timeline shortcuts.
- No automated tests were added or run for this request. Build compilation is not runtime acceptance.
- Existing project changes from T029–T031 remain uncommitted and are preserved in the same working tree. OpenTrack was closed before the build and remains closed.

## Build and source identity

- x64 release build with local ASIO: `build.ps1 -Architecture x64 -LocalAsio`; `build-info.json` confirms `local_asio=true`.
- OpenTrack.exe SHA-256: `4EC7FEF814FE58763AB2D40F2DD00EFE3E0DAE7684C1551BD4E133BADED6F771`.
- `audio_events.rs` SHA-256: `7122D7DFACBF390C2DF6DDF7990EF1533D360E49C47641815846410CF140541C`.
- `app.rs` SHA-256: `1E5564B84110C84F60E3C96760A020FDF03128336A88B7053871ABEF7EB2F7FB`.
- `project.rs` SHA-256: `DC0C81B3541AAB1002F7416D1B8E95F01766BEFDFD6DF70ADCAC23A8A0DC27DE`.
