# Current assignment

Updated: 2026-10-24. Actual implementer: GPT-6.1 Sol.

## Active stage

T017 full-height waveforms and Discard recording cleanup, implemented and awaiting user runtime review. Current accumulated source publication was explicitly authorized. T016 waveform appearance received positive user feedback with a full-height correction. The user requested recording undo remove the actual take files, then requested waveforms during capture and higher detail. The user closed OpenTrack before this build; a final process check found OpenTrack running again after the build. The agent did not launch or interact with it.

Undo now removes newly recorded WAV filenames together with their events; redo restores exact files from Metadata/Undo. Only successfully placed recorder-created takes are owned by the history entry. Imported/existing media is preserved. Multi-take operations preflight, do not replace another file, and retain the history step/recovery on failure. Saved-project Discard restores recordings referenced by the saved document, including after a redo branch was discarded. Automatic post-crash recovery is not implemented; path manifests remain available. Redo/saved-document recovery temporarily retains disk data.

Waveforms are signed per-channel envelopes in 64-frame blocks with connected filled drawing at pixel density, replacing the 256-peak sparse comb. Decoded files use individual samples at close zoom; streamed-file sample-level zoom remains a gap. The recording writer publishes growing waveform snapshots about every 33 ms, outside the audio callback, and Project draws a route-correct growing event from captured frames. Desktop cadence, visual/audio acceptance and long-take resource qualification remain pending. See [T016](tasks/T016-recording-undo-and-waveforms.md).

T015 bus controls and compact Device Setup remain in the build and await hardware/visual review. Sends DSP, automation, Listen/control-room processing, independent bus Solo, extra buses and arbitrary physical maps remain incomplete. Third-party VST hosting remains separate.

Waveform drawing now uses the full event height with a small border margin, retaining true signal amplitude. Discard removes recorder-created WAVs since the last save, even after history trimming, and preserves imported/saved media. New take filenames avoid undo/recovery reservations. See [T017](tasks/T017-waveform-height-and-discard.md).

## Build and evidence

Current x64 local-asio release SHA-256: 88482EEE7B248AF7F32A36D7CBD69CF7921484B101976324A0900BE23BD3666F. Artifact: `builds/x64/OpenTrack.exe`. 108 workspace tests pass (58 app, 39 audio, 11 core); standard WASAPI cargo check passes. All seven targeted recording-history tests pass, including unsaved Discard cleanup and reserved take names. Offline tests cover file undo/redo, import preservation, branch cleanup, saved/discard recovery, collision handling, live writer publication, signed stereo detail, sample-level decoded zoom, and clipped dense mesh drawing. These do not establish desktop/hardware acceptance.

No OpenTrack instance was launched. EXAMPLE.opr was not opened or modified. Keep one instance, close through the UI before building, and leave closed after review. The build script refuses to replace a running executable.

Previous crash-fix release was 5E3A608D; its navigation layout panic was reproduced and corrected. ASIO support remains optional (`build.ps1 -Architecture x64 -LocalAsio`); WASAPI remains standard. The read-only Mackie driver query remains valid hardware inventory, not a verified audio stream: first Analog 1/2 pair, preferred 128 frames, supported 44.1..192 kHz. Streaming, control panel, recovery and coexistence still need user/hardware qualification.

Audio metrics use **audio-workflow-v6**: retain v5's 25 requirements/weights and add recording cleanup on Discard. Baseline remains incomplete and implementation percentage suppressed. Runtime/reference acceptance is pending or stale; no whole-DAW score is inferred. Earlier transport and project-folder metrics stay separate.

## Next review

Record a short take on a disposable project and inspect waveform growth before Stop. Check mono/stereo separation, full-height amplitude drawing and zoomed detail. Close and Discard unsaved takes: their files should be removed; saved recordings/imports should remain. Stop, Undo (events and original WAV names disappear), Redo (both return); confirm unrelated imports remain. Check saved/reopened audio and prior T015 bus controls. Do not overwrite EXAMPLE for destructive recording checks. Stop for user feedback before expanding to another milestone.

Preserve accepted Mixer geometry, working Project/ruler/transport, startup without a project, managed project folders and unchanged-project close behavior. Accepted Mixer baseline: `ac758cc`; published development checkpoint: `cbd9835`. Existing uncommitted work is preserved.

Publication: current accumulated development checkpoint is authorized for origin/main. T017 release built; 108 tests pass. The app was not launched by the agent. No saved projects, audio takes or build artifacts are staged.
