# T070 - Tonal Balance Control 2 options crash

- Status: diagnostics added; root cause not yet confirmed.
- Scope version: 1.
- User report (2026-11-01): opening Options (the gear) inside Tonal Balance Control 2 crashes OpenTrack.

## Likely area

The crash happens inside the plug-in's native editor, not in OpenTrack's own panels. Two host-side suspects:

1. `IPlugFrame::resizeView` resized the host editor window for **any** view that asked. A plug-in options popup or sub-view is not the main editor view, so resizing the host window and then calling `onSize` on that sub-view can leave the plug-in with mismatched window/view state.
2. Native faults (access violations) never reach the Rust panic hook, so the previous logging inside `crash_log` could not record them.

## Change

- `IPlugFrame::resizeView` now only resizes the host window when the requesting view is the main view attached to it (`HostShared::editor_view`). Sub-views and popups still receive `onSize` for their own rectangle.
- `crash_log` now also installs a Windows unhandled-exception filter. On a native fault it appends the exception code, faulting module name, and raw return addresses to `%LOCALAPPDATA%\OpenTrack-\logs\exception.log`. Rust panics still go to `panic.log` in the same folder.

## How to test

Reproduce opening Options in TBC2, then check:

```
%LOCALAPPDATA%\OpenTrack-\logs\exception.log
%LOCALAPPDATA%\OpenTrack-\logs\panic.log
```

The exception log names the faulting module (for example the plug-in DLL) and the code (`0xC0000005` for an access violation). Raw frames are included; they can be matched to `builds\x64\OpenTrack.pdb` from the same build if a crash is inside OpenTrack.

If the crash disappears with the `resizeView` guard, that confirms suspect 1 and the guard is the fix. If it still crashes, the log identifies whether the fault is in the plug-in DLL or in OpenTrack.

## What the evidence shows

The crash is a **hang**, not an access violation. Windows recorded `Application Hang` (event 1002) for `OpenTrack.exe` at the time of the report, and neither `exception.log` nor a new `panic.log` was produced. So the process stops responding rather than faulting.

## Additional diagnostics

- A UI heartbeat is sampled by a background logger thread. It writes `ui alive (heartbeat N)` every five seconds and `ui stalled` every second once frames stop.
- The VST3 editor thread logs its start, each command, the open/close events, and `editor thread: alive` about every two seconds. The app logs editor open/close requests.
- All of that goes to `%LOCALAPPDATA%\OpenTrack-\logs\session.log`, truncated at each launch.

After a hang, the log tail shows which side stopped: `ui stalled` but `editor thread: alive` means the main thread is blocked; a missing editor heartbeat means the editor thread is stuck (for example inside the plug-in's own modal options loop).

## Hardening

`EditorHost::close` no longer waits on the editor thread indefinitely. It sends the close command, waits up to two seconds, then detaches a thread that does not exit, so a plug-in stuck in its own modal loop cannot freeze the application. The log records `editor thread did not exit within 2 s; detached`.

## Root cause (confirmed from session.log)

The user's 2026-11-01 11:40 session shows:

```
[..205] app: opened editor "Tonal Balance Control 2"
[..208] editor thread: alive
[..210] ui alive (heartbeat 352) / editor thread: alive
[..213] ui stalled (heartbeat 377)   <- frozen from here on
```

The main UI thread froze while the editor thread stopped its two-second `alive` ticks at the same moment. That is the plug-in's modal Options loop holding its controller/UI state while the **main thread** was inside the edit controller: `show_vst3_windows` called `sync_output_parameters` every ~16 ms, which calls `IEditController::setParamNormalized`. VST3 edit controllers are not thread-safe and iZotope holds a lock during the modal loop, so the main thread blocked permanently.

Fix:

- Processor-produced parameter changes are now applied on the **editor thread**, in its message loop, where the view lives. The main thread no longer calls the controller while an editor is open.
- `capture_states` skips any insert with an attached editor, so saving while an editor is open cannot query state across that same lock; the state is captured when the editor closes.
- `EditorHost::close` waits at most two seconds and then detaches a stuck editor thread, so a plug-in modal loop can no longer freeze the application.

The session log and heartbeat tracing stay in place, so a recurrence is still diagnosable.

## Second hang (12:37)

The controller deadlock fix was not enough. A second session shows:

```
[..629] editor thread: opening "Tonal Balance Control 2"
[..629] editor thread: opened
[..630] app: opened editor / command set-owner
[..632] editor thread: alive
[..634] ui stalled (heartbeat 1186)   <- frozen from here on
```

The UI froze about four seconds after the editor opened, and the editor thread's next `alive` tick never arrived. So both threads stop, but the main thread is no longer in `setParamNormalized` (that call now runs on the editor thread).

To locate the remaining block, the watchdog now records the **UI stage** at the moment frames stop: `ui stalled at <stage>` where stage is `workspace`, `dialogs`, `vst-windows`, `vst-timers`, `vst-bounds` or `vst-sync`. That names the blocking section on the next reproduction:

- `vst-bounds` -> the editor-host bounds mutex.
- `vst-timers` -> dispatching thread timers into the plug-in on the UI thread.
- `vst-sync` -> plan rebuild's `Runtime::sync` (instance load/state restore).
- `workspace`/`dialogs` -> rendering or a dialog, not an obvious plug-in call.

## Third reproduction: `vst-timers`

The stage log then named the culprit directly:

```
ui stalled at vst-timers (heartbeat 411)
```

The main thread was inside `pump_ui_thread_timers`, dispatching WM_TIMER messages on the UI thread while an editor was open. The plug-in creates that timer during component initialization, which happens on OpenTrack's UI thread; dispatching it re-enters the plug-in on the UI thread while its editor thread is inside the modal Options loop, so the two block each other.

Fix: OpenTrack no longer dispatches plug-in thread timers from the UI thread. The editor thread pumps its own messages, which is where view work belongs. `pump_ui_thread_timers` was removed. The trade-off is that iPlug2-style processors that push meter data through an idle timer may no longer update their editor from the host thread; that is preferable to a host freeze and can be revisited with a timer mechanism that cannot re-enter the plug-in.

## Checks

- `cargo test -p opentrack-vst3`: existing module/effect tests.
- `cargo test -p opentrack --bin opentrack`: app suite.
- Runtime confirmation of TBC2 Options on the user's machine is pending.

