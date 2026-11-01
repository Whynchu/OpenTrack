# ComboBox selector audit

Scope: all `egui::ComboBox::from_id_salt` declarations in `crates/opentrack-app/src` (30 instances across 5 source files). This is a source trace, not a runtime usability or persistence test. “App storage” means eframe app preferences; “project” means serialized project data. Transient selections are explicitly marked. Line references refer to the source snapshot audited.

## Audio and recording selectors

| # | Control and source | Values / data source | Consumer and persistence | Status / unknowns |
|---|---|---|---|---|
| 1 | Audio Driver, `audio_configuration.rs:163` | System/default (`None`) plus enumerated ASIO output devices | Applies via `apply_audio_output_settings`; selected output ID stored as `audio-output-device` | Active; app-storage persistence traced. Exact backend fallback behavior is in audio engine. |
| 2 | WASAPI Mode, `audio_configuration.rs:189` | Shared Low Latency, Shared Compatible, Exclusive | Applies output settings; app storage key `audio-device-mode-v1` stores preference value | Active; app-storage persistence traced. Only shown when active backend is not ASIO. |
| 3 | Capture Device, `audio_configuration.rs:370` | System default plus enumerated non-ASIO input devices | Updates `selected_input_device`, probes format/capture state; saved as `audio-input-device` | Active; disabled during recording. |
| 4 | Playback Device, `audio_configuration.rs:407` | System default plus enumerated output devices | Applies output settings; saved as `audio-output-device` | Active; app-storage persistence traced. |
| 5 | Driver Sample Rate, `audio_configuration.rs:485` | Driver default plus `audio_output_sample_rates` | Applies selected output rate; saved as `audio-output-sample-rate` | Active subject to backend/mode enablement and recording lock. |
| 6 | Driver Buffer Size, `audio_configuration.rs:499` | Driver default plus currently computed `choices` | Applies output buffer frames; saved as `audio-output-buffer-frames` | Active when choices exist; available options depend on device and mode. |
| 7 | MIDI Port Setup Input, `audio_configuration.rs:541` | No MIDI input plus enumerated MIDI input devices | `select_midi_input_device`; saved as `midi-input-device` | Active; app storage. |
| 8 | Project Recording Sample Rate, `audio_configuration.rs:585` | 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000 Hz | Updates project sample rate through pending project-rate workflow; project serialization | Active; disabled while recording or rate change pending. Project media conformity workflow applies. |
| 9 | Project Recording Format, `audio_configuration.rs:602` | PCM16, PCM24, Float32 | Updates `project.recording_format`; used by new recordings; project serialization | Active; disabled while recording or rate change pending. |

## Analyzer selectors

Analyzer controls are `Analyzer` view state in `analyzer_view.rs`; no eframe save/load path for this view state was found. Treat selections as session/transient unless another owner is added.

| # | Control and source | Values / data source | Consumer | Status / unknowns |
|---|---|---|---|---|
| 10 | Analysis Stage, `analyzer_view.rs:483` | Pre or Post inserts/EQ (shortened labels in Tonal view) | Chooses spectrum tap for display/analysis | Active; transient. Tonal and spectrum modes use separate stage fields. |
| 11 | Analysis Reference, `analyzer_view.rs:525` | None plus available channel choices except owner | Selects channel comparison | Active in Spectrum view; transient. |
| 12 | Tonal Average Duration, `analyzer_view.rs:608` | 10, 30, 60, 120 seconds | Sets continuous tonal average time constant | Active in Tonal view; transient; defaults to 30 s. |
| 13 | Tonal Reference, `analyzer_view.rs:624` | `TonalReference::ALL` (whole-spectrum mean, 200 Hz–2 kHz average, 500 Hz band per tooltip) | Sets display reference for tonal level | Active in Tonal view; transient. |
| 14 | Spectrum Range, `analyzer_view.rs:651` | 60, 90, 120 dB | Sets plot vertical range | Active in Spectrum view; transient; defaults to 90 dB. |
| 15 | Spectrum Tilt, `analyzer_view.rs:663` | 0, 3, 6 dB/octave | Display weighting around 1 kHz; does not alter audio/capture | Active in Spectrum view; transient. |
| 16 | Spectrum Resolution, `analyzer_view.rs:679` | Fast 2K, Balanced 4K, Fine 8K | Selects FFT resolution, affects update/low-frequency detail | Active in Spectrum view; transient; default Balanced. |

## Key command and workspace selectors

