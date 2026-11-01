# OpenTrack implementation roadmap

This roadmap tracks the development of OpenTrack as a dependable open-source DAW. Feature priorities come from the user's workflows and the capabilities needed for recording, editing, arranging, mixing, and finishing music. Project-format interoperability is a product goal, with support established separately for each format and version.

## Working method

Delivery follows the feature task and acceptance process in [the workflow guide](workflow/README.md). Checked items record implemented scope. Unchecked items include both acceptance checks for implemented behavior and features still awaiting implementation; task wording indicates which applies. Current user-review gates are listed in [current workflow state](workflow/current.md) and their task packets.

For each feature, keep the intended behavior, implementation, and acceptance evidence distinct:

- **Feature requirements** from the user's workflow and explicit product decisions.
- **Implementation and verification** from source, automated checks, and runtime use of OpenTrack.
- **User acceptance and limitations** for the behavior delivered. Compatibility statements are scoped to the specific project format and version tested.

Each milestone should include a user-visible workflow, persisted project state where applicable, bounded real-time behavior, focused automated checks, and a manual acceptance procedure. Audio callbacks must not block on disk or UI work, allocate in steady state, or take contended locks. Recordings must report overrun/dropout conditions rather than silently implying a clean take.

## Milestones

### 0. Historical product research

Earlier reference research remains background context. It is not an active delivery milestone or a prerequisite for developing features.

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

- [x] Add persisted EQ bands (up to 32) to each audio channel and device bus, including enable and bypass controls.
- [x] Implement stereo filters with coefficients and per-channel state prepared outside the real-time callback, including 12 and 24 dB/octave pass filters and several shelf flavors.
- [x] Show an EQ graph and band strip with add/remove/selection, direct node dragging, per-band solo audition and a selected-band editor for gain, frequency, Q, filter type and dynamic controls.
- [x] Add dynamic EQ bands with threshold, range, attack, release and live gain movement; the graph and audio share one response model for shelves and bells.
- [x] Add live spectrum analysis with selectable resolution, tilt and peak hold, plus a separate tonal-reference capture/import mode with persisted targets.
- [x] Verify parameter bounds, neutral response, frequency response, stereo behavior, queue overflow handling, and backward-compatible track loading.
- [ ] Review the EQ controls, parameter persistence, and audio behavior in OpenTrack against the current feature specification.

### 3. Insert effects, VST3 hosting and first-party plug-ins

- [x] Add a persisted per-channel insert chain, enable/bypass, ordering, and parameter state.
- [x] Expose insert ordering controls in the Mixer so users can move processors within the chain.
- [x] Add the first effect processor and UI; run effects after clip summing and EQ and before the channel fader.
- [x] Original stereo-linked Compressor prototype completed, then removed from the built-in insert path on 2026-10-26 by user direction. Its old project record is retained as an inactive legacy slot; new first-party effect work targets VST3 plug-ins.
- [x] Add the TestGenerator control set to the insert chain using the original Rust DSP prototype.
- [x] Host third-party VST3 effects in a stereo insert slot: enumerate factories in a separate scanner process, negotiate a stereo main bus, process prepared blocks with delay compensation, save and restore component/controller state, expose normalized parameters, and attach native editor windows. Compatibility limits are recorded in the VST3 task notes; this is not universal support.
- [x] Replace the fixed insert rack with a compact single-line list: one slot by default, a `+` row that opens more slots (up to 24), a whole-row plug-in list button, right-anchored names, a power button that unloads the plug-in, a separate bypass button that keeps it loaded but out of the chain, and one eye button per plug-in that shows, hides and reopens its editor. Right-click removes a row.
- [x] Keep processor construction and memory allocation off the audio callback; define the initial effect's time, feedback, wet-mix, and stereo limits.
- [ ] Define expected controls, audio behavior, and state persistence for each selected plug-in as its own feature task.
- [ ] Verify plug-in behavior against its feature specification and document supported capabilities and limitations.
- [ ] Build first-party effects and instruments as separately loadable VST3 plug-ins, designed with the user, rather than adding new built-in DSP processors.

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
- [x] Add independent per-track height resizing, Ctrl-drag resizing for all tracks, compact row controls, workspace persistence, and three track-zoom key commands.
- [x] Reuse the open Channel Settings window when another channel's FX button is used; a new window is created only when none is open, and its insert/send racks stay compact.
- [x] Toggle playback by double-clicking the timeline block at the top of the Project, MIDI editor or Sample Editor.
- [ ] Manually accept track creation, rename, reorder, removal, undo, and MIDI/audio content preservation on the target PC.
- [ ] Verify window restoration across workspaces, display scaling changes, and monitor arrangements; save and validate monitor identity before claiming multi-monitor support.
- [ ] Review track heights, compact-row controls, zoom increments, and exclusive zoom in OpenTrack with the user; refine them against the intended workflow.
- [ ] Implement channel routing to support the intended recording and mixing workflows.
- [ ] Add media collection/relocation, autosave, crash recovery, and format migrations for future project fields.
- [x] Stream large audio sources through a bounded per-clip cache and generate their waveforms on a background scan without loading the entire source into memory.
- [ ] Manually accept long-file playback, seeking, and waveform generation under the target device's real-time load.
- [x] Add persisted beat-based left/right cycle locators and loop playback in the audio callback and fallback transport.
- [ ] Add fades, trim/edge editing, snap/grid, timebase choices, markers, and non-destructive clip processing.
- [ ] Verify old OpenTrack- project versions continue to load as fields are added.
- [ ] Accept MIDI input recording and MIDI Thru with a physical device; verify event timing, note-off handling, queue overflow reporting, and stop/reopen/playback.

