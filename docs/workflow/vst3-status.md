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

## Per-module scan isolation (2026-11-01)

Scanning loads arbitrary native code, so each module's factory enumeration now
runs in its own child process of the scanner (`--enumerate-module PATH --output
FILE`) under a bounded budget (`--module-timeout SECS`, default 30, range 1-600):

- A module that exceeds its budget is killed and reported as `timed_out` with
  "module did not finish enumerating within N s and was terminated".
- A module that kills its worker is reported as `crashed`, with the exit code,
  signal (Unix) and the worker's stderr attached.
- A module that loads but refuses to enumerate is reported as `load_failed` with
  its own reason. The worker writes an explicit `Failed` result and exits
  successfully, so "the module said no" is never confused with "the worker died".
- The scan continues through the remaining modules; nothing is skipped silently.
- `Devices > Plug-in Information` lists every rejected module from the last scan
  with its status label, path and reason, so a timeout or crash is visible
  without reading a log.

The worker ships beside the application (`opentrack-plugin-scan` /
`opentrack-plugin-scan.exe`), and the app resolves it from its own directory. When
the worker is missing the scan still runs, in-process, and the report note and the
Plug-in Information status say so instead of implying isolation.

Unit tests drive the isolation directly with stand-in programs (a hanging
process, an abnormal exit, a clean exit without a result, a result written from
the worker side). Integration tests run the real binary: the worker protocol for
a refused module, and a full isolated scan that attributes a module which the
precheck accepted but the worker could not load.

Limits: the worker is the scanner binary, not a sandbox — a module that corrupts
shared state (for example the process working directory or a global hook) still
affects its own worker process only, because that worker exits afterwards. Audio
hosting still runs in-process in the application, so this does not protect
playback. Windows job-object containment is still pending.

## Failure isolation (2026-11-01)

A plug-in that cannot load must not take the project with it. Current behavior,
each step covered by a test where it is testable without a plug-in:

- `Runtime::sync` records a failure per `(channel, slot)` and remembers the
  failed class, so one broken insert is not re-instantiated on every plan rebuild
  (that quarantine used to exist only in the runtime; it is now surfaced).
- A failed state restore keeps the instance running and records the reason, so a
  plug-in that loads but cannot take its saved state still passes audio.
- The Channel Settings insert row marks the failed plug-in with a red `!` at the
  name, shows the recorded reason on hover, and retries on click
  (`retry_insert` → `Runtime::retry` → plan rebuild). The parameter window keeps
  its own Retry Load button.
- Opening a project with failures appends "N VST3 insert(s) could not be loaded
  (marked ! in Channel Settings)" to the status line instead of failing the open.
- Hosted processing errors are reported separately by the engine
  ("N VST3 processing error(s); the channel fell back to dry audio").

Known limits: this is isolation, not sandboxing. A plug-in that crashes the
process still takes OpenTrack with it; scanning has no per-module timeout or
crash attribution yet. Windows native editor hosting is established by the
Windows-specific compatibility evidence. Linux source now supports the
X11/XWayland `X11EmbedWindowID` path and Linux `IRunLoop`; native Wayland-only
embedding remains unsupported. DISTRHO MVerb from DPF VST3 1.7 passed an
XWayland lifecycle probe covering paint, focus, pointer parameter editing,
resize, processing/state, and close/reopen. This is one plug-in only; full app
user review, native-Wayland-only behavior, and broader compatibility remain open.
The no-X11 error response now has a focused regression test, but a native
Wayland VST3 host requires a separate compositor connection for the plug-in
and a parent-surface/subsurface bridge; that runtime path is not implemented.
An attempted full-app review on XWayland reached the main window, but KWin's
Remote Control confirmation blocked automated input before the example
project/insert workflow could be reviewed. The no-project window was closed
through a window-manager close request.
See [the Linux editor task](tasks/LINUX-VST3-editor-hosting.md) for exact evidence.

## Not implemented

- Hosted VST3 instruments/MIDI routing, multi-out, sidechain, surround.
- Offline bounce/tail handling, automation lanes, true sandboxing.
- Windows job-object containment for scanner workers, persistent quarantine list
  (a module that crashed is rejected for that scan only).
- Process-level sandboxing for hosted plug-ins (isolation today is per-insert
  quarantine inside the app process).
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
