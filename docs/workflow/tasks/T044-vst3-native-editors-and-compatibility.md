# T044 — Native VST3 editors and installed-effect compatibility

Date: 2026-10-26. Implementer: Codex (GPT-6.1 Sol) with OpenCode continuation (DeepSeek V4.1 Flash); no independent agent review. Status: source and x64 ASIO build complete; controls and live-meter probe verified, final in-app meter/save-reopen acceptance pending.

## User requirement and acceptance established before fixes

Gateway must open its own graphics from an insert, rather than the generic host
sliders. Waves insert creation must stop crashing. Exercise the installed
`C:\Program Files\Common Files\VST3` inventory, distinguish effects from
instruments, and preserve the existing Rust audio path and ASIO support.

Acceptance checks: native editor first on insert add/click; correct native client
size; safe close/reopen/replacement; real installed classes load/process/save/
restore with overlapping instances; failures attributed per class in bounded test
processes. Visual identity, real device playback and user acceptance require live
review and are not inferred from a successful ABI call.

## Reproduced failures and changes

- Baseline: all nine installed Waves mono classes crash in `setBusArrangements`
  when coerced to stereo. Their stereo variants pass. Negotiate each plugin's
  native mono/stereo main buses; downmix host stereo input for mono processors
  and duplicate a mono output into both host channels. Mono/stereo variants stay
  distinct selectable classes.
- Ozone 8 and Tonal Balance Control 2 return `kNotImplemented` from the
  processing-state notification. Accept that legacy no-op. Before this fix the
  resulting preparation error also exposed a host teardown crash: a cloned
  local factory reference was released after its module unloaded. A Windows
  debug exception stack resolved to `FUnknown_release` / factory `ComPtr` drop.
  Borrow the module's factory so no independent local reference survives an
  error that drops the partially initialized effect.
- Sync initial component state into a separate controller before preparation.
  Probe editor availability after preparation rather than before it.
- Adding/clicking an insert opens its native editor directly. Generic parameters
  remain an explicit fallback when the native editor cannot open; the status
  message records the error. Repeated clicks focus the existing editor.
- Detach the previous native view before switching parent windows, replacing or
  removing its instance, or changing the audio format. Window close hides the
  frame until the UI thread calls `removed`; the child HWND remains alive during
  detach. The installed regression verifies this ordering with `WM_CLOSE`.
- Match the enclosing host frame to the plugin's requested client dimensions,
  including size changes during attachment and `IPlugFrame::resizeView`.
- Poll native editor dirty notifications so edits mark the project modified;
  existing save captures component/controller state.

No new dependencies. Windows HWND/COM calls remain in the existing VST3/app
platform boundaries. No plugin assets/binaries were copied or redistributed.

## Dedicated native editor thread (2026-10-26 continuation)

The full application still reproduced a white Waves editor that eventually
painted but did not respond to control input. Isolated eframe probes rendered
normally with the same plug-in, window tree, audio processing and ASIO setup, so
the remaining fault was the editor sharing OpenTrack's main message/paint
cadence. A second plug-in instance fixed input but left the CLA-76 meter dead
because it was not the processor receiving audio.

The final path:

- `Vst3Effect::open_detached_editor` creates a view from the live audio effect's
  controller on a dedicated STA thread.
- `EditorHost` owns that thread, its window message pump, focus calls and a
  60 Hz repaint request. `DetachedEditor` closes the view on the same thread.
- The editor window is an unowned tool window. Passing a main-thread owner across
  the editor thread deadlocked window creation.
- `IPlugView::onFocus(true/false)` is sent around attach/detach.
- Parameter edits continue through the effect's host services, and track-wide
  insert bypass still applies.

A continuous-tone probe showed the CLA-76 VU/GR meter moving and the controls
responding. The user confirmed the controls work in the application; final
in-app meter, save/reopen and long-run review remain pending.

## Verification

- `cargo test -p opentrack`: 133 passed, 8 opt-in tests ignored, six
  pre-existing pitch-editing failures owned by the separate pitch track.
- `cargo test -p opentrack-vst3 -p opentrack-plugins -p opentrack-plugin-scan`:
  VST3, catalog and scanner suites pass; installed probes remain opt-in.
- `cargo test -p opentrack-audio hosted`: 6 passed.
- `python scripts/test_vst3_installed.py`: fresh enumeration of 11 modules / 28
  classes. All 25 effects passed native editor attach/message pumping,
  overlapping instances, finite processed audio, state save/restore,
  reopen, and Windows close ordering. All 19 Waves classes passed, including
  the nine mono variants that crashed in the baseline.
- Full sweep output with per-class logs, source SHA-256 values and test executable
  SHA-256: `builds/x64/vst3-compatibility.json`; fresh inventory alongside it.
  Baseline crash attribution: `builds/x64/vst3-lifecycle-baseline.json`.
- Gateway's native editor was held open for a bounded 45-second message-pumped
  check and completed successfully. Computer-use inspection found its native
  window, but screenshot capture reported `Computer Use app approval timed out`.
  No screenshot-based visual acceptance is claimed.
- Installed CLA-76 mono/stereo audio checks pass, and the dedicated STA editor
  host opens/closes the installed Waves class in a bounded regression.
- `./build.ps1 -Architecture x64 -LocalAsio` succeeded. Release SHA-256:
  `84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`. `builds/x64/build-info.json` confirms `local_asio: true`.
- Patch whitespace check passed. The final build was not launched by this
  session; final in-app meter and save/reopen review remain pending.

## Limits and next review

Retrologue, Superior Drummer 3 and sforzando are instruments, reported as
`unsupported_instrument_routing`; they were not represented as working inserts.
MIDI/instrument hosting, sidechains, multi-out and production process isolation
remain separate unfinished capabilities. The per-class test processes isolate
this sweep, not production inserts. These bounded tests do not establish
universal compatibility, long-run/device stability, sound quality or identical
visuals. Review Gateway/Waves graphics and controls in the release, play real
audio, and save/reopen an edited project before user acceptance.

The transport ledger scope remains `transport-v16-pitch-time-editing-workflows`;
VST compatibility is outside that denominator. No transport readiness/parity
percentage was promoted by this work. Independent review and user acceptance
remain pending.