| # | Control and source | Values / data source | Consumer and persistence | Status / unknowns |
|---|---|---|---|---|
| 17 | Key Command Category, `app.rs:6942` | All categories plus categories from loaded keymap rows | Filters visible commands; copied back to `key_command_category` when window closes | Active filter, transient; no storage write found. |
| 18 | Workspace Shortcut ID, `app.rs:7290` | None plus available explicit IDs | Sets `workspace.shortcut_id`; workspace data saved in project workspace state | Active when explicit IDs enabled; project persistence traced. Options exclude conflicting IDs per UI construction. |

## MIDI and track creation selectors

| # | Control and source | Values / data source | Consumer and persistence | Status / unknowns |
|---|---|---|---|---|
| 19 | MIDI Input Device, `app.rs:8132` | No input plus enumerated MIDI devices (duplicate names disambiguated with IDs) | Selects MIDI input stream for MIDI thru/record-related input; app key `midi-input-device` | Active duplicate of MIDI Port Setup Input; same state and storage. |
| 20 | Add Audio Track Configuration, `app.rs:10287` | Mono (1 channel), Stereo (2 channels) | `dialog.audio_channels`, passed to track insertion | Active for add-audio-track dialog; dialog state only until insertion, then project track property persists. |
| 21 | Add Audio Speakers, `app.rs:10318` | No options (empty popup); disabled, fixed label `M` | No selection consumer | Disabled placeholder. Source comment says speaker choices are unverified. |
| 22 | Add Group Channel Configuration, `app.rs:10328` | Mono (1 channel), Stereo (2 channels) | `dialog.audio_channels`, passed to track insertion | Active for add-group dialog; dialog state only until insertion, then project track property persists. |
| 23 | Add Instrument Choice, `app.rs:10359` | Empty popup; fixed label `No VST Instrument` | No consumer | Disabled placeholder; no instrument can be chosen here. |
| 24 | MIDI Position Grid, `app.rs:15215` | 1/64, 1/32, 1/16, 1/8, 1/4, 1/2, whole note | Sets MIDI editor position quantize used by note placement/movement | Active; app storage key `midi-position-quantize`. Snap checkbox in this same toolbar is a separate local variable, so its actual persistence/interaction is not established by this ComboBox trace. |
| 25 | MIDI Length Grid, `app.rs:15223` | Same seven note values | Sets MIDI note length quantize used by inserted/resized notes | Active; app storage key `midi-length-quantize`. |
| 26 | MIDI Controller Lane, `app.rs:15494` | Modulation CC1, Volume CC7, Pan CC10, Expression CC11 | Selects controller events/curve and controls inserted points/defaults | Active; editor UI state defaults to CC11. No save/load key for selected controller was found, so selection appears transient. MIDI events themselves are project data. |

## Sample Editor and project timing selectors

| # | Control and source | Values / data source | Consumer and persistence | Status / unknowns |
|---|---|---|---|---|
| 27 | Sample Editor Timebase, `sample_editor.rs:413` | Bars + Beats or Seconds | Selects editor ruler presentation through `state.musical` | Active; no app save/load of this field found; appears transient. |
| 28 | Sample Editor Grid Type, `sample_editor.rs:430` | `GridType::ALL` | Determines snap step | Active; included in `SnapSettings`, app storage key `sample-editor-snap-v1`. |
| 29 | Sample Editor Quantize, `sample_editor.rs:438` | `QuantizeNote::ALL` × Straight/Triplet/Dotted | Determines snap step | Active; included in `SnapSettings`, app storage key `sample-editor-snap-v1`. |
| 30 | Time Signature Beat Unit, `transport_panel.rs:732` | 1, 2, 4, 8, 16, 32 | Mutates `project.beat_unit`, marks history commit and calls project timing update | Active; project data persistence. |

## Audit notes

- Search method: `rg -n "ComboBox::from_id_salt" crates/opentrack-app/src`; app source currently contains declarations in `audio_configuration.rs`, `analyzer_view.rs`, `app.rs`, `sample_editor.rs`, and `transport_panel.rs`.
- Counts: 9 audio/setup/recording, 7 analyzer, 2 keymap/workspace, 8 MIDI/track creation, and 4 Sample Editor/timing controls = 30. Counted by UI declaration, including the two disabled empty placeholders.
- Persistence descriptions are source-based. No runtime reopen test was performed. For transient controls this means no app preference serialization was located, not a claim that no external UI-state mechanism exists.
- Several controls bind to global app storage or project model, so multiple visually distinct controls (for example MIDI input setup and MIDI input preferences) are two UI entry points into the same value.
- The toolbar Snap checkbox in MIDI editor is initialized as a local `true` on render and is not connected to the snap behavior in the shown code; it is not itself a ComboBox, but is a notable adjacent unknown.
