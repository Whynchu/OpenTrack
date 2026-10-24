# VST Audio Channel Settings study

**Status: partial investigation; implementation and runtime parity are not accepted.** This is a bounded M2 reference pass for the Project-attached VST Audio Channel Settings window. The Compressor editor remains out of scope.

## Reference conditions

- Reference: Cubase 5, project `cubase file.cpr`, selected `Track 01` (audio channel).
- Desktop capture: 1920×1033, Windows dark appearance. Project, Mixer, and Transport windows were visible behind the floating Channel Settings window. Display scaling was not independently measured.
- Input gain was `0.0`, pan was centered (`C`), and EQ values were already non-neutral. Earlier notes say Record Enable was off, but the current Cubase screenshot shows its red active indicator. No Record Enable click was made, so its baseline is marked unknown. Mute, Solo, Listen, Read, Write, and Monitor appeared off.
- Cubase was inspected without saving. A temporary pan test moved the value to `L68`; inline editing restored and committed `C`. EQ band 1 was bypassed once to inspect its control and immediately re-enabled; no EQ values were edited. No other project values were intentionally changed.
- Evidence: `../_analysis-tools/reports/opentrack-comparisons/Project-Channel-study-2026-10-24/`, including earlier `57` input-gain hover, `73` baseline, `74` Channel Level hover, `76` selector, `77` restored state, and `81`-`85` click/menu captures. The additional EQ1 bypass was observed in a live screenshot but was not saved as a separate comparison file. Existing manifests record the main capture conditions.

## Observed structure and interactions

| Area | Observed behavior | Evidence level |
| --- | --- | --- |
| Channel selector | Cubase lists `Track 01`, `Audio 02`, `Prologue 01`, `Stereo In`, and `Stereo Out`; the current channel is checked. | Observed; channel switching not tested |
| Channel name | Hover says `Channel Name (Use double-click to edit)`. Double-click enters inline editing and selects the name; Escape cancels. | Observed |
| Input routing | Left-click opens `No Bus` and a `Stereo In` submenu with `Left - Stereo In` / `Right - Stereo In`. | Observed |
| Output routing | Left-click opens `No Bus`, `Stereo Out`, `Left`, and `Right`. | Observed |
| Pan | Left-clicking the bar did not change the centered value. Dragging changed it continuously and updated the Project Inspector and Mixer readouts. Double-clicking the pan value opens an editable text field; entering `C` and pressing Enter commits center. Right-click opens the shared Channel Settings menu. | Observed; exact drag scale and modifier behavior remain unmeasured |
| Input gain | Hover says `Input Gain (use shift)`. The control is a knob beside a numeric value. | Observed; modifier amount and reset gesture not remeasured in this pass |
| Input phase | Tooltip is `Input Phase`; a click toggles the icon and the Mixer indicator, and restoring the control returns both to baseline. Right-click opens the shared menu. | Observed |
| Channel level | Hovering the fader value shows `Channel Level`. | Observed |
| EQ graph and bands | Right-clicking a graph point showed no menu. Left-clicking the band type opens a filter-shape menu; for band 1 it lists Parametric I, Low Shelf I, High Pass I, High Pass II, Parametric II, Low Shelf II, Low Shelf III, and Low Shelf IV (current). Clicking the EQ1 power icon bypasses that band and redraws the curve while leaving its values visible; clicking it again restored the band. Right-clicking the band label opened the shared Channel Settings menu. Right-clicking the 58.0 Hz field focused/selected its text instead of opening a context menu. A double-click attempt on that value was inconclusive: the Channel Settings window disappeared from view, and no value change was confirmed. | Observed for band bypass, shape menu, and right-click behavior; graph dragging, reliable numeric editing, and reset gestures remain untested |
| Insert slots | Left-clicking an empty slot opens an effect catalog: No Effect, Delay, Distortion, Dynamics, EQ, Filter, Mastering, Modulation, Other, Pitch Shift - Pitch Correct, Restoration, Reverb, Spatial, Surround, SurroundDither, Tools, and Vocoder. Sampled nested lists: Delay has DoubleDelay, ModDelay, ModMachine, MonoDelay, PingPongDelay, StereoDelay; EQ has GEQ-10, GEQ-30, StudioEQ; Dynamics has Compressor, DeEsser, EnvelopeShaper, Expander, Gate, Limiter, Maximizer, MIDI Gate, MultibandCompressor, VintageCompressor, VSTDynamics. No effect was loaded. | Observed; only these submenus sampled; availability depends on installed plugins |
| Send slots | Left-clicking an empty Send slot opens No Bus or Outputs, then Stereo Out, Left, or Right. No bus was assigned. | Observed |
| Shared context menu | Right-clicking channel title, routing, input gain, phase, pan, fader, EQ band label, and empty Insert/Send slots opens the same menu: Customize View >, checked Always on Top, Save Selected Channels, Load Selected Channels, Save All Mixer Settings, and Load All Mixer Settings. EQ numeric fields focus text instead; an EQ curve point opens nothing. | Observed |
| Customize View | The menu has visibility/layout choices for Inserts, Insert Routing, Equalizers, Equalizers + Curve, Equalizers + Curve + Dials, Sends, Send Routing, Studio Sends, Studio Sends Routing, Control Strip, and Fader, plus Show All, Default, Fader Only, and Setup. Setup shows Visible Items and Hidden Items lists, move/reorder buttons, a Presets selector, save and delete icons, and OK. It was closed without changes. | Observed |

