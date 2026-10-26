# VST3 plug-in hosting plan

- **Status:** planning only. Not active implementation.
- **Date:** 2026-10-25.
- **Product target:** x86_64 Windows, Rust, egui/eframe.
- **Scope:** VST3 effects and instruments, catalog/scanner, project state, editors, delay compensation, testing.
- **Not in this plan:** VST2, 32-bit hosting, CLAP/AU/AAX, surround, multi-out beyond the stereo main output, sidechain, process sandboxing in v1, and a plug-in automation lane UI in v1.
- **Relationship to current work:** T037/T038 remain the active review. This plan starts only after that review is resolved and the user approves the plan. It is outside the current `transport-v12-pitch-time-v4` ledger. When work starts, create a separate `vst3-hosting-v1` ledger scope and task packets; do not reuse transport metrics for plug-in hosting.
- **Version meaning:** in this plan, **v1** means milestones M0-M5: catalog/scanner, engine groundwork, VST3 effects, editors, and instruments. M6 (automation/offline/multi-out/sidechain/sandbox) and M7 (format expansion) are post-v1.

This document is a specification and delivery plan, not evidence that any of it works. Every runtime claim below remains pending until a build, audio test, save/reopen test, or user review establishes it.

## 1. Decisions

| ID | Decision | Reason |
| --- | --- | --- |
| D1 | **VST3 only, x64 Windows** for v1. | Every unique plug-in in the current collection is available as x64 VST3. VST2 distribution remains legally constrained and adds no capability for this collection. |
| D2 | **Build our own host** in `opentrack-vst3`; use the `vst3` crate only for generated ABI bindings. No `vst-audio` or other full host dependency. | Keeps host lifecycle, graph integration, state, and editors under OpenTrack control. Keeps the repo free of Steinberg-authored files. |
| D3 | **Scan out of process; host in process for v1.** | Scanning untrusted modules is the dangerous part. In-process hosting keeps v1 latency, editor, and graph work simpler. The host boundary remains swappable so process isolation can be added later. |
| D4 | **Plug-in instances live outside `PlaybackPlan`**, keyed by stable runtime IDs. | The current plan is rebuilt on many edits and crossfaded. Recreating VSTs per rebuild loses state; sharing them across two plans double-processes them. |
| D5 | **Refactor the channel path to block processing and add PDC before hosted inserts.** | VST3 is block-based and reports latency. A per-sample host cannot be correct or efficient. |
| D6 | **Effects first, instruments second.** | Instruments require the MIDI event path, live-input routing, and more latency work. |
| D7 | **Stereo in/out only in v1.** | Keeps the bus model bounded. Multi-out instruments use their stereo main output; multi-out routing and sidechains come later. |
| D8 | **Floating native editor windows first.** No egui embedding in v1. | Matches the Cubase 5 reference style and avoids immediate-mode/child-HWND compositing problems. |
| D9 | **Class ID is identity; path is a hint.** State lives in project-side sidecars. | Plug-ins move and duplicate. The project must survive either without losing state. |
| D10 | **Apache-2.0, no CLA, no paid-licensing plan.** | The user wants broad reuse and does not intend to profit from the project. Apache-2.0 supplies a patent grant and a clear notice obligation without copyleft. |
| D11 | **No VST2, 32-bit, CLAP, or sandbox in v1.** | Separate compatibility/format projects. Re-evaluate after the VST3 workflow is accepted. |
| D12 | **Built-in effects and the oscillator instrument remain.** Hosted plug-ins are additive. | Preserves existing workflows and tests; the built-in path is the fallback when no plug-in is loaded. |
| D13 | **Expose parameters and gestures in v1; no automation lanes yet.** | The project has no automation engine. The parameter model and undo gestures must be correct before lanes are added. |
| D14 | **No Steinberg-authored files in the repository.** The only VST3-derived code is the third-party `vst3` bindings dependency, used behind `opentrack-vst3`. | User constraint. Do not vendor `pluginterfaces`, `public.sdk`, VSTGUI, or SDK sample code. |

## 2. Current-state audit

### 2.1 Code facts

- `crates/opentrack-audio/src/plugins/mod.rs` exposes only the built-in `test_generator`; there is no third-party loader, host context, or scanner.
- `crates/opentrack-core/src/project.rs` stores built-in inserts as `Vec<InsertEffect>` with `InsertEffectKind`. There is no hosted-plug-in reference or opaque state field.
- `crates/opentrack-audio/src/lib.rs` builds a `ChannelProcessor` per track. `ChannelProcessor::process(&mut left, &mut right)` is called once per sample.
- `crates/opentrack-app/src/app.rs` rebuilds the entire `PlaybackPlan` from project state in `rebuild_audio_plan()`. `AudioEngine::set_playback_plan_with_metronome()` swaps in a new immutable plan and crossfades the old and new plan for 10 ms.
- The UI already has eight insert slots in `crates/opentrack-app/src/channel_settings.rs`, but the picker only offers built-in effects.
- Instruments are built-in oscillator voices created through `InstrumentFactory` / `InstrumentProcessor`. `InstrumentProcessor::next_frame()` renders a frame; there is no sample-accurate host event list.
- Live MIDI is rendered by a global `LiveMidiRenderer` and bypasses per-track DSP. It is not routed to a selected instrument track.
- There is no automatic delay compensation. `track_delay_ms` is a manual user delay, not measured plug-in latency.
- There is no automation engine, plug-in state store, missing-plug-in UI, scanner, catalog, or native editor host.

### 2.2 Why the current engine is the real constraint

The VST3 ABI is a bounded implementation problem. The larger risk is fitting hosted plug-ins into the current audio graph:

1. **Plan lifetime.** If a VST instance lives inside `PlaybackPlan`, every plan rebuild recreates it, or the 10 ms transition calls `process()` twice on the same instance. Both are wrong. Instances must be long-lived runtime objects.
2. **Per-sample processing.** VST3 expects a block with planar buffers, events, parameter changes, and a process context. The current per-sample `ChannelProcessor` must become block-aware.
3. **MIDI.** VST instruments need note events with sample offsets. The current instrument factory renders notes internally. Live MIDI needs to reach the selected instrument track, not a global oscillator that bypasses track DSP.
4. **Latency.** Any look-ahead compressor or linear-phase EQ shifts one track against another unless PDC is implemented.
5. **State.** A plug-in's component and controller chunks are opaque and can be large. They need a versioned, portable representation and a missing-plug-in path.
6. **Editors.** The host must implement `IPlugFrame`, own native HWNDs, and handle DPI, resize, focus, and message pumping.
7. **Threading.** The audio callback must not load, create, destroy, allocate, lock, or call controllers/editors. All of that belongs on control/UI threads.