### 5. MIDI, instruments, and routing

- [x] Add persisted MIDI part containers and beat-based notes, project v6 migration, arrange-view part selection, and a basic piano-roll editor with position snap, note-length choices, velocity, selection, deletion, and quantized move/resize gestures.
- [x] Schedule MIDI-part notes against the output timeline and render them through a basic sine voice, track EQ, and inserts.
- [x] Add a reusable Rust instrument factory/processor interface and selectable sine, square, sawtooth, and triangle oscillator voices for Instrument tracks.
- [x] Add persisted controller lanes for modulation, volume, pan, and expression; schedule them in playback and restore values after seeks.
- [x] Add Windows MIDI input enumeration, callback capture, multi-track MIDI recording, and live MIDI Thru through the starter oscillator.
- [ ] Add MIDI output ports and typed MIDI/audio buses before adding richer instruments, sends, groups, or sidechains.
- [ ] Add automation lanes and deterministic playback/recording of automation.
- [ ] Expand input/output device and bus selection; evaluate ASIO only after a tested x64 WASAPI path and latency measurements.

### 6. Project-format interoperability

- [x] Decode the observed length-prefixed ASCII label sequence in each CPR ROOT block and inventory the labels across the available templates.
- [ ] Build a CPR adapter from representative project files; map supported fields and preserve unknown data as unchanged stubs where the format permits.
- [ ] Verify CPR import and export round trips for named file versions, and report unsupported or lossy elements.
- [ ] Add other DAW project formats as separate adapters with versioned fixtures and explicit support statements.
- [ ] Establish a valid distribution path before evaluating a legacy VST 2 host; then prototype and measure a bounded 32/64-bit IPC path.
- [ ] Maintain a feature matrix with implementation, verification, user-review, and format/version support status.

### 7. Sample Editor, pitch and time

- [x] Open the Sample Editor from audio-event double-click and the Mixer lower-zone tabs; track-label double-click remains rename.
- [x] Persist editable pitch/time intent in the project and retain valid analysis by source identity/range and algorithm/model version. Analysis starts only when Pitch & Time is activated; save/reopen restores the analyzed state without repeating analysis, and source changes invalidate affected results.
- [x] Provide the Sample Editor's own persisted finer snap settings, a combined ruler with seek/zoom, middle-pan and edge scrolling, and ordinary cuts plus Pitch & Time instead of AudioWarp.
- [x] Keep waveform rendering, the live Channel Settings spectrum and pitch analysis as separate features; re-analysis is explicit, cancellable and never overwrites newer edits.
- [ ] Review editing, analysis and audible correction in OpenTrack, including source changes and cancellation.

## Current milestone

The arrange view supports independent row resizing, Ctrl-drag scaling of all rows, compact header controls, track zoom commands, and per-workspace restoration of row heights. The exact interaction remains subject to user review in OpenTrack.

The CPR inspector reports observed big-endian ROOT/ARCH block framing and reads exact length-prefixed ASCII label strings from each ROOT payload. Of the 37 available templates, 22 contain four ROOT/ARCH pairs and 15 contain six; the root labels form seven unique pairs across the set. The paired ARCH records remain opaque. No track payload decoder or CPR importer exists yet.

The current prototype provides selectable audio input capture, WAV takes, live recording-input and channel playback meters, optional audio input monitoring, Windows MIDI input capture and MIDI Thru through a starter oscillator, multi-track MIDI recording alongside armed audio tracks, a 32-band static and dynamic equalizer with live spectrum and tonal-reference analysis, an insert rack with built-in processors and third-party VST3 hosting including native plug-in editors, persisted MIDI parts with a piano-roll and four controller lanes, per-note and per-controller MIDI channels, a reusable Rust instrument interface with four oscillator choices, persisted cycle locators with playback looping, bounded project undo/redo, unavailable-WAV relinking, a Sample Editor with on-demand Pitch & Time analysis, and large-file streaming with background waveforms, editable Windows key commands and global workspace presets. Device buses and track/group channels apply persisted input trim and polarity ahead of the equalizer.

The arrange view has selectable tracks with direct mute, solo, record-enable, and Channel Settings controls. Mixer, Channel Settings, MediaBay, and MIDI Editor open as separate windows with per-workspace visibility and geometry; the Channel Settings window retargets in place when another channel's FX button is used, and its insert/send racks are compact one-line rows that start with one slot and grow from a `+` row. Project, MIDI editor and Sample Editor rulers toggle playback on double-click.

Existing checks cover audio recording-to-playback, callback and input meters, monitor resampling and mixing, streamed seeking, the instrument factory, oscillator shapes, controller scheduling, EQ response and dynamic behavior, spectrum/tonal analysis, VST3 module loading and parameter handling, and older project migrations. MIDI hardware timing, MIDI Thru, physical-device recording, VST3 behavior in real hosts, key-command UI persistence, and workspace preset behavior have not been manually checked. Cycle-loop behavior, long-file streaming under real-time load, monitor latency/dropout, and the EQ, Sample Editor and Channel Settings surfaces need hands-on acceptance. Automation, typed buses and routing, Send DSP, cycle takes, MIDI output, richer instruments, and manual recording/device-latency acceptance remain open. Project-format interoperability remains planned, with CPR as an initial format. Edit-history behavior is in [edit-history.md](edit-history.md); piano-roll facts are in [midi-editor.md](midi-editor.md); window/track requirements are in [window-and-track-fidelity.md](window-and-track-fidelity.md); audio boundaries remain in [audio-engine.md](audio-engine.md).