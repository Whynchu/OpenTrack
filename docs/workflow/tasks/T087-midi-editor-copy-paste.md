# T087 - MIDI editor note copy and paste

- Status: ready
- Scope version and requirement IDs: 1
- Desired observable outcome: Copy/Cut/Paste works for selected MIDI notes in the MIDI editor without requiring selection of arrange audio events.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown): User reported (2026-11-03) that MIDI editor copy/paste did not work. Source inspection found `edit.copy`, `edit.cut`, and `edit.paste` dispatch only to audio-event methods; no MIDI note clipboard exists. Current MIDI editor selection is `selected_midi_note_ids` and notes are owned by the selected `MidiPart`.
- Dependencies and decisions needed: Use a transient app clipboard, preserving copied note pitches, durations, velocities, channels, and relative beat spacing. Paste into the active part at the project transport cursor translated to part-relative time, snapping note starts using the MIDI editor's position grid. Assign fresh `next_midi_note_id` values. Cut removes only the copied selection through one undoable project edit. If no MIDI selection is active, retain the existing audio-event clipboard behavior.
- Code scope and architecture boundaries: App command dispatch, Edit menu enablement, MIDI note clipboard state and validated project-history mutation. Do not alter MIDI file serialization or audio-event clipboard semantics. Clipboard is transient and does not dirty project until pasted/cut.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. Select one or more notes and Ctrl+C; clipboard captures the group with relative timing/pitch.
  2. Move transport and Ctrl+V; notes appear at the snapped cursor-relative position in the active part with fresh IDs and unchanged relative offsets/velocities/channels/durations.
  3. Ctrl+X removes selected notes in one undo step and keeps them available to paste; Undo restores notes and Redo removes them.
  4. Empty selection or non-MIDI focus keeps current audio-event Copy/Cut/Paste behavior.
  5. Paste refuses or clips safely at part bounds without partial project mutation; save/reopen and MIDI export retain pasted notes.
- Affected regressions: MIDI note selection, project history, shortcut focus ownership, Edit menu enablement, audio-event clipboard dispatch, save/reopen and MIDI export.
- Essential gates (audible result, persistence, undo/dirty state, recovery as applicable): No duplicate note IDs or partial paste on invalid bounds. Copy is transient; Cut/Paste content edits are undoable and dirty the project. Headless command and project-history checks plus running-app key interaction.
- Review owner and milestone stop: implementation review then user review in the running MIDI editor.

## Handoff

- Actual implementer/reviewer model:
- Source identity (commit plus dirty diff/artifact identity) and build hash:
- Changes and reasons:
- Commands/checks and actual results:
- Runtime environment and evidence paths:
- Ledger IDs updated; report command/result:
- Review findings, rework, elapsed effort if available:
- Failures/unknowns/pending acceptance:
- Next ready task:
