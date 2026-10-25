# OpenTrack- implementation roadmap

This roadmap tracks the path from the current prototype to a dependable OpenTrack DAW. The product target is broad Cubase 5 workflow coverage in original Rust code. Compatibility claims require evidence from the reference application; matching control names alone is not enough.

## Working method

Delivery now follows [the fidelity plan and user review landmarks](fidelity-plan.md). The first acceptance sequence and current mismatch audit are in [basic-track-workflow.md](basic-track-workflow.md). Feature checkboxes below indicate implementation history, not verified Cubase parity.

For each feature, keep three records distinct:

- **Reference facts** from the Cubase 5 manual, REA observations, or controlled reference-app sessions.
- **OpenTrack- decisions** made to build a useful and safe implementation where reference details are not yet known.
- **Parity evidence** from controlled, repeatable comparisons. Do not call a rewrite accurate until its parameters, state, routing, and output have been compared.

Each milestone should include a user-visible workflow, persisted project state where applicable, bounded real-time behavior, focused automated checks, and a manual acceptance procedure. Audio callbacks must not block on disk or UI work, allocate in steady state, or take contended locks. Recordings must report overrun/dropout conditions rather than silently implying a clean take.

## Milestones

### 0. Evidence and baseline

- [x] Inventory the application, the bounded REA scan, and the available Cubase 5 Operation Manual.
- [x] Record which plug-in files were established as x86 and which module architectures remain unknown.
- [x] Add targeted, page-numbered notes for recording, channel EQ, and input routing in [cubase5-audio-reference.md](cubase5-audio-reference.md).
- [ ] Capture repeatable reference-app observations for UI details and audio behavior before making parity claims.

### 1. Audio capture and recorded takes

- [x] Open the default input device and expose its name, sample rate, and channel count.
- [x] Enumerate input devices, allow a device choice in the Audio menu, and remember the selected device ID.
- [x] Capture mono or stereo input on the audio callback into a bounded, preallocated queue; perform WAV writing on a worker thread.
- [x] Record the selected input through one shared stream into synchronized WAV takes on all armed audio tracks; place each take at the record-start position.
- [x] Add Windows MIDI input capture for multiple armed MIDI/Instrument tracks, with MIDI Thru to the built-in starter oscillator and support for recording alongside multiple armed audio tracks.
- [x] Finalize WAV metadata on stop, add takes through the in-memory or streamed source path, generate waveforms, and persist their external paths through the native format.
- [x] Surface input, file, stream, and queue-overrun errors, including a report of dropped frames.
- [x] Add optional lock-free input monitoring while recording, resample it to the output rate, and report monitor-ring overflow.
- [x] Show live recording-input and post-fader track peaks in the Mixer using lock-free callback telemetry.
- [x] Exercise the production queue-to-WAV worker with synthetic input, then decode the finalized WAV and render its first frame through a playback plan without opening an audio device.
- [x] Verify callback meter publication, dBFS scale mapping, and input peak reporting when the recording queue drops frames.
- [ ] Manually import and play a WAV above the in-memory threshold, seek around, and confirm waveform completion and clean playback on the target PC.
- [ ] Verify manually with a short input recording, stop/reopen/playback, and compare the file's frame count, sample rate, channels, and waveform.
- [ ] On the target PC, confirm live input and playback meter response, the dBFS scale, and clipping indication with known signal levels.
- [ ] On the target PC, confirm input-monitor latency and dropout behavior at matching and differing input/output sample rates.

### 2. Channel EQ

- [x] Add four persisted EQ bands to each audio channel, including enable and bypass controls.
- [x] Implement stereo filters with coefficients and per-channel state prepared outside the real-time callback.
- [x] Provide Mixer controls for band type, frequency, gain, and Q.
- [x] Verify parameter bounds, neutral response, frequency response, stereo behavior, queue overflow handling, and backward-compatible track loading.
- [ ] Compare Cubase's actual defaults, filter choices, Q range, and response against the manual and reference application before claiming parity.

### 3. Insert effects and first-party plug-in rewrites