### 2.3 Current plug-in inventory

A read-only scan of the user's folders on 2026-10-25 found:

- `C:\Program Files\Common Files\VST3`
  - 10 unique x64 VST3 modules: Gateway, Ozone 8, Tonal Balance Control 2, NeuralAmpModeler, ParametricOD, sforzando, Retrologue, Superior Drummer 3, TDR Nova, and WaveShell1.
  - 16 module paths if duplicate bundles are counted recursively. `Gateway.vst3`, `NeuralAmpModeler.vst3`, `ParametricOD.vst3`, `sforzando.vst3`, `Retrologue.vst3`, and `Superior Drummer 3.vst3` appear both as bundles and as module files; Superior Drummer 3 also appears at the top level and under `Toontrack\`.
  - `WaveShell1-VST3 15.0_x64.vst3` is a shell module exposing multiple plug-in classes. The catalog must enumerate the child classes and use the class ID, not the module name.
  - `Superior Drummer 3` is an instrument with large state and multiple outputs. v1 targets its stereo main output only.
- `C:\Program Files\Steinberg\VSTPlugins`
  - 4 x64 VST2 DLLs: Ozone 8 (two DLLs), TDR Nova, and Superior Drummer 3. All of these have VST3 versions installed. VST2 is deferred.
- `C:\Program Files (x86)\Steinberg\VSTPlugins`
  - 2 x86 VST2 DLLs. They cannot load in the x64 host and must be reported as unsupported if the folder is added.
- The `vst3` and `VSTPlugins` folders under `G:\dev\Opentrack` are Cubase reference-install artifacts outside the OpenTrack repository. They are not an SDK and must not be scanned by default.

The module count is not the class count: WaveShell alone exposes multiple child classes, and the catalog must list those classes individually. This inventory is a test snapshot, not a promise of universal VST3 compatibility.

## 3. Scope and non-goals

### 3.1 In scope for v1

- VST3 folder management, out-of-process scanning, catalog cache, and plug-in manager.
- VST3 effect inserts on audio and instrument tracks (any track with an audio path).
- VST3 instrument slot on instrument tracks, with the built-in oscillator retained as fallback.
- Stereo main input/output; mono-to-stereo where the plug-in declares it.
- Host-level bypass with a click-free crossfade.
- Generic parameter UI, parameter gestures, and basic program/preset selection if the plug-in exposes it.
- Floating native editor windows.
- Component and controller state save/restore, missing-plug-in placeholders, and safe mode.
- Automatic delay compensation for insert chains and instrument latency.
- Sample-rate and buffer-size changes with plug-in re-preparation.
- Crash detection at scan time, blacklist, autosave-aware recovery, and a "start without plug-ins" safe mode.
- Audible, state, editor, and persistence tests with the current plug-in collection.

### 3.2 Deferred but planned

- Multi-out instrument routing.
- Sidechain buses and sidechain routing UI.
- Plug-in automation lanes and sample-accurate automation editing.
- Offline render/bounce with plug-in pre-roll and tail.
- `.vstpreset` import/export and a full preset browser.
- Process-isolated hosting and sandboxing.
- Freeze/bounce-as-new-track workflows.
- CLAP, VST2, 32-bit bridging, AU/AAX.
- Surround and non-stereo speaker layouts.
- Plug-in MIDI output routing.

### 3.3 Explicit non-goals

- Recreating Cubase's bundled plug-ins.
- Claiming Cubase plug-in visual or DSP parity.
- A universal VST3 compatibility guarantee. Plug-ins are third-party code; host correctness and plug-in correctness are separate.
- Shipping plug-in binaries, SDK headers, or SDK sample code in the repository.
- A paid/commercial licensing model.

### 3.4 Compatibility policy

- **Supported:** x64 VST3 audio effects and instruments on Windows 10/11, with a mono or stereo main audio path.
- **v1 bus policy:** stereo output; mono-to-stereo where declared; instruments use their stereo main output. Sidechains and additional outputs are rejected or shown as unsupported, not silently misrouted.
- **Unsupported:** VST2, 32-bit modules, ARM64 modules, AAX/AU, surround/non-stereo layouts, and plug-ins requiring host-specific extensions OpenTrack does not implement.
- **Best-effort:** individual third-party plug-ins. A plug-in that fails validation is quarantined with diagnostics; it is not a promise that every plug-in works.
- **No silent fallback:** if a plug-in cannot be loaded or its layout cannot be negotiated, the slot shows an explicit error and preserves saved state.

## 4. Architecture

### 4.1 Crate and module boundaries

```text
opentrack-core
  PluginFormat, PluginClassId, PluginSlotId, PluginStateRef
  project schema and migration only; no VST3 types

opentrack-plugins
  catalog, descriptors, cache schema
  PluginHost / PluginInstance / PluginController traits
  scan protocol, errors, parameter/event/process types

opentrack-vst3
  vst3-crate bindings
  host context, component/controller lifecycle
  buses, processing, parameters, state, editor view
  no egui types

opentrack-plugin-scan
  helper binary; no egui
  owns the scanner process lifecycle and JSON output

opentrack-audio
  block graph, plugin runtime registry, PDC, event queues
  built-in effects/instruments remain

opentrack-app
  preferences, plugin manager, insert/instrument pickers
  missing-plugin UI, safe mode, native editor window host
```

The application depends on the format-neutral facade. `opentrack-vst3` types must not appear in `opentrack-core`, project files, or egui view code. The app may register the VST3 backend at startup, but the runtime interface is `PluginHost`/`PluginInstance`. Only `opentrack-vst3` depends on the `vst3` crate; `opentrack-plugin-scan` depends on `opentrack-vst3` to perform introspection.

### 4.2 Runtime topology

```text
Preferences folders
        |
        v
