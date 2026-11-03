# Revision 5 delivery handoff (2026-11-02)

Revision 5 (`roadmap-32-steps-v5.md`) was executed serially from step 1 through step 32 by DeepSeek V4.1 Flash (`opencode-go/deepseek-v4.1-flash`). Publication was authorized; each verified slice was committed and pushed to `origin/main`. No independent reviewer, desktop runtime acceptance, listening or hardware validation is claimed. Step 2 (the runtime checklist for the revision-4 manual-review fixes) stays with the user and is **not** closed by this document.

## Delivered (source + automated)

1. **Baseline reconciliation:** inventory regenerated deterministically, 147/152 dispatch at the start, four roll tests proven to fail identically on the pristine revision-4 checkpoint (no audio device), README/accessibility/inventory reconciled.
2. **Runtime checklist (user):** open — see "Pending" below.
3. **Export contract review:** mixdown and stems share one locator-only range function, bounce uses the selection by design, no row claims an arranger export, cancelled exports publish nothing. Two decisions remain open: stems include group/FX channels; the arranger has no export entry.
4. **Tempo/signature contract:** `Project::set_tempo`/`set_time_signature` with shared bounds (1–1000 BPM, 1–32 beats per bar, beat units 1/2/4/8/16/32); no OPR bump needed. Accents stay an application preference, recorded as a decision.
5. **Tempo Track editor:** tempo, beats per bar and beat unit, one "Tempo and Signature" step, no-op writes nothing, recording disables the controls.
6. **Conversion audit:** render-length test at 60 BPM/44.1 kHz, 120/48 kHz and 174/96 kHz; no hard-coded tempo or rate in production conversion paths.
7. **Tempo exchange:** versioned `opentrack-tempo` v1 JSON, Foreign/unknown/invalid data refused; import applies through the editor path.
8. **Ctrl+T** dispatches; the curated inventory guard pins the tempo row as reachable.
9. **Transpose:** verified the existing Transpose/Velocity dialog; `Add Track > Transpose` stays unavailable because it names a transpose track.
10–11. **Retrospective capture:** 60 s / 8,192-event in-memory buffer with arrival-age correction, per-key/channel pairing, unclosed keys closed; `Retrospective Record` materialises a part on the armed destination as one step; Shift+Numpad\* dispatches.
12. **Virtual keyboard close-out:** MIDI Reset and close both release held keys, covered by a test; audible result pending.
13. **Offline chain model:** `OfflineProcess` + `AudioClip::offline_chain`, OPR 38, eight entries, validated, locked events refuse edits.
14. **Staged chain render:** each processor reads the previous stage, failures delete staged files, one "Direct Offline Processing" step.
15. **Direct Offline Processing window:** draft editing, Save = one "Edit Offline Chain" step, Apply bakes separately.
16. **Add/Remove Plug-in:** built-in processor submenu and last-processor removal through the same validated path.
17. **Offline acceptance:** locked refusal, per-event isolation, failure cleanup, persistence; the synchronous render limitation was recorded here.
18. **Audio CD import:** bounded CUE parser (64 KiB, 99 tracks), one event per track, INDEX 00 ignored, unsupported commands reported, one history step.
19. **Background Track Archive import:** worker with progress, cancel before any copy, stale-result rejection by project fingerprint, menu gated, one step.
20. **Archive round-trip acceptance:** folders, FX return and typed send preserved with remapped ids, shared media stored once, source media untouched.
21. **Interchange decision:** other project formats wait for full feature support (user, 2026-11-02); no adapter or alias.
22. **Add Track mapping:** Marker/Ruler toggle existing lanes, Signature/Tempo open the tempo editor, Transpose opens its dialog, Video and Browse Sounds stay unavailable.
23. **Regions editor polish:** multi-select, keyboard cursor/navigation, confirmed delete, region selection independent of audio events.
24. **Arranger preview review:** refusals (empty, Cycle, unprepared plan), sequence-to-source mapping across repeats, clean stop and rewind; audible result pending.
25. **Secondary ruler review:** same clock at 44.1/96 kHz for seconds, 30/60 fps timecode with offset, bars; samples scale with the rate; format persists off by default.
26–28. **Decision records:** MMC needs a MIDI output path and a loopback device; external sync needs a clock source and lock/drift handling; video needs an approved decoder (FFmpeg FFI recommended if it becomes a requirement). No dependencies added.
29. **Performance:** the offline chain render moved to a worker with stage progress and cancellation, removing the frame-blocking hotspot recorded in step 17. No frame-time number claimed.
30. **Accessibility:** dialogs now own the keyboard (`modal_dialog_open()`/`dialog_owns_input()` guard) — previously Delete with the Regions editor open deleted the event behind it; report regenerated to 149/152 with Video, Open Score Editor and Use External Sync as feature-blocked.
31. **Acceptance:** app 579 passed / 4 known device failures / 9 ignored, core 166, audio 171 / 3 ignored, clean fmt and diff checks, release build SHA-256 `4c0034ac2992150c65483d39cb61b2a5b1e6ebd8727bbabd10b2bdd444b13a40`, OPR 38 with migrations to v29–v38.
32. **Consolidation:** inventory 275 leaves (41 available, 157 state-dependent, 69 unavailable, 8 informational); README, native-format doc, current.md and this handoff updated; no post-close slices appended.

## Measured size

- `app.rs` grew from 34,262 lines at the revision-4 handoff to 34,789 (+527).
- Focused app modules under `src/app/` now total 5,792 lines across 12 files: `arranger_editor`, `cc_automation`, `cue_import`, `drum_editor`, `drum_map_setup`, `logical_presets`, `offline_processing`, `region_editor`, `retrospective`, `tempo_editor`, `tempo_file`, `track_archive`.

## Pending (not delivered)

- **R5-02 user runtime review** of the revision-4 manual-review fixes: drag Mixer 2 over the arrange, open both extra mixers, read the Spectrum Analyzer status line, open Audio Mixdown.
- Audible review of FX returns, arranger preview, the virtual keyboard, retrospective capture with a real controller, and a baked offline chain versus the direct commands.
- Windows x64 local-ASIO build and runtime; VST3 compatibility with third-party plug-ins; Linux VST3 editor embedding.
- The four `transport_locate_tests` roll cases need an audio device; they fail identically on the pristine revision-4 checkpoint in this container.

## Open product decisions

- Should stems include group and FX channels (their sum currently exceeds the mixdown)?
- Does the arranger need an export path, or stay preview-only?

## Next queue recommendation

Do not append post-close slices to revision 5. Resume with the user's runtime review and the open decisions, then pick one: a MIDI output path (unblocks MMC and, with a clock source, external sync), a video decoder (only after the licence/packaging decision in step 28), or score engraving (needs product design). CPR/OMF remain deliberately deferred until the feature set is complete.
