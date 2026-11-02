# R5-18 — Audio CD import through cue sheets

- Status: accepted
- Scope version and requirement IDs: revision 5, step 18
- Desired observable outcome: File > Import > Audio CD imports a CUE sheet's tracks as one event per track, with bounded parsing, a clear report of unsupported commands and no compatibility claim beyond CUE + WAV.
- Reference evidence (IDs/paths): `crates/opentrack-app/src/app/cue_import.rs` (new); `opentrack_audio::audio_file_metadata`.
- Dependencies and decisions needed: none. Red-book image support and CD authoring are explicitly out of scope.
- Code scope and architecture boundaries: new focused module; one menu row becomes enabled; no new dependency.
- Acceptance checks: parser test for track starts at 75 frames/second, INDEX 00 ignored, malformed sheets refused, non-AUDIO tracks reported; app test for a real WAV + cue producing three events with correct offsets, names, ordering and one history step, plus a missing-file refusal that changes nothing.
- Affected regressions: the File > Import > Audio CD row moves from unavailable to state-dependent.
- Essential gates: bounded input, no partial import, undo, media registered.
- Review owner and milestone stop: self-review; desktop review of the file dialog pending.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: `CueSheet::parse` handles FILE/TRACK/INDEX/TITLE with a 64 KiB and 99-track bound and reports other commands; the import resolves the audio file next to the cue sheet, computes each track's span to the next start (or the file end), places one event per track sequentially with the source offset set, and commits one "Import Audio CD" step.
- Commands/checks and actual results: `cargo test -p opentrack cue_` → 2 passed; app suite 566 passed / 4 known device failures / 9 ignored; inventory 74 unavailable rows.
- Runtime environment: Kubuntu Linux.
- Review findings: the first parser let a track inherit the previous TITLE, which the import test caught; a track now starts untitled and only a following TITLE names it.
- Failures/unknowns/pending acceptance: desktop review of the dialog and an audible check of an imported image.
- Next ready task: R5-19.