opentrack-plugin-scan.exe  (separate process, disposable)
        |  descriptors + class metadata
        v
Catalog/cache  (class ID, vendor, version, buses, params, status)
        |
        v
Project slot (class ID + state ref + bypass)
        |
        v
PluginRuntime registry (control thread owns controller/editor;
                        audio thread owns processor)
        |
        v
Block audio graph (inserts, EQ, fader/pan, PDC, meters)
        |
        v
Native editor windows (UI thread)
```

### 4.3 Lifecycle and state machine

A plug-in instance moves through:

```text
Discovered -> Loading -> Ready -> Processing
                    \-> Failed
Ready -> Bypassed -> Processing
Any -> Missing (project load, no module)
Any -> Recovering (optional later sandbox)
Any -> Closed
```

Rules:

- `Loading` happens on a control thread, never the audio callback.
- `Processing` is the only state in which the audio callback calls `process()`.
- `Bypassed` still keeps the instance alive; whether it processes for tails is a per-host policy, documented below.
- `Failed` keeps the slot and state so the user can retry, replace, or remove it.
- `Missing` is a project-load state, not a scan state. It preserves the saved state and name.
- `Closed` is reached after the graph no longer references the instance and the callback has retired it.

### 4.4 Host context responsibilities

The VST3 host must implement, at minimum:

- `IHostApplication` for plug-in instance creation and host name.
- `IComponentHandler`, `IComponentHandler2`, `IComponentHandler3` for edit gestures, dirty state, editor requests, and context menus.
- `IPlugFrame` for editor resize negotiation.
- `IBStream` for component/controller state and preset streams.
- `IMessage` and `IAttributeList` for host/plug-in messages.
- A host context object passed to `IPluginBase::initialize`.

The host must create the component and controller, connect them through `IConnectionPoint` when they are separate, set the controller's component handler, negotiate buses, call `setupProcessing`, activate, and only then process. On teardown: stop processing, deactivate, disconnect, terminate, release.

### 4.5 Audio block contract

The engine passes blocks with:

- planar f32 buffers (left/right for v1; the ABI layer can adapt mono/stereo),
- `frames` (up to the prepared maximum),
- sample position and musical position,
- tempo, time signature, cycle range, playing state, and system time,
- input events with sample offsets,
- input parameter changes with sample offsets,
- output events and output parameter changes.

The host must preallocate buffers, event lists, and parameter queues. The audio callback must not allocate, lock, or perform file/UI/COM work. If the engine's device buffer exceeds the prepared maximum, split it into prepared-size blocks rather than reallocating.

### 4.6 Graph ownership and plan swaps

Introduce a `PluginRuntimeRegistry` owned by the audio engine:

- Each hosted insert has a stable `PluginSlotId` and a runtime handle.
- The handle owns the audio-side processor; the control side owns the controller, editor, and state access.
- `PlaybackPlan` (or its successor) stores handles/IDs, not VST objects that are rebuilt with the plan.
- Parameter-only changes update the runtime and mix parameters; they do not rebuild or recreate plug-in instances.
- Structural changes (add/remove/replace a plug-in, change instrument) use a short fade-out, graph swap, fade-in. The old instance is retired after the callback generation completes.
- The existing 10 ms dual-plan crossfade must not call the same plug-in instance twice. For hosted tracks, use one live graph with smoothed gain/pan/EQ and a structural-change fade instead. The built-in-only path may keep the old crossfade until the block graph is fully adopted.
- No plug-in object is freed on the audio thread. Retirement uses the same control-thread reclamation discipline as current playback plans.

### 4.7 Delay compensation

- Query latency after `setupProcessing`; treat latency changes (`restartComponent` / latency-changed notifications) as graph changes.
- Per-track insert latency is the sum of the active chain's reported latency plus instrument latency.
- Compute the maximum path latency to the stereo master. Delay each track's output by `max_latency - track_latency` before summing.
- Report per-slot and per-track latency in the UI.
- Use smoothed compensation changes; a latency change must not click or phase-jump.
- Add a "Constrain Delay Compensation" option later for live monitoring; v1 reports latency and compensates playback.
- Offline render (post-v1) pre-rolls by the maximum latency and flushes `getTailSamples()`; trim the pre-roll and keep the tail.

### 4.8 Bypass

- Bypass is host-level, not a plug-in parameter.
- Keep the plug-in processing during bypass when tails matter; crossfade dry/wet over a short window. If the plug-in is suspended, document that tails stop.
- Bypass must be latency-aligned; a bypassed look-ahead plug-in must not shift the dry path.
- Toggling bypass is undoable and marks the project dirty.
- In a process-isolated host (post-v1), a plug-in crash during bypass/recovery must leave the dry path audible. In-process v1 relies on safe mode and project recovery.

### 4.9 Events and MIDI

- Add a format-neutral `PluginEvent` model: note on/off, poly pressure, control change, pitch bend, channel pressure, program change, and bounded SysEx.
- Build events per block from MIDI parts and the live MIDI queue, with sample offsets.
- Instrument tracks route events to the hosted instrument; the built-in oscillator remains the fallback.
- Live MIDI routes to the selected/armed instrument track, not a global oscillator. Monitoring and recording remain separate concerns.
- VST3 event buses carry MIDI; there is no MIDI bus. `IMidiMapping` maps CCs to plug-in parameters where the plug-in supports it.
- Note-expression/MPE and MIDI output are later.

### 4.10 State and persistence

Project state must be format-neutral and portable:

```text
PluginSlot
  instance_id: stable u64
  format: Vst3
  class_id: [u8; 16]
  saved_name, vendor, version
  path_hint: optional absolute path
  bypassed: bool
  state: PluginStateRef
