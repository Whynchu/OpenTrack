# T075 - Standard MIDI File exchange

- Status: core parser/writer and app import/export integration implemented; authored format-0/1 fixtures and independent export decoding pass; DAW-file/runtime acceptance pending.
- Roadmap: step 11.
- User outcome: move note and controller performances between OpenTrack and other MIDI applications without altering the project tempo or existing tracks.

## First milestone contract

- Import Standard MIDI File format 0 and 1 with PPQ timing. Reject format 2 and SMPTE time division with an actionable explanation.
- Import into new MIDI tracks at beat zero. Each source track containing supported note or control-change events becomes one new MIDI track and one MIDI part. Preserve track name, channel, pitch, velocity, beat positions, note lengths, controller number and value. Match repeated same-key notes in FIFO order per channel and pitch; velocity-zero note-on is note-off. Unmatched note-ons end at their track's last event tick. The project model permits one CC value per channel/controller/beat; colliding points use the last file value and are counted as ignored duplicates.
- Keep the project's tempo, meter, existing tracks and events. Convert tick positions to quarter-note beats using file PPQ. Return counts for imported notes/controllers and ignored event classes. Do not silently claim to preserve tempo maps, program changes, pitch bend, pressure, SysEx, markers, automation or instrument patches; report that these are unsupported by this milestone.
- Parse into a cloned project and validate before returning it. A malformed/unsupported file or exhausted project identity space must not partially mutate the live project.
- Export as format 1 at 480 PPQ with one conductor track containing the project's single tempo and time signature, then one track for each MIDI/Instrument track with parts. Preserve notes and control-change events/channels on the project beat timeline. Project loop locators, automation, patches, sends, markers and non-CC events are outside this first export scope.
- Write export output through the existing atomic project-file writer. Import runs off the UI thread and only applies if the source project is unchanged; the result is one undoable project operation. Export captures a project snapshot and never mutates project state.

## Source findings

- File > Import > MIDI File and MIDI File export were disabled in `app.rs`.
- `opentrack-core::Project` already stores arranged MIDI parts, beat-relative notes, channels and CC events, with identity allocation and validation helpers.
- The project has one tempo and one time signature, so importing tempo maps or broader MIDI event state would require a project-schema decision beyond this first milestone.
- No Standard MIDI File parser or writer dependency is present in the workspace lockfile.

## Verification

- Use authored format-0 and format-1 fixtures, including running status, tempo/meter meta events, notes across channels, overlapping same-key notes, velocity-zero note-off, CC events, SysEx and unsupported format/division cases. Compare import/export behavior against files produced by an independent DAW before declaring interchange accepted.
- Verify import failure leaves project data and identity counters byte-for-byte equivalent; successful import validates and can be undone.
- Verify exported files parse independently and retain beat positions, channels, durations, CC values, tempo and meter. Round-trip note/controller fixtures through export/import and document the intentionally lost fields.

## Progress

- Added a bounded, dependency-free format-0/1 PPQ parser and format-1 480-PPQ writer in `opentrack-core`. Import preserves source track names, MIDI channels, notes and CC events, pairs repeated same-key notes FIFO, accepts velocity-zero note-off, and leaves project tempo and meter unchanged. It reports ignored tempo/meta/SysEx/channel events and unmatched notes.
- Added File > Import > MIDI File and File > Export > MIDI File actions in a dedicated app module. Import parses a project clone on a worker, applies only if the project data is unchanged, validates the result and records the entire import as one undo operation. Export snapshots the project and uses the existing atomic file writer.
- Eight focused MIDI tests cover format-0 running status and CC, format-1 multi-track imports, event pairing, unsupported-event reporting, invalid-file atomicity, SMPTE rejection, round-trip semantics, and an independent decoder of the exported event stream. The complete core suite passes (31/31). The Windows x64 local-ASIO release build also succeeds.
- Still required: compare with independent DAW-produced MIDI files, verify undo/redo and menu/file-dialog behavior in the running app, and document actual results from Kubuntu and Windows use. The Windows build hash is `51B23CE5495A7496E599DFEDAF94F604149638FF80FB67ECBA4AC348D8A3F22E`.
