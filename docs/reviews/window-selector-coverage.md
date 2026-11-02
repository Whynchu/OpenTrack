# Window and selection source coverage

Snapshot: HEAD `1b61c7a` plus existing modified `app.rs`, `channel_settings.rs`, `vst3_host.rs`, workflow current and T066/T070. Source-only review by GPT-6.1 Terra with GPT-6 Luna Low audit passes; no runtime review. Status `unknown` means the control family has been located but each option/consumer has not yet been verified. This is a coverage ledger, not a claim of complete item-level coverage.

| ID | Window/dialog or selection family | Entry point / consumer | Source status and limits |
|---|---|---|---|
| W01 | Main shell / Project / Mixer | `main.rs:24`, `app.rs:8631,9839` | Partial audit: views and project actions exist; each control not enumerated. |
| W02 | Transport / metronome / tempo | `app.rs:9025`, `transport_panel.rs:653,706,732` | Source connected: transport and signature selector; individual controls require inspection. |
| W03 | Channel Settings / Setup | `app.rs:9209,17136`, `channel_settings.rs:2763` | Partial: insert routing and studio sends unavailable; sends lack DSP. |
| W04 | MediaBay / browser | `app.rs:9704` | Partial: browser menu choices share Media Pool; individual controls require inspection. |
| W05 | MIDI editor / Sample Editor | `app.rs:15215`, `sample_editor.rs:413` | Partial: Snap checkbox no consumer; Split at Selection disabled; sample selectors source-connected. |
| W06 | Preferences | `preferences.rs:175-226` | Partial: metering settings connected; remaining pages planned; preset buttons and direct monitoring disabled. |
| W07 | Device Setup / Project Setup / VST Connections | `audio_configuration.rs:41,572,668` | Partial: device/rate selectors source-connected; Add Bus/presets disabled and several connection tabs unavailable. Dynamic devices vary. |
| W08 | Plug-in Information / native and generic editors | `audio_configuration.rs:1044`, `app.rs:4509,4616`, `vst3_host.rs` | Partial audit: scan/editor paths exist; per-plug-in behavior and controls require runtime and compatibility review. |
| W09 | Add Track | `app.rs:10243-10406` | Partial: count is bounded by remaining project capacity; audio/group mono/stereo selection and mono Left/Right input channel are connected. Speaker layouts, instrument selection, and Browse Presets are explicitly unavailable; accepted Instrument tracks are created empty. |
| W10 | Key Commands / assignment / conflict / Workspace Organizer | `app.rs:6935,7119,7178,7237` | Window creation and controls located; full command and per-control trace unknown. |
| W11 | File/folder pickers and project folder setup | `app.rs:2693,2703,2727,2881,4282`, `audio_configuration.rs:1268` | Source paths located; project lifecycle and individual failure conditions need separate tracing. |
| W12 | Track colors, rename, remove | `app.rs:4753,4846,4929`, `project_controls.rs:57` | Source paths located; remove gating and confirmation are conditional. |
| W13 | Open error, loading, audio file operation, project rate | `app.rs:3647,3944`, `media_pool.rs:150`, `project_rate.rs:80` | Conditional dialogs located; individual action outcomes unknown. |
| W14 | About / realtime performance / splash | `app.rs:8609,11116`, `startup_splash.rs:313` | Source entry points located; per-control trace unknown. |
| W15 | Disabled window destinations | `app.rs:8397-8456` | Control Room, MIDI Device Manager and other legacy windows disabled; product scope is undecided. |
| S01 | Audio backend, WASAPI mode, input/output, sample rate/buffer, MIDI input | `audio_configuration.rs:163-556`, `app.rs:8132` | Source-connected; dynamically enumerated devices, rates and buffers. Hardware behavior unknown. |
| S02 | Project recording rate/encoding | `audio_configuration.rs:585,602`, `project_rate.rs:80` | Source-connected; rate change has confirmation. Runtime/persistence unverified. |
| S03 | Analyzer stage/reference, tonal duration/reference, spectrum range/tilt/resolution | `analyzer_view.rs:483-679` | Source-connected display/analysis settings. Runtime behavior unverified. |
| S04 | Project grid and quantize | `app.rs:1407,1436` | Popup families located; each choice and persistence require tracing. |
| S05 | MIDI position/length/controller | `app.rs:15215,15223,15494` | Source-connected editing parameters; Snap checkbox is a local nonpersistent value. |
| S06 | Sample Editor timebase/grid/quantize | `sample_editor.rs:413-461` | Source-connected display and editing state; runtime unverified. |
| S07 | Key-command category / workspace shortcut | `app.rs:6942,7290` | Combo boxes located; full per-option mapping unknown. |
| S08 | Track color / track selection / inspector | `project_controls.rs:57,430`, `project_inspector.rs:296`, `app.rs:10703` | Popup families located; individual choices, undo and persistence require tracing. |
| S09 | Channel selection / inserts / sends | `channel_settings.rs:125`, `mixer.rs:1177,1214`, `app.rs:17136,17484` | Partial: plug-in catalog dynamic; sends DSP unavailable; inspect individual choices and rack state. |
| S10 | Project/event/ruler/transport context menus | `project_controls.rs:189,207`, `app.rs:11081,12747,15716`, `transport_panel.rs:177` | Families located; menu/command pass still must enumerate each entry. |
| S11 | Add Track choices | `app.rs:10287-10371` | Mono/stereo and mono input Left/Right choices are source-connected; speaker layouts, instrument selection, and Browse Presets visibly unavailable and not applied. |

## Remaining work

The counts above are coverage families, not counts of all controls. Complete individual item registers, examine custom-painted selectors and dynamically generated menus, and trace their actions, persistent state and processing consumers. The earlier menu and keyboard-command passes are also still incomplete. All runtime, audio, plug-in, OS and hardware verification belongs to the later running-app audit. See [summary inventory](ui-implementation-inventory.md).

## Step 3–4 detailed registers

- [Windows and dialogs](audit-windows-app.md): entry points, conditions, actions and explicit unknowns; reconciled with T071 at HEAD `7b898c68`.
- [ComboBox selectors](audit-comboboxes.md): 30 source declarations, options, consumers and persistence where traced. The inspected selector source predates T071; the only selector-free application changes since were transport meter additions.
- [Custom selectors and popups](audit-custom-selectors.md): custom selection families and dynamic sources. The audited selector owners were unchanged through T071.
- [Menus and context menus](audit-menus.md): current HEAD, open/empty shell routes, active/conditional/disabled items and dynamic families.
- [Keyboard commands](audit-keyboard-commands.md): all 152 reference keymap rows, 158 binding expressions and 198 expanded shortcut bindings at current HEAD.

## Reconciled source findings

- Keyboard command registry: 45 implemented dispatch actions, 7 partial, 100 generic fallback. The menu register is separate because menu actions are largely direct UI callbacks, not registered IDs.
- The menu register enumerates menu leaves by explicit path and generated family; runtime-discovered device/plug-in items are inventoried by generator, not by machine-specific contents.
- Windows and selector findings remain source-only. Disabled placeholders, conditional state gates, and documented exclusions are distinguished in the detailed registers.
- The source inventory does not claim running-app, audio, hardware or user acceptance. Native file pickers, dynamic catalogs, and some individual control behaviors are recorded as unknown where code tracing is insufficient.

These reports classify source evidence only. A named window, selector or dispatch branch is not evidence of successful runtime execution, audible output, device compatibility or user acceptance.