```

- Identity is `format + class ID`, not the module path.
- State is an envelope with magic, envelope version, class ID, saved plug-in version, component chunk, and controller chunk.
- The proposed v1 layout stores state in a project-side sidecar under `Plugins/<instance-id>.otstate` in a managed project folder. For older projects without managed folders, use a sibling `<project-stem>.plugins/` folder. The project stores the relative path, size, and hash. Managed-project creation and migration add `Plugins/`.
- `Save As` copies referenced sidecars. Deleting a slot does not immediately delete state; prune unreferenced sidecars only through an explicit cleanup or after undo history no longer references them.
- On load, create the component/controller, apply component state, call the controller's `setComponentState`, apply controller state, then re-query parameters. Order matters.
- A missing plug-in keeps the slot, descriptor, saved name, and state. The UI offers Retry, Locate/Replace, and Remove. Never silently drop the insert.
- A version mismatch attempts restore, then reports a warning while preserving state.
- `getState` runs on the control/UI thread, not the audio callback. Large states may show progress; autosave and crash recovery must include them.
- Project schema version increments; older projects load with empty hosted slots. Test the full supported native-version interval.
- Undo covers add/remove/replace, bypass, and coalesced parameter gestures. Opaque state edits from the plug-in UI mark the project dirty and save with the project; they are not one undo step per parameter tick.

### 4.11 Editor hosting

- One top-level native window per editor, owned by the main OpenTrack window, titled with the plug-in name.
- Create `IEditController::createView("editor")`, verify `isPlatformTypeSupported("HWND")`, set the host `IPlugFrame`, query size, create/show the HWND, then attach the view with `kPlatformTypeHWND`.
- Handle `IPlugFrame::resizeView` by resizing the window and calling `IPlugView::onSize` as the SDK host does.
- Handle DPI with `IPlugViewContentScaleSupport` where supported; otherwise use the window's DPI and document scale limitations.
- Handle close, hide/show, focus, multiple editors, and window geometry. Use owner-relative pinning like Channel Settings; never use global topmost.
- When the plug-in window has focus, route keystrokes to the view; do not let DAW shortcuts steal text/parameter input. Transport shortcuts remain a documented policy.
- Initialize COM as needed on the UI/control thread (apartment-threaded); never initialize or call COM on the audio thread. Some plug-ins use COM internally.
- Provide a generic egui parameter window when a plug-in has no editor or the editor fails.
- The eframe/winit message pump must be proven to dispatch messages for the editor HWND. If it does not, create a dedicated UI thread with a message loop and marshal controller calls. This is a phase-0 spike, not an assumption.

### 4.12 Scanner and catalog

**Discovery**

- Default VST3 folders: `C:\Program Files\Common Files\VST3` and `%LOCALAPPDATA%\Programs\Common\VST3`.
- User can add/remove folders; each folder can be enabled/disabled.
- Recursively find `.vst3` bundles and single-file `.vst3` modules. For bundles, find the module under `Contents\x86_64-win\`; do not recurse into `Contents\Resources` or documentation.
- Read the PE header before loading. Reject x86/ARM64 in the x64 host with a clear status. Never attempt to load an incompatible module.
- Use wide/UTF-16 Windows paths and long-path-aware file APIs at the ABI boundary; never lossy-convert module paths.
- Use `moduleinfo.json` when present as a fast metadata path; still instantiate in the scanner for buses, parameters, latency, and a smoke test.
- Deduplicate by class ID. Prefer the canonical bundle module path; remember duplicate paths for diagnostics.
- Enumerate every audio-effect class, including shell children (WaveShell).
- Skip the Cubase reference-install folders unless the user explicitly adds them; they are x86 reference artifacts.

**Scanning**

- Run in a separate `opentrack-plugin-scan.exe` process with no egui.
- Use a job object with kill-on-close; per-module timeout (for example 20 seconds); a hung or crashed module is quarantined and the scan continues.
- Call `InitDll`/`GetPluginFactory`/`ExitDll` correctly; unload only after all instances/views from that module are gone.
- Instantiate each class with a minimal host context, query buses/parameters/latency/tail, and process a short silent block to catch obvious failures. Do not claim this proves DSP correctness.
- Some plug-ins show authorization dialogs or require a license; use a hidden message pump where possible, time out, and mark "needs authorization" rather than hanging.
- Bound all metadata strings, class counts, bus counts, and parameter counts; treat scanner output as untrusted input.
- The scanner process is a crash boundary, not a security sandbox. Do not claim otherwise; optional OS sandboxing is post-v1.
- Persist scan progress and the offending module so a crash can be attributed and blacklisted.

**Cache**

- Store at `%LOCALAPPDATA%\OpenTrack\PluginCache\catalog-v1.json` (atomic write).
- Key on module path, size, mtime, scanner version, host version, and format.
- Store class ID, names, vendor, version, category, buses, parameter count, architecture, status, errors, and scan timestamp.
- Do not store plug-in state in the catalog.
- "Scan changed only" is the default; "Rescan all" and "Clear cache" are explicit actions.
- A blacklist entry is keyed by module path/hash and status; "Try again" clears it.

### 4.13 Project model migration

To preserve insert order and older projects, add a hosted variant to the existing insert model rather than a parallel list:

```text
InsertEffect
  enabled: bool
  kind:
    Built-in variants (existing)
    HostedPlugin(HostedPluginRef)

HostedPluginRef
  slot_id
  format
  class_id
  saved_name / vendor / version
  path_hint
  state_ref