### Hover text confirmed

Exact observed labels include `Mute`, `Solo State (Solo Defeat with ALT click)`, `Listen`, `Read Automation`, `Write Automation`, `Monitor`, `Record Enable`, `Input Phase`, `Input Gain (use shift)`, `Channel Level`, and `Channel Name (Use double-click to edit)`. Earlier section-state hovers read `Inserts State (Bypass Inserts with click / Context-menu shows usage)`, `EQs State (Bypass EQs with click)`, and `Sends State (Bypass Sends with click / Context-menu shows usage)`. The ALT solo-defeat gesture and individual automation controls were advertised by tooltip but were not activated during this pass.

## OpenTrack status

| Capability | Current status |
| --- | --- |
| Channel strip layout | Project source adds a channel-name dropdown above input/output routing, the phase/gain/pan controls, Mute/Solo/Listen/Read/Write row, Monitor and Record Enable controls, and fader. The accepted layout, shared fader renderer, channel switching, and channel-name select-all/Escape behavior were checked at runtime by the root agent; committing the rename was not tested. The new tooltip changes in this pass remain unreviewed. |
| Pan interaction | Updated to change only while dragging; a plain click no longer changes pan. The Cubase double-click text editor for the pan value is not implemented and remains a known gap. |
| Gain interaction | The new Channel Settings knob uses per-frame pointer movement and the observed Shift hint. The per-track gain field has no confirmed audio-engine application. |
| Tooltip discipline | Source now maps the recorded `t4-033`–`t4-081` Channel Settings hovers and the recorded section-state hints to the Inserts header indicator, header/rack state icons, Insert 1–8 power controls, loaded/empty Insert expand controls, EQ band labels/power/value fields, Send 1–8 power controls, input/output routing, input gain value/knob, strip buttons, and fader value. Empty Send bodies and EQ filter-type labels have no added tooltips because no text was observed for those targets. Runtime hover review of these additions is pending. |
| Selection | The new in-pane selector lists project tracks; the runtime selection change and title synchronization still need verification. `Stereo In` / `Stereo Out` are not channel-selector targets. |
| Context/view management | Source implements the shared right-click menu with a popup ID separate from the widget's left-click dropdown, consistent popup spacing/width across Channel Settings menus while retaining OpenTrack's current theme colors, Always on Top, selected-channel and all-mixer JSON save/load, Customize View variants, a reorderable Visible/Hidden Setup dialog, and user view presets. Runtime checks confirmed the shared context menus, channel switching, and rename select-all/Escape behavior; rename commit remains untested. Setup list/reorder/preset behavior is source-only and has not been runtime reviewed. |
| Inserts and Sends | The Insert picker exposes the observed category structure and sampled Delay/EQ/Dynamics plug-in names. StereoDelay and Compressor map to existing processors; Test Generator remains explicitly under OpenTrack Effects. Unimplemented Cubase plug-ins are disabled. Send destinations follow the observed No Bus/Outputs choices. Insert-rack bypass reaches the audio playback processor. Insert 1–8 power and empty expand controls now carry the corresponding activate/deactivate tooltip; the loaded slot's narrow type control opens the picker while the main Compressor body still opens the existing editor. Insert/Send/EQ state icons reflect active content. Send 1–8 have UI-only activation flags and recorded Activate tooltips; these flags do not route or process audio. The Compressor editor is preserved. |
| EQ | Every band's filter-type field now opens a selector. Band 1 uses the exact eight options recorded in the reference; bands 2 and 3 expose only Parametric I/II, consistent with project validation and the manual's statement that those bands are parametric. The manual identifies EQ4 as supporting parametric, shelving, high-pass, and low-pass families. The restored reference state shows `High Shelf II`; the generic model HighShelf value is displayed with that observed label, without adding unsupported shelf variants. EQ4 offers currently modeled members of those families; its exact Cubase menu inventory remains unknown. Root runtime checks confirmed graph dragging, band activation, band bypass, and channel-wide bypass. The user pointed out that the small bars below Gain/Freq/Q should represent their values and be directly clickable/draggable. Source now draws value-positioned rails, centered for signed gain and logarithmic for frequency/Q; band/global bypass controls their accent tint. The drag mapping and visual state are implementation choices pending live review. The `t4-044`–`t4-060` Active/Gain/Freq/Q tooltip additions are also source-only and remain unreviewed. Filter variants still use generic OpenTrack filter families; measured Cubase responses are not established. |
| Audio behavior | Insert-rack bypass is connected to playback. Per-track input gain/phase, Monitor, Listen, automation Read/Write, Send routing, Send activation, and Sends bypass remain model/UI state without established per-track signal-path parity. The current audio monitor remains app-level. |
| Visual/runtime parity | The accepted baseline layout and the interactions listed above were checked at runtime. This pass's tooltip targets and four-band type selectors, plus the Setup list/reorder/preset controls, still need direct hover/click review. Compilation alone does not establish parity. |

