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

## Checks

- `cargo test -p opentrack-vst3`: existing module/effect tests.
- `cargo test -p opentrack --bin opentrack`: app suite.
- Runtime reproduction on the user's machine is required; TBC2 options cannot be exercised in this environment.
