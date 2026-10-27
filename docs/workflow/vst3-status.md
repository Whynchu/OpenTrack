# VST3 implementation status

Updated: 2026-10-26. This status is outside the transport parity ledger; do not
reuse transport metrics for plug-in hosting.

## Latest verified correction — T044

Native plugin editors are now the default on insert add/click; generic sliders
are a fallback with an explicit error. Fixed native mono bus negotiation for
Waves, factory/module teardown order on load errors, optional legacy processing
notifications for iZotope, initial controller state sync, editor close ordering
and host frame sizing.

The blank/stalled Waves editor was fixed by hosting the native view on a
dedicated STA thread with its own message loop. The view is created from the live
audio effect's controller, not a second instance, so CLA-76 VU/GR meters and
other processing displays follow the real audio callback. A continuous-tone
probe showed the meter moving and controls responding. All 25 installed effects
(including all 19 Waves classes) had previously passed the fresh-folder native
lifecycle/process/state sweep; that sweep was not repeated after the thread
change. Three instruments still require instrument/MIDI hosting. See
[T044](tasks/T044-vst3-native-editors-and-compatibility.md) for current build
identity, exact checks and pending visual/listening acceptance. Current x64
local-ASIO build: `84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`.
The historical implementation notes below are not evidence of current parity.

## Implemented

- `opentrack-plugins`: format-neutral class IDs, descriptors, catalog/cache
  schema, scan statuses. Committed `81227f5`.
- `opentrack-plugin-scan`: bounded folder discovery, bundle selection, PE
  architecture precheck, path dedupe, JSON/text reports. Committed `81227f5`.
  Report protocol is now 2 with class enumeration fields.
- `opentrack-vst3`: x64 module loading using
  `LoadLibraryExW`/`LOAD_WITH_ALTERED_SEARCH_PATH`, optional `InitDll`/`ExitDll`,
  a multithreaded COM apartment, factory/class metadata, and a control-thread
  host skeleton (`IHostApplication`, `IMessage`, `IAttributeList`).
- `opentrack-plugin-scan --enumerate`: loads each x64 module inside the scanner
  process and emits class descriptors. The scanner is a separate process, but
  per-module timeout and crash attribution are still pending.

## Runtime evidence

Command, run from `G:\dev\Opentrack\OpenTrack-` with OpenTrack closed:

```powershell
$out = (Resolve-Path 'builds\x64').Path + '\vst3-enumeration.json'
.\.build-cache\debug\opentrack-plugin-scan.exe --enumerate `
  --no-default-folders --folder "C:\Program Files\Common Files\VST3" `
  --json --output $out
```

Result: `modules=11`, `enumerated_modules=11`, `plugins=28`, `rejected=0`.
WaveShell1-VST3 15.0_x64 exposed 19 child classes. Other classes included
Gateway, NeuralAmpModeler, ParametricOD, Retrologue, Superior Drummer 3,
TDR Nova, Ozone 8, Tonal Balance Control 2, and sforzando.

The WaveShell probe required `InitDll` before `GetPluginFactory`; without it the
scanner process crashed. The loader now performs that handshake. Moving the
output to an absolute path was also required because a loaded plug-in changed the
process working directory.

## T043 insert hosting (in progress)

- `InsertEffectKind::Vst3` stores class ID, module hint, display names and
  opaque component/controller state in the existing insert slots.
- `Vst3Effect` now creates the component/controller, negotiates stereo main
  buses, prepares and processes blocks, drains parameter changes, saves/restores
  state, exposes generic parameters and owns a native editor view.
- `opentrack-audio` accepts `HostedInsert`/`HostedProcessor`, processes hosted
  plans in 64-frame blocks and applies per-track PDC. Plans without VSTs retain
  the original per-sample path.
- The app registry keeps instances alive across playback-plan rebuilds, captures
  state before saving, and exposes scanned VST3s in the normal insert category
  menus plus a parameter/editor window.
- `Devices > Plug-in Information` now manages VST3 folders, scans in a
  background thread, persists folders/catalog, shows failed modules, and feeds
  the insert library. There is no separate VST3 effect picker.
- TDR Nova 2.1.6 smoke test passed in the VST3 crate and app registry: 75
  parameters, 184-sample latency, 184-sample tail, native editor available,
  finite processed audio, 3,608-byte component state, and restore into a fresh
  instance. The app registry reloaded the saved state and enumerated the same
  parameters.
- WaveShell1 CLA-76 Stereo also loads through the same path with a live
  parameter list and finite block output.
- Current x64 local-ASIO build: `84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`.
  App suite: 133 passed, 8 ignored opt-in tests, six pre-existing pitch-editing
  failures. VST3 suite, scanner/catalog suites, 6 hosted-audio checks and the
  installed CLA-76 mono/stereo audio checks pass. Native CLA-76 editor and
  dedicated-thread regressions pass. Live in-app listening/visual review remains
  pending.

## Not implemented

- Hosted VST3 instruments/MIDI routing, multi-out, sidechain, surround.
- Offline bounce/tail handling, automation lanes, true sandboxing.
- Per-module scanner timeout/quarantine/job object, cache persistence/UI.
- Universal compatibility. Current evidence is one installed folder.

## Tests

Current targeted checks: app 133 passed (8 opt-in ignored) with six separate
pitch-editing failures; core 18 passed; scanner and catalog suites pass; VST3
suite passes with installed probes ignored; hosted audio 6 passed; installed
CLA-76 mono/stereo audio and dedicated-editor checks pass. The historical
workspace totals below are superseded.

## Next

1. Add per-module scanner isolation with timeout/crash attribution and job-object
   containment.
2. Instantiate each class in the scanner for buses, parameters, latency, tail,
   and editor availability; reject non-stereo-main layouts explicitly.
3. Persist the catalog/cache and expose a plug-in manager.
4. Replace the per-sample channel path with a block graph and PDC before any
   in-process VST audio processing.