### User runtime report — 2026-10-24

The user's screenshot `Screenshot 2026-10-24 081731.png` shows egui duplicate-widget-ID warnings layered over the Channel Settings right-click menu. The user reports right-click behavior differs across targets; Always on Top did work. Source inspection found that `Response::context_menu` and left-click `menu_button` popups use the same default popup ID for a shared response. The context menu now uses a separate stable popup ID. The user prefers OpenTrack's existing menu colors; current popup colors are retained while Channel Settings menu spacing and width are aligned. The fader now uses the Mixer’s shared renderer and meter level. These source changes have been rebuilt; direct visual and interaction review remains pending.

### Additional runtime verification by root agent — 2026-10-24

The user accepted the current Mixer and Channel Settings rendering before this tooltip/selector pass. Root manually rechecked EQ graph dragging, band activation and bypass, channel-wide bypass, routing/insert/send/fader context menus, numeric right-click text focus, channel switching, and channel-name select-all/Escape behavior. Rename commit remains untested. The latest tooltip placements, every band's type menu, and the Setup list/reorder/preset controls were not opened in the UI during this pass.

### Source evidence for tooltip and EQ selector pass — 2026-10-24

- Tooltip strings and targets follow `docs/reviews/tooltip-sweep-log.md` rows `t4-033`–`t4-081`. In particular, empty Send bodies had no tooltip; only their power controls advertised activation. The Send activation set is transient Channel Settings UI state and has no audio or project persistence.
- The local Cubase 5 Operation Manual, printed page 124, describes the four EQ modules as sharing the 20 Hz–20 kHz frequency range and ±24 dB gain range; it says EQ1/EQ4 may use parametric, shelving, or high/low-pass types, while EQ2/EQ3 remain parametric. It does not enumerate EQ4's popup entries.
- EQ1's eight menu entries are directly observed in the channel-settings reference notes. Capture 77 shows `High Shelf II` as the restored EQ4 type. EQ2/EQ3's Parametric I/II choices are restricted by the current project validator. EQ4's selector uses the corresponding supported model variants, without claiming an exact Cubase menu match or inventing additional high-shelf variants.

## Remaining unknowns

- Whether ALT-click on Solo sets Solo Defeat and how it is cleared.
- Input gain and pan gestures, fader precision, and the selector contents/hover text added in this pass still need runtime review. The root agent verified EQ graph dragging and numeric value editing; the earlier double-click on the 58.0 Hz value remains inconclusive as a Cubase behavior.
- The behavior of whole-section EQ/Sends bypass controls beyond their observed tooltips, and whether section bypass is independent of per-band EQ power buttons.
- How channel views and saved settings persist, including scope when multiple channels are selected.
- The full Insert catalog outside the sampled Delay, EQ, and Dynamics submenus; third-party availability may vary by Cubase installation.
- Exact filter-menu inventory for EQ4, EQ dial scaling/anchor behavior, and whether the modeled EQ shapes match Cubase response curves.
- Runtime behavior of the latest tooltip placements, four EQ type selectors, Setup list/reorder/presets, Always on Top, and `.otchannel` / `.otmixer` save/load actions.
- Direct interaction with the new Gain/Freq/Q parameter bars, including whether their fill and click/drag mapping match the reference. The user reports that Cubase's bars are filled according to their setting/status and are normally clickable/draggable; exact transfer curves were not measured.
- Sends bypass DSP and per-track Send signal flow; no Send processor exists yet.
- Whether the red Record Enable indicator is an intentional armed state or a mismatch with the earlier notes; it was not clicked or changed.
- Which buses and channel types are included in the selector for other projects, and the behavior when switching a Channel Settings window between tracks and buses.
- Audio signal processing and hardware-monitor routing for these per-track fields.

## Next acceptance slice

The latest x64 executable is `builds/x64/OpenTrack.exe` (SHA-256 `C3F4AA02721B7035A29D79322CC44F048565AE4CFD8A0D26DD2872D1651DFAFB`); it compiled after the value-positioned EQ rails were added. OpenTrack remains closed. Runtime review still needed for the tooltip placements, all four EQ type selectors, Gain/Freq/Q bar interactions, UI-only Send activation controls, and Setup list/reorder/preset controls. The user accepted the baseline Mixer and Channel Settings rendering; the Compressor editor remains unchanged. Project-save and round-trip checks remain pending.
