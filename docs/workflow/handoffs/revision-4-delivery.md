# Revision 4 delivery handoff (2026-11-02)

Revision 4 (`roadmap-32-steps-v4.md`) was executed serially from step 1 through step 32 by DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). Publication was authorized; each verified slice was committed and pushed to `origin/main`. No independent reviewer, desktop runtime acceptance, listening or hardware validation is claimed.

## Delivered (source + headless)

1. **Menu/command inventory:** reproducible generated inventory, corrected stale accessibility claims. 147/152 bound IDs dispatched.
2. **Media pipeline review:** junction/symlink containment for pool destinations and locked-event refusal for destructive bakes.
3. **Command boundary:** shared selection span for play-from/until, locate-selection and locators-to-selection.
4–6. **Pre/post-roll:** persisted 0–64 beat lengths, four Transport commands, cancellation and reasons; per-frame stop overshoot is a known timing limit.
7. **MIDI Reset:** releases on-screen keys and sends CC120/121/123 on 16 channels; no external MIDI output exists for hardware stuck-note testing.
8–10. **Drum map/setup/editor:** identity-default presentation map, draft Apply dialog, map-ordered drum editor sharing note IDs/history.
11. **Logical presets:** pitch/velocity selection and atomic selected-note channel change.
12. **CC Automation Setup:** one persisted CC lane (0–127), readable labels, view-only switching, undoable CC content; no OPR bump for the view field.
13–16. **FX returns:** distinct kind (OPR 35), typed `FX:` sends with cycle/dangling refusal, Add FX Channel with optional VST3 insert, deletion clears targeting sends, FX automation explicitly unsupported. Live/offline generated-signal test matches.
17–20. **Folders:** OPR 36 hierarchy (8 levels), Add Folder/Move/Remove/duplicate/reorder, indentation + `▤F` folding persisted, Mixer/Inspector omit folder channels, navigation skips hidden rows without deleting selection.
21. **Marker lane:** optional workspace-persisted lane, snapped add/drag/delete/seek, one history step per gesture, Escape rollback; defaults off to preserve layout.
22. **Regions:** list editor with select/locate/rename/resize/add/delete, Apply-time snap and one history step; selection separate from audio events.
23. **Secondary ruler:** optional second row using existing bars/seconds/timecode/samples/60 fps conversion; old layout unchanged when off.
24–28. **Arranger:** OPR 37 bounded section/sequence contract, editor with locator/region creation and sequence editing, offline sequence transformer (audio/MIDI/CC/volume automation with boundary clipping; fades refused), playback-only preview with Cycle/recording refusal and source/sequence readout, Add Arranger opens the single editor (no fake track).
29–30. **Track Archive v1:** OpenTrack-only directory package with manifest/version, hierarchy, media dedupe, external route/plug-in report, staging/rename publication; import validates format/path containment, remaps IDs/routes, disconnects external buses and applies one history step. Import is synchronous.
31. **Acceptance:** core 164 passed; audio 171 passed/3 ignored; app 544 passed/9 ignored; fmt/diff clean; x64 release built with local ASIO, SHA-256 `7206DC24A44620E59D57828E6C6CBC7EEB0C0B849DAA5545AA13B434AF23A826`. A later app suite after the Add Arranger/archive work reported 544 passed; the final step-32 pass reruns and records the definitive counts.

## Runtime/hardware pending

- Desktop review of every new dialog/lane/menu interaction and Project/Mixer/Inspector layouts.
- Audible FX return/insert, arranger preview/stop/seek/MIDI-hang, metronome/roll timing and export comparisons.
- VST3 compatibility with real third-party effects/instruments, including missing-module load behavior.
- ASIO hardware stability and low-buffer performance; the build retains local ASIO but was not launched.

## Remaining product gaps and dependencies

| Area | Status | Needs |
| --- | --- | --- |
| Score engraving | not implemented | product/engraving design, layout fixtures |
| Tempo/signature edit + import/export | deferred | one beat/sample conversion contract across recording, automation, DSP, MIDI and render |
| Transpose track menu | disabled | existing MIDI transpose command can back a bounded UI; design decision on scope |
| Video | not implemented | video engine/decoder and sync design |
| External sync / MMC | not implemented | protocol/clock source and device fixtures |
| Retrospective record | not implemented | bounded buffer, timestamp/privacy policy, user workflow |
| Control Room / MMC surface | not implemented | monitoring matrix and device mapping design |
| Offline plug-in stack | not implemented | ordered persisted processing contract, cancellation, stale-result policy |
| Legacy interchange (CPR/OMF) | inspection-only / none | real named-version fixtures and format-specific adapters |
| Platform | Windows x64 primary; Linux verified earlier | ongoing packaging and VST3 editor support review |

## Measured size

`app.rs` grew from 31,512 lines at `12a01e0` to 34,262 lines (+2,750). New focused app modules total about 2,742 lines (`drum_editor`, `drum_map_setup`, `logical_presets`, `cc_automation`, `region_editor`, `arranger_editor`, `track_archive`), plus `opentrack-core::arranger` and `opentrack-audio::arranger`. Final inventory: 275 project-open leaves, 40 available, 145 state-dependent, 82 unavailable, 8 informational; 18 shell leaves; 147/152 bound command IDs dispatched.

## Next queue recommendation

Do not append more slices to revision 4. Resume with runtime acceptance of this revision, then choose one of: (a) tempo/signature contract, (b) offline plug-in processing contract, or (c) Track Archive import backgrounding/progress. Each needs a written decision before dependent work; real CPR/OMF files remain a fixture blocker.