```

`Track` also gets an optional `instrument_plugin: Option<HostedPluginRef>` while keeping `instrument_waveform` as the fallback. This preserves `Vec<InsertEffect>` ordering and lets old `.opr/.otp/.rwp` files load with no hosted entries. A later cleanup may introduce a dedicated `InsertSlot` enum, but v1 does not need that churn.

### 4.14 Threading rules

| Thread | Owns | Must not do |
| --- | --- | --- |
| UI | controller, editor, parameter display, state requests | call `process()`, load modules |
| Control | create/destroy instances, prepare/activate, graph swaps, state I/O | block the audio callback |
| Audio | audio-side processor, event/parameter queues, PDC buffers | allocate, lock, touch HWND/UI, call controller/editor, load/unload |
| Scanner | foreign module loading, metadata, smoke test | touch the project or audio device |
| Editor UI | native editor window and plug-in view | process audio |

All cross-thread communication uses bounded queues or preallocated snapshots. The callback uses atomics/queue reads only. Structural changes fade, swap, and retire on the control thread. No unbounded channels or hidden allocations; evaluate any new queue dependency against real-time behavior before adding it.

## 5. UI and user workflow

### 5.1 Preferences → Plug-ins

- VST3 folder list with default system and per-user folders.
- Add Folder, Remove, Enable/Disable, Rescan, Rescan Changed, Clear Cache.
- Scan on startup toggle; background scan with progress and cancel.
- Safe mode / "Start without plug-ins."
- Status: last scan time, number of modules/classes, failures, cache location.

### 5.2 Plug-in Manager

- Searchable vendor/category tree or list.
- Name, vendor, version, format, architecture, module path, class ID, bus layout, parameter count, reported latency, scan status.
- Favorites and recent.
- Blacklist/timeout/crash status with "Try again."
- "Rescan selected," "Reveal in Explorer," "Copy diagnostics."
- Missing-plug-in entries from the current project, with Replace/Remove.

### 5.3 Insert picker

- Searchable, categorized list of VST3 effects.
- Recent and favorites.
- Shell children shown as individual plug-ins.
- Unsupported/arch-mismatch modules disabled with a reason.
- "Scan for new plug-ins" shortcut.
- Show reported latency and mono/stereo support.
- Selecting a plug-in creates a slot; the slot shows power/bypass, name, latency, editor button, replace, remove.

### 5.4 Instrument picker

- VST3 instruments plus the existing built-in oscillator.
- Same search/status/editor behavior.
- Instrument track shows the selected plug-in and its state; no plug-in means the oscillator fallback.
- Multi-out instruments initially expose only the stereo main output; additional outputs are clearly marked "not routed in this version."

### 5.5 Missing plug-in and recovery

- Project open is never aborted because a plug-in is missing or fails.
- The slot shows "Missing: <saved name>" with Retry, Locate/Replace, Remove.
- Saved state is retained across save/reopen.
- A plug-in that fails during load is "Failed" with diagnostics; the rest of the project opens.
- After an abnormal exit, offer "Start with plug-ins disabled" and list recently loaded modules.
- A scan crash never blocks startup; the module is quarantined until retried.

### 5.6 Editor windows

- Floating, owned by the main window; optional pin.
- Resizable only when the view allows it.
- Remember position/size per project workspace (not global topmost).
- Multiple editors can be open; closing an editor does not unload the plug-in.
- Generic parameter window as fallback.
- No editor embedding inside egui in v1.

## 6. Licensing and provenance

### 6.1 Project license

- Add `LICENSE` with Apache License 2.0.
- Add `NOTICE` with the OpenTrack copyright line.
- Set `license = "Apache-2.0"` in `[workspace.package]` and `license.workspace = true` in each crate.
- Add a short License section to `README.md`.
- Optional: SPDX headers in new source files.
- Optional: a one-line `CONTRIBUTING.md` statement that contributions are submitted under Apache-2.0.
- Contributions are accepted under Apache-2.0 section 5 by default; no CLA, copyright assignment, or separate contributor agreement.
- No paid licensing, dual licensing, or proprietary edition plan.

Apache-2.0 permits others to use the project commercially, including closed-source forks. That is accepted. Apache-2.0 does not grant trademark rights, so the OpenTrack name is not automatically licensed to forks; a formal trademark policy is deferred until it is needed.

### 6.2 VST3 bindings

- Use the `vst3` crate (MIT OR Apache-2.0) for generated ABI bindings only.
- Pin an exact version; review its provenance and host-side interface coverage in the phase-0 probe.
- Do not vendor `pluginterfaces`, `public.sdk`, VSTGUI, or SDK sample code.
- Do not commit SDK sample plug-ins or third-party plug-in binaries.
- Add the `vst3` crate license and provenance to `docs/third-party-notices.md` and the shipped notices.
- If the crate's host coverage is insufficient, stop and decide between generated bindings outside the repository, a hand-written minimal ABI subset, or a different approach. Do not silently add SDK files to the repo.
- If the requirement is **no Steinberg-derived code anywhere, including dependencies**, stop: this plan does not satisfy that. That would require a clean-room ABI implementation and legal review, which is a separate project.

### 6.3 Third-party notices

- Extend `docs/third-party-notices.md` with:
  - `vst3` crate (MIT OR Apache-2.0),
  - existing `maolan-asio` (BSD-2-Clause),
  - egui/eframe, cpal, wasapi, windows/windows-sys, hound, serde, rfd, and any new dependency.
- Add a build/packaging step that copies the notices beside `OpenTrack.exe`, matching the existing `maolan-asio-LICENSE.txt` pattern.
- Add `cargo-deny` configuration and a CI check for licenses, advisories, and duplicate/unknown dependencies.
- Use `cargo-about` or an equivalent generator if the notices file becomes large.

### 6.4 VST2 and 32-bit

- No VST2 SDK, headers, or host implementation in v1.
- The existing `docs/legacy-plugin-hosting.md` remains the record of the VST2 distribution constraint.
- No x86 helper in v1. The x86 VST2 duplicates in `Program Files (x86)` are reported as unsupported if scanned.
- Revisit VST2/32-bit only through a separate, legally reviewed compatibility plan.

### 6.5 Trademark

- Use "VST3" descriptively in UI and documentation.
- Do not use Steinberg logos or imply endorsement.
- Follow Steinberg's VST usage guidelines if branding is added.

### 6.6 Build and packaging

- `build.ps1` builds `opentrack-plugin-scan.exe` for x64 and copies it beside `OpenTrack.exe`.
- The installer and portable build include the scanner and notices; they do not include plug-in binaries, SDK headers, or SDK samples.
- Add an app/scanner protocol/version handshake; refuse a mismatched scanner with a clear error instead of loading plug-ins with an incompatible protocol.
- Run the scanner as the normal user, not elevated.
- Release builds should be Authenticode-signed when distribution exists; never sign third-party plug-ins.
- Document antivirus scan-performance expectations; do not add exclusions automatically.

## 7. Test and evidence plan

### 7.1 Evidence levels

Keep these separate:

1. Source presence and static review.
2. Compilation and unit tests.
3. Headless integration/audio tests.
4. Real plug-in runtime and audible checks.
5. Editor interaction and visual checks.
6. Save/reopen, undo, and recovery checks.
7. User acceptance.

Compilation never proves audio, editor, or persistence behavior. A plug-in's own DSP/UI is its own; OpenTrack's host behavior is what we test.

### 7.2 Phase-0 probe

Build an external probe under `_analysis-tools/vst3-probe` (outside the OpenTrack repository) that:

- depends on `vst3` 0.3 and loads an x64 VST3 module,
- enumerates classes and shell children,
- creates a component/controller with a host context,
- negotiates stereo buses and processes a block,
- opens a native editor,
- saves/restores state,
- prints reported latency and tail.

Acceptance: TDR Nova loads, processes, opens its editor, and restores state; the host-side interfaces are available; no Steinberg-authored files appear in `git ls-files`. If any of this fails, stop and revise the ABI strategy before adding the dependency to the product.

### 7.3 Scanner tests

- Scan the default folder and a custom folder; find the 10 unique modules.
- Deduplicate duplicate bundle/module paths by class ID.
- Enumerate WaveShell child classes.
- Report `Program Files (x86)` DLLs as x86/unsupported.
- Malformed `.vst3`, wrong bitness, missing dependency, crash, hang, and authorization-dialog fixtures.
- Cache hit/miss after mtime change, scanner-version change, and folder removal.
- Cancel a scan; restart after a scanner crash; blacklist and "Try again."
- Confirm no project mutation and no audio-device access during scan.

### 7.4 Host tests

- Instantiate each in-scope effect and instrument class.
- Process silence and a known sine/impulse; check finite output and no host crash.
- Verify bus negotiation and clear errors for unsupported layouts.
- Verify bypass null against the dry signal after latency alignment.
- Save state, create a fresh instance, restore, and compare parameter values/state bytes.
- Test 44.1, 48, and 96 kHz; 64/128/256/512-frame blocks.
- Test sample-rate and buffer-size changes while loaded.
- Test missing module, missing class, version mismatch, and moved module.
- Test multiple instances of the same class.
- Test parameter gestures, UI-originated edits, and dirty state.
- Test editor open/close/reopen and audio continuity.

### 7.5 Audio graph tests

- **PDC null test:** two identical tracks, one with a latency plug-in, invert one; residual stays below a documented threshold.
- **Bypass null test:** dry vs bypassed hosted insert after compensation.
- **Block-boundary test:** events at block edges, cycle wrap, transport stop/start.
- **Offline render test:** deterministic effect rendered offline matches real-time capture within a documented tolerance; tail is flushed and trimmed.
- **Plan-swap test:** adding/removing a hosted insert fades and swaps without double-processing or use-after-free.
- **Meter/spectrum test:** hosted insert output feeds the existing post-insert meter/spectrum taps correctly.

### 7.6 Real-plug-in matrix

| Plug-in | Type | v1 focus |
| --- | --- | --- |
| TDR Nova | Effect | Basic process, editor, state, latency, automation gestures |
| NeuralAmpModeler | Effect | Mono/stereo handling, model/state restore, latency |
| ParametricOD | Effect | Basic process, parameters, editor |
| Gateway | Effect | Basic process, bypass, state |
| Ozone 8 | Effect | Heavy state/editor, latency, save/reopen |
| Tonal Balance Control 2 | Effect/metering | Editor, state, stereo operation |
| sforzando | Instrument | Note events, SFZ file/state, editor |
| Retrologue | Instrument | Note events, presets/state, editor |
| Superior Drummer 3 | Instrument | Large state, editor, stereo main only; multi-out deferred |
| WaveShell1 | Shell | Class enumeration, child load, state identity |

VST2 Ozone/TDR/SD3 entries are explicitly unsupported in v1. The UI must say why.

### 7.7 Performance

- Measure CPU cost per plug-in and per track at 128/256/480-frame buffers.
- Measure scan time and cache load time.
- Measure memory per instance and project open with large state.
- Record xrun/dropout counts on the Onyx/WASAPI and local-ASIO paths.
- Compare bypass/active CPU and report plugin-process time where feasible.
- Set a per-instance CPU/memory budget for the v1 acceptance review.

### 7.8 Editor tests

- Open/close/resize each editor; verify reported size and actual window size.
- Test DPI at 100%/125%/150%/200% and across monitors where available.
- Test focus and keyboard input; verify DAW shortcuts do not steal parameter text.
- Test multiple editors and owner-relative pinning.
- Test modal dialogs and file pickers (sforzando/Ozone).
- Capture screenshots for visual review; do not call them Cubase parity.

### 7.9 Persistence and recovery tests

- Save/reopen with effects and instruments; verify audio and state.
- Save As copies sidecars; moving the project folder keeps state.
- Missing plug-in preserves slot/state; replacing it restores audio.
- Undo/redo add/remove/replace/bypass/parameter gestures.
- Autosave/crash recovery includes state.
- Safe mode starts with plug-ins disabled and the project intact.
- Older `.opr/.otp/.rwp` files load without hosted slots.
- Project version migration is tested across the supported interval.

## 8. Milestones

Each milestone ends at a review stop. Do not start the next milestone until the current one is accepted.

### M0 — Decisions, licensing, and ABI probe

**Deliverables**

- This plan approved.
- Apache-2.0 license/NOTICE/Cargo metadata and third-party-notice scaffolding.
- External `vst3`-crate probe; host-side API coverage report.
- Decisions recorded: state sidecar location, editor thread model, scanner process model, in-process hosting v1.
- License audit: `git ls-files` contains no Steinberg-authored files; `cargo-deny` baseline passes.

**Acceptance**

- Probe loads TDR Nova, enumerates classes, processes a block, opens an editor, saves/restores state, and reports latency.
- No OpenTrack source or build dependency on `vst-audio`.
- No SDK headers committed.
- User approves the host/ABI/state decisions.

**Stop:** architecture and licensing review.

### M1 — Catalog, scanner, preferences, and plug-in manager

**Deliverables**

- `opentrack-plugins` descriptors/catalog/traits.
- `opentrack-plugin-scan` helper binary with timeout, job object, PE check, shell enumeration, cache.
- Preferences → Plug-ins page and Plug-in Manager.
- Default folders and user folder management.

**Acceptance**

- Finds the 10 unique VST3 modules, deduplicates duplicate paths, lists WaveShell children, reports x86 VST2 as unsupported.
- Survives malformed, crashed, hung, and authorization-dialog fixtures without taking down the app.
- Cache survives restart and invalidates on change.
- UI stays responsive; cancel works.
- No audio/project changes yet.

**Stop:** user reviews the catalog and scanner behavior.

### M2 — Block graph, runtime registry, and PDC

**Deliverables**

- Block-based track/channel path; built-in effects/EQ/instrument retain behavior.
- Stable `PluginRuntimeRegistry`; plan references runtime handles.
- Structural-change fade and retirement; no double-processing.
- PDC delay lines, latency reporting, null tests.
- Event/parameter queue scaffolding.

**Acceptance**

- Existing workspace tests pass; playback, recording, metering, cycle, and Sample Editor behavior do not regress.
- PDC null test passes; plan-swap stress test passes.
- A mock plug-in node processes blocks correctly.
- No allocations/locks in the callback (instrumented test where practical).
- CPU/latency baseline recorded.

**Stop:** timing/threading/audio review. This is the highest-risk milestone.

### M3 — VST3 effects

**Deliverables**

- `opentrack-vst3` effect host: component/controller, buses, process, parameters, state, bypass.
- Insert picker, hosted slots, generic parameter window.
- Missing-plug-in handling and state sidecars.
- Scanner-to-runtime descriptor resolution.
- Safe mode, blacklist, and project-recovery integration for failed plug-ins.

**Acceptance**

- TDR Nova, NeuralAmpModeler, ParametricOD, Gateway, and Ozone 8 process audible audio on a track.
- Bypass is click-free and nulls against dry after PDC.
- State saves/reopens in a fresh project and a fresh instance.
- Missing plug-in/version mismatch preserves state.
- 44.1/48/96 kHz and 64/128/256/512 blocks pass.
- Sample-rate and buffer-size changes while loaded re-prepare without losing state or crashing.
- No scan or load crash in the host.
- Project schema migration passes.

**Stop:** user audible and persistence review.

### M4 — Native editors

**Deliverables**

- Floating editor windows with `IPlugView`/`IPlugFrame`.
- DPI/resize/focus/multiple-editor handling.
- Generic parameter fallback.
- Window geometry persistence.

**Acceptance**

- Editors for TDR Nova, Ozone 8, Gateway, and ParametricOD open, resize, focus, and close without stopping audio.
- DPI tests pass at the available scales.
- DAW shortcuts do not steal text/parameter input.
- Multiple editors and project close are clean.
- User visual/interaction review.

**Stop:** editor UI review.

### M5 — VST3 instruments and live MIDI

**Deliverables**

- Instrument slot, event list generation, live MIDI routing to the selected/armed instrument track.
- Built-in oscillator fallback.
- Instrument state/editor integration.
- PDC for instrument latency.

**Acceptance**

- sforzando, Retrologue, and Superior Drummer 3 (stereo main) play from MIDI input and recorded parts.
- Note on/off and controller offsets are sample-accurate enough for the fixture.
- Live monitoring and recording work; latency is reported.
- Save/reopen restores instrument and state.
- sforzando, Retrologue, and Superior Drummer 3 editors open and operate through the same editor host.
- Multi-out is visibly deferred, not silently broken.

**Stop:** user instrument/live-MIDI review.

### M6 — Post-v1: automation, offline render, multi-out, sidechain, sandbox

Split into independently reviewable slices:

- **M6a:** parameter automation lanes and sample-accurate automation; offline render/bounce with pre-roll and tail.
- **M6b:** multi-out instrument routing and sidechain buses.
- **M6c:** optional process-isolated hosting/sandbox and crash recovery.

Each slice gets its own acceptance criteria. Do not bundle them into one unreviewable milestone.

### M7 — Format expansion decision

- Evaluate CLAP (MIT, simpler ABI, good sandbox story).
- Re-evaluate VST2/32-bit only if a legally reviewed need remains.
- Default expectation: no VST2.

## 9. Risks and open questions

### 9.1 Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| `vst3` crate lacks host-side coverage or changes incompatibly | Blocks the chosen ABI path | M0 probe; pin exact version; fallback to generated bindings outside the repo or a hand-written minimal subset |
| Plan-swap double-processing | Corrupt audio/state; CPU spikes | Stable runtime registry; no dual-plan crossfade for hosted tracks; structural fade |
| Per-sample path refactor regresses existing audio | Transport/recording/metering break | M2 is a dedicated milestone with full regression suite and review |
| In-process plug-in crash kills the DAW | Data loss | Safe mode, autosave, blacklist; optional sandbox M6; keep host boundary swappable |
| In-process plug-ins are fully trusted code; no security boundary | Malicious/buggy plug-in can access user data | Document the trust model; scanner is a crash boundary only; run unelevated; optional OS sandbox post-v1 |
| Editor HWND/message-pump problem in eframe/winit | Editors unusable | M0 editor spike; dedicated UI thread fallback |
| DPI/focus/modal dialogs | Poor editor UX | Real-plugin matrix, DPI tests, user review |
| Latency changes / wrong reported latency | Tracks misalign | Query after setup, handle `restartComponent`, PDC null tests, report latency |
| Large state (SD3/Ozone) | Slow save, huge project | Sidecars, progress UI, atomic writes, state size measurement |
| WaveShell/shell classes and duplicate paths | Duplicate/incorrect entries | Class-ID dedupe; shell enumeration; canonical path preference |
| Authorization dialogs during scan | Scan hangs | Hidden message pump, timeout, "needs authorization" status, blacklist |
| Sample-rate/buffer changes | Plug-in reset/clicks | Re-prepare under control thread, fade, state restore, test matrix |
| Thread-affinity/COM mistakes | Crashes/deadlocks | Controller/editor on UI thread; audio process on audio thread; queues; no COM in callback |
| CPU/memory with many instances | Poor performance | Benchmark, per-instance budget, bypass CPU measurement, later freeze |
| Legal/provenance | Distribution risk | No Steinberg files, `vst3` notice, license audit, counsel if clean-room ever considered |
| Universal compatibility expectation | User disappointment | Document supported scope; test matrix; no universal guarantee |

### 9.2 Open decisions

1. **In-process vs process-isolated hosting for v1.** Plan recommends in-process + out-of-process scanning. If crash isolation is a hard requirement, M0 must switch to a worker model before the graph refactor.
2. **State sidecar folder name and migration.** Plan proposes `Plugins/` in managed projects and `<project-stem>.plugins/` for older projects. Confirm during M0/M3.
3. **Editor thread model.** Main UI thread vs dedicated editor UI thread; M0 spike decides.
4. **Scanner process model.** Batch with restart vs one process per module; batch is proposed for speed, with crash attribution.
5. **Effects before instruments.** Proposed; confirm at plan approval.
6. **Preset support in v1.** Basic program/preset selection if cheap; full `.vstpreset` browser deferred.
7. **Multi-out v1 scope.** Proposed deferred; SD3 stereo main only.
8. **Automation v1 scope.** Parameter model/gestures only; lanes in M6a.
9. **`vst3` crate version/pin policy.** Pin exact; review updates deliberately.
10. **CLAP/VST2 later.** Separate plan.

### 9.3 Known unknowns

- Whether the current `vst3` crate exposes every host-side interface needed for editors, state, buses, and parameter gestures. The probe answers this.
- Whether eframe/winit dispatches messages for a foreign top-level HWND reliably across DPI changes and modal dialogs.
- Actual latency/CPU overhead of the chosen in-process design on the user's plug-ins and hardware.
- Actual state sizes and save times for Ozone 8 and Superior Drummer 3.
- Whether WaveShell children enumerate and load correctly through the generic host path.
- Whether all in-scope plug-ins tolerate 64-frame blocks, 96 kHz, and repeated activate/deactivate.
- How many users' plug-ins depend on VST2/32-bit; the current collection does not.

## 10. Proposed task sequence

Task IDs are proposals; do not create packets until the current T037/T038 review is resolved and this plan is approved.

| Proposed task | Scope | Depends on |
| --- | --- | --- |
| T039 | Apache-2.0 files, notices, `cargo-deny`, external `vst3` probe, state/editor decisions | Plan approval |
| T040 | Catalog, scanner helper, cache, preferences, plug-in manager | T039 |
| T041 | Block graph, runtime registry, structural fade, PDC | T040 |
| T042 | VST3 effects, insert picker, bypass, generic params | T041 |
| T043 | State sidecars, missing plug-in, project migration | T042 |
| T044 | Native editor windows | T043 |
| T045 | Instruments, events, live MIDI | T044 |
| T046 | Automation lanes and offline render | T045 |
| T047 | Multi-out, sidechain, optional sandbox | T046 |
| T048 | CLAP/VST2 decision | T047 |

Each task packet must name requirement IDs, code scope, architecture boundaries, acceptance checks, regressions, and essential gates. When the first packet becomes active, add a `vst3-hosting-v1` scope to `docs/workflow/ledger.json`, record source/build/runtime evidence, and run `python scripts/parity_report.py`. Do not reuse transport scope metrics for plug-in hosting.

## 11. Definition of done for v1

A user can:

1. Add VST3 folders in Preferences; scan them without crashing OpenTrack; see a searchable catalog with correct deduplication, shell children, architecture status, and failures.
2. Insert a VST3 effect on a track; hear it process; bypass it click-free; edit parameters; open its native editor; save and reopen the project with the same state.
3. Load a VST3 instrument; play it from live MIDI and recorded parts; record MIDI; save/reopen with state and editor.
4. Keep tracks aligned with automatic delay compensation and see reported latency.
5. Open a project with a missing or failed plug-in without losing the slot or state; retry, replace, or remove it.
6. Start in safe mode after a bad plug-in/scan and recover the project.
7. Run the acceptance test matrix at 44.1/48/96 kHz and 64/128/256/512-frame blocks with no host-side crashes or unreported dropouts.
8. Review the audible, editor, persistence, and recovery results. Compilation or source presence alone is not acceptance.

Universal VST3 compatibility, multi-out, sidechain, automation lanes, sandboxing, VST2, and 32-bit hosting are explicitly outside v1.

## 12. Change control

- Decisions D1-D14 are accepted only when the user records them as accepted. This plan is a proposal until then.
- Scope, format, licensing, or host-model changes require updating this document and the user's recorded intent.
- Do not start a milestone while T037/T038 or a prior milestone review is unresolved.
- Each milestone has its own acceptance evidence. Source presence or compilation is not acceptance.
- When implementation starts, create a separate `vst3-hosting-v1` ledger scope and task packets; do not edit the transport scope to make plug-in work look complete.
- Update `docs/audio-engine.md`, `docs/native-project-format.md`, `docs/architecture.md`, and `docs/third-party-notices.md` as implementation lands; do not claim features before runtime acceptance.
- Update `README.md` only after the user accepts the relevant workflow.
- The current transport ledger and metrics remain unchanged by this plan.

## 13. References

- Steinberg VST 3 Developer Portal, licensing: <https://steinbergmedia.github.io/vst3_dev_portal/pages/VST+3+Licensing/Index.html>
  - Since VST 3.8, the SDK is MIT-licensed; VSTGUI and mda files are BSD-like. This plan uses the `vst3` crate bindings, not vendored SDK files.
- `vst3` crate, MIT OR Apache-2.0: <https://crates.io/crates/vst3> and <https://docs.rs/vst3>
- Existing project evidence:
  - `docs/architecture.md`
  - `docs/audio-engine.md`
  - `docs/legacy-plugin-hosting.md`
  - `docs/native-project-format.md`
  - `docs/third-party-notices.md`
  - `docs/workflow/architecture-agreement.md`
- Code references:
  - `crates/opentrack-audio/src/lib.rs` (`PlaybackPlan`, `ChannelProcessor`, render path)
  - `crates/opentrack-app/src/app.rs` (`rebuild_audio_plan`)
  - `crates/opentrack-core/src/project.rs` (`InsertEffect`)
  - `crates/opentrack-app/src/channel_settings.rs` (insert slots)
- Local plug-in inventory evidence was collected read-only on 2026-10-25 from the user's `C:\Program Files\Common Files\VST3`, `C:\Program Files\Steinberg\VSTPlugins`, and `C:\Program Files (x86)\Steinberg\VSTPlugins`. It is a snapshot, not a universal compatibility claim.
