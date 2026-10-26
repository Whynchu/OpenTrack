# VST3 implementation status

Updated: 2026-10-25. This status is outside the transport parity ledger; do not
reuse transport metrics for plug-in hosting.

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
  state before saving, and exposes `VST3 Plug-in...` in the existing insert
  picker plus a parameter/editor window.
- TDR Nova 2.1.6 smoke test passed in the VST3 crate and app registry: 75
  parameters, 184-sample latency, 184-sample tail, native editor available,
  finite processed audio, 3,608-byte component state, and restore into a fresh
  instance. The app registry reloaded the saved state and enumerated the same
  parameters.
- WaveShell1 CLA-76 Stereo also loads through the same path with a live
  parameter list and finite block output.
- Current x64 local-ASIO build: `EFAA96C0E4BE778F70DAE29C5E7C45DBD45C9602BFE8E251C4730F259C403A58`.
  App suite: 136 passed, 2 ignored installed-plugin tests. VST3 suite: 13
  passed, 2 ignored installed-plugin tests. Hosted audio: 6 passed. Live
  listening/visual review remains pending; the full audio suite still has
  unrelated in-progress pitch-quality failures owned by Codex.

## Not implemented

- Hosted VST3 instruments/MIDI routing, multi-out, sidechain, surround.
- Offline bounce/tail handling, automation lanes, true sandboxing.
- Per-module scanner timeout/quarantine/job object, cache persistence/UI.
- Universal compatibility. Current evidence is one installed folder.

## Tests

`cargo test --workspace` passes: app 131, audio 63 (1 ignored manual comparison),
core 17, scanner 16, catalog 11, VST3 3 (1 ignored installed-module probe).

## Next

1. Add per-module scanner isolation with timeout/crash attribution and job-object
   containment.
2. Instantiate each class in the scanner for buses, parameters, latency, tail,
   and editor availability; reject non-stereo-main layouts explicitly.
3. Persist the catalog/cache and expose a plug-in manager.
4. Replace the per-sample channel path with a block graph and PDC before any
   in-process VST audio processing.
