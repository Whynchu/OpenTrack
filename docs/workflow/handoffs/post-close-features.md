# Post-close feature batch (2026-11-02)

This batch followed the revision-5 close-out at the user's direction ("keep filling the missing features"). It is not a new numbered revision; each slice has a packet under `docs/workflow/tasks/` and each was committed and pushed separately.

## Delivered

| Work | Packet | Result |
| --- | --- | --- |
| Audio xrun recovery | `audio-xrun-recovery.md` | Recoverable ALSA underruns no longer destroy the engine; they are counted and shown as "N output xruns" |
| Buffer size reporting | `audio-xrun-recovery.md` | The negotiated period is read back (no more "0 frames"), the preset list comes from the device range, and the control is adjustable even when a device declares no range |
| Event color palettes | `event-color-palettes.md` | Six complete 16-color sets with per-slot editing; applying a palette assigns each track a slot by id as one undo step; Default Color is the standard grey; flavours resaturated |
| Remove Empty Tracks | `remove-empty-tracks.md` | Media tracks with no events, parts or automation are removed as one undo step; buses, folders, armed tracks and automation survive |
| Menu wiring batch | `menu-wiring-batch.md` | Edit > History, Audio > Spectrum Analyzer, Media > Open Pool Window, Media > Import Audio CD, Devices > MIDI Device Manager, File > Save as Template |
| Window list | `window-list-and-recent-projects.md` | Window > Windows opens/closes/focuses the seven app windows |
| Recent Projects | `window-list-and-recent-projects.md` | Newest ten opened/saved projects persisted and reopened from File |
| File > Revert | `revert-and-record-locator.md` | Reloads the saved project behind a discard confirmation |
| Start Record at Left Locator | `revert-and-record-locator.md` | Seeks to the locator and runs the ordinary record toggle |
| Back up Project | (module tests in `project_backup.rs`) | Bounded whole-folder copy with unique names and no partial backup |
| Use Pre-roll / Use Post-roll | `transport-pre-post-roll.md` | Persisted checkboxes; plain Play starts before the left locator and stops after the right one |
| Help > OpenTrack on the Web | commit `9fdb926` | Opens the project page through an OpenUrl command |
| Plug-in browser | `plugin-browser.md` | Audio > Plug-ins families open a filtered browser over the scanned VST3 catalog and insert into the selected track |
| File > Cleanup | `project-cleanup.md` | Unreferenced media moves to a dated trash folder inside the project, preserving relative paths |
| VST Connections | `vst-connections.md` | Read-only window: device buses and every group/FX return with width, device, output and send counts |
| Edit > Macros | `macros.md` | Named command macros persisted in settings, run through the ordinary dispatch path |
| Track library | `track-library.md` | File > New/Open/Save Library: application-wide track templates with kind, inserts and portable routing |
| Browse Sounds | `sound-browser.md` | One persisted folder of WAV files with a format preview and import through the shared WAV path |
| Control Room | `control-room.md` | Persisted listen source and talkback preferences with a routing readout; no monitor-bus DSP claimed |

Measured: the disabled project-open rows fell from 68 at the close-out to **40**, with 277 leaves (50 available, 164 state-dependent, 55 unavailable, 8 informational). App 620 passed / 4 failed / 9 ignored (633 tests), core 167, audio 172 / 3 ignored.

## Remaining rows and why they are still unavailable

- **Score editor and Scores menu (24 rows):** no engraving engine exists. This needs a scoped design before any row can be honest.
- **Video (Video Window, Import Video File, Replace Audio in Video, Extract Audio from Video, Generate Thumbnail Cache, Add Track > Video):** blocked on an approved decoder; the comparison and recommendation are in `R5-28-video-decision.md`.
- **MMC Master, Project Synchronization Setup, Use External Sync:** blocked on a MIDI output path and a clock source (`R5-26`, `R5-27`).
- **File > Page Setup / Print:** score printing depends on the score engine.
- **Media > Reconstruct:** the row has no defined product meaning yet; the waveform and analysis data are recomputed from source, so nothing is cached to reconstruct.
- **Help > Registration:** the project has no registration system.
- **File > Save / Save As / Edit > Undo / Redo and the "No project is open" rows:** these are the empty-project shell and are correctly disabled there.

## Verification limits

No desktop, listening, hardware, VST3-compatibility or Windows/ASIO verification is claimed in this batch. The four `transport_locate_tests` roll failures remain the documented environment baseline (no audio output device).
