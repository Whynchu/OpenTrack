# T069 - VST3 load stall diagnostics and lazy editor probe

- Status: review_pending; needs a trace from the user's machine.
- Scope version: 1.
- User report (2026-11-01): loading settings for TDR Nova or Tonal Balance Control 2 freezes OpenTrack.

## Diagnosis so far

Two known costs sat on the UI thread when an insert was created:

1. `Vst3Effect::load` asked the controller for an `"editor"` view immediately (`probe_editor`) solely to learn whether an editor exists. Building a plug-in view is expensive, and some plug-ins block while constructing it. That happened for every instance load, before any state restore.
2. Restoring large component/controller chunks calls `setState`/`setComponentState` on the UI thread. A big state (Tonal Balance Control 2) can take a long time; a plug-in that waits for its own window/message loop during `setState` can stall indefinitely.

## Change

- `has_editor` is now a lazy `AtomicBool`: instance loading no longer creates an editor view. The flag becomes true when the editor actually opens, and a public `probe_editor()` still creates and releases one view for the ignored installed-plug-in smoke tests.
- `Runtime::sync` and `Runtime::load_insert` print stage timings when `OPENTRACK_VST3_TRACE=1` is set: module path, instantiation time, state-restore time with byte count, and total sync time. These lines go to stderr, which the Windows build shows when launched from a terminal.

This removes the eager view creation (the likely freeze) and makes any remaining stall attributable to instantiation or state restore.

## How to test

Run OpenTrack from a terminal with the trace enabled, load the project/preset with TDR Nova or Tonal Balance Control 2, and note the last trace line before the stall:

```powershell
$env:OPENTRACK_VST3_TRACE = "1"
.\builds\x64\OpenTrack.exe
```

- No `instantiated` line for the plug-in: the stall is inside module load / component creation.
- `instantiated` but no `state restore` line: the stall is in `setState`.
- Both lines present with large millisecond values: the load is slow rather than hung.

## Checks

`cargo test -p opentrack-vst3` passes 14 tests with 2 ignored; focused app VST3 checks pass 5. Runtime diagnosis on the affected plug-ins is still needed; no live reproduction was possible in this environment.