- [x] Add a persisted per-channel insert chain, enable/bypass, ordering, and parameter state.
- [x] Expose insert ordering controls in the Mixer so users can move processors within the chain.
- [x] Add the first effect processor and UI; run effects after clip summing and EQ and before the channel fader.
- [x] Add an original stereo-linked Compressor with persisted threshold, ratio, attack, release, and makeup controls; validate its parameter bounds and verify steady-state compression and stereo balance.
- [x] Connect the documented TestGenerator control list to the insert chain using the original Rust DSP prototype; keep signal parity marked unknown.
- [x] Keep processor construction and memory allocation off the audio callback; define the initial effect's time, feedback, wet-mix, and stereo limits.
- [ ] Record the selected plug-in's actual UI defaults, audio behavior, state, and repeatable reference test signals before extending the prototype toward parity.
- [ ] Measure null/error and perceptual differences against the reference application; track unsupported behavior explicitly.

### 4. Project and edit reliability

- [x] Add bounded in-memory project undo/redo for UI gestures and keyboard project edits.
- [x] Add a File-menu relink action for unavailable WAVs; decode the replacement before updating all clips that share the old path, then rebuild playback and retain the project edit in undo history.
- [ ] Add action-specific transactions and recovery semantics before expanding destructive media operations.
- [ ] Manually accept undo/redo across recording, mixer parameter edits, insert ordering, and MIDI note gestures.
- [x] Add a categorized Key Commands window with add/change/remove actions, conflict confirmation, default reset, persistent application settings, and editor-context shortcuts.
- [x] Connect global and project workspace presets to the organizer, persist global settings, copy presets into projects, create workspaces from presets, and auto-instantiate global presets idempotently when enabled.
- [x] Add separate Mixer, Channel Settings, MediaBay, and MIDI Editor windows; store project-specific open state and geometry in the active workspace.
- [x] Add arrange-track Mute, Solo, Solo Defeat, Record Enable, and Channel Settings controls; Ctrl-click Solo is exclusive and Alt-click toggles Solo Defeat through playback.
- [x] Add track rename, reorder, and confirmed removal through the Track menu; preserve selection and keep removal unavailable during recording.
- [x] Add independent per-track height resizing, Ctrl-drag resizing for all tracks, compact row controls, workspace persistence, and the three Cubase track-zoom key commands.
- [ ] Manually accept track creation, rename, reorder, removal, undo, and MIDI/audio content preservation on the target PC.
- [ ] Capture Cubase window roles, positions, sizes, and open/closed combinations in controlled reference sessions, then compare at matching DPI and display scale.
- [ ] Verify window restoration across workspaces, display scaling changes, and monitor arrangements; save and validate monitor identity before claiming multi-monitor support.
- [ ] Compare track heights, compact-row controls, zoom increments, and exclusive zoom against the installed Cubase 5 build; refine the current inferred exclusive-zoom behavior from direct observation.
- [ ] Implement channel routing from captured Cubase reference behavior.
- [ ] Add media collection/relocation, autosave, crash recovery, and format migrations for future project fields.
- [x] Stream large audio sources through a bounded per-clip cache and generate their waveforms on a background scan without loading the entire source into memory.
- [ ] Manually accept long-file playback, seeking, and waveform generation under the target device's real-time load.
- [x] Add persisted beat-based left/right cycle locators and loop playback in the audio callback and fallback transport.
- [ ] Add fades, trim/edge editing, snap/grid, timebase choices, markers, and non-destructive clip processing.
- [ ] Verify old OpenTrack- project versions continue to load as fields are added.
- [ ] Accept MIDI input recording and MIDI Thru with a physical device; verify event timing, note-off handling, queue overflow reporting, and stop/reopen/playback.

### 5. MIDI, instruments, and routing

- [x] Add persisted MIDI part containers and beat-based notes, project v6 migration, arrange-view part selection, and a basic piano-roll editor with position snap, note-length choices, velocity, selection, deletion, and quantized move/resize gestures.
- [x] Schedule MIDI-part notes against the output timeline and render them through a basic sine voice, track EQ, and inserts; this is an OpenTrack- starter voice, not a Cubase instrument rewrite.
- [x] Add a reusable Rust instrument factory/processor interface and selectable sine, square, sawtooth, and triangle oscillator voices for Instrument tracks.
- [x] Add persisted controller lanes for modulation, volume, pan, and expression; schedule them in playback and restore values after seeks.
- [x] Add Windows MIDI input enumeration, callback capture, multi-track MIDI recording, and live MIDI Thru through the starter oscillator.
- [ ] Add MIDI output ports and typed MIDI/audio buses before adding richer instruments, sends, groups, or sidechains.
- [ ] Add automation lanes and deterministic playback/recording of automation.
- [ ] Expand input/output device and bus selection; evaluate ASIO only after a tested x64 WASAPI path and latency measurements.

### 6. Accuracy and compatibility

- [ ] Compare documented commands, project workflows, recording behavior, and each rewritten plug-in against Cubase 5.
- [ ] Continue bounded REA use for evidence gathering; keep the reference install and analysis tools outside the product repository.
- [x] Decode the observed length-prefixed ASCII label sequence in each outer ROOT block and inventory the labels across all 37 installed templates.
- [x] Confirm the x86 DLL process boundary and document the VST 2 distribution constraint for any optional legacy helper.
- [ ] Investigate CPR parsing/import against real samples and report unsupported data; do not infer round-trip support from inspection fixtures.
- [ ] Capture controlled Cubase-saved project differences and map CPR fields before implementing an importer.
- [ ] Establish a valid distribution path before evaluating a legacy VST 2 host; then prototype and measure a bounded 32/64-bit IPC path.
- [ ] Maintain a feature matrix with status: implemented, observed, tested, parity-verified, or unknown.

## Current milestone

The arrange view now supports independent row resizing, Ctrl-drag scaling of all rows, compact header controls, the Cubase track-zoom key bindings, and per-workspace restoration of row heights. Direct Cubase comparison remains pending; the exclusive zoom size and toggle are currently an informed implementation choice.

The CPR inspector reports observed big-endian ROOT/ARCH block framing and reads exact length-prefixed ASCII label strings from each ROOT payload. Of the 37 installed templates, 22 contain four ROOT/ARCH pairs and 15 contain six; the root labels form seven unique pairs across the set. The paired ARCH records remain opaque. No track payload decoder or CPR importer exists yet.

The current prototype provides selectable audio input capture, WAV takes, live recording-input and channel playback meters, optional audio input monitoring, Windows MIDI input capture and MIDI Thru through a starter oscillator, multi-track MIDI recording alongside multiple armed audio tracks, four-band EQ, Stereo Delay, the OpenTrack- stereo-linked Compressor, TestGenerator inserts, persisted MIDI parts with piano-roll and four controller lanes, per-note and per-controller MIDI channels, a reusable Rust instrument interface with four oscillator choices, persisted cycle locators with playback looping, bounded project undo/redo, unavailable-WAV relinking, and large-file streaming with background waveforms and editable, persisted Windows key commands and global workspace presets. The arrange view now has selectable tracks with direct mute, solo, record-enable, and Channel Settings controls. Mixer, Channel Settings, MediaBay, and MIDI Editor can open as separate windows with per-workspace visibility and geometry. Existing workspace checks cover audio recording-to-playback, callback and input meters, monitor resampling and mixing, streamed seeking, the instrument factory, oscillator shapes, controller scheduling, compressor response/stereo balance, and older project migrations. MIDI hardware timing, MIDI Thru, and physical-device recording and key command UI persistence and workspace preset behavior have not been manually checked. Cycle-loop behavior and the new track/window interactions need hands-on acceptance; Cubase layout matching needs controlled reference captures. MIDI output, typed buses and routing, richer instruments, cycle takes, long-file hardware acceptance, monitor latency/dropout acceptance, manual recording/device-latency acceptance, and Cubase parity comparisons remain open. Edit-history behavior is in [edit-history.md](edit-history.md); piano-roll facts are in [midi-editor.md](midi-editor.md); window/track evidence is in [window-and-track-fidelity.md](window-and-track-fidelity.md); audio boundaries remain in [audio-engine.md](audio-engine.md) and the [Cubase audio reference notes](cubase5-audio-reference.md).
