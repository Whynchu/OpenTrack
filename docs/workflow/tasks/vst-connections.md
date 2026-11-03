# VST Connections

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: Devices > VST Connections... (F4, `devices.vst_connections`) opens a read-only window listing the device buses and the project's bus channels with name, channel width, device, output and send counts.
- Reference evidence: `device_buses`, `project_buses` and the `device_connections` tests in `crates/opentrack-app/src/app/device_connections.rs`; the existing `vst_connections_open` flag, F4 and keymap command.
- Dependencies: none; reads existing project, routing and device state.

## What existed and what was missing

The Devices row, F4 and the `devices.vst_connections` command already toggled `vst_connections_open`, and the flag was in the modal guard, but no window drew it, so the controls did nothing. This slice adds the missing surface without adding routing behaviour.

## Behaviour

- **Device Buses:** Stereo In and Stereo Out with their width, the device in use (resolved input name, engine output name, or "System default"), and counts: audio tracks recording from the input bus, and channels reaching Stereo Out directly (a track routed into a group is excluded).
- **Project Buses:** every group and FX return with its kind, width, resolved output (a group target shows the group's name) and how many sends target it (`Group:<id>` or `FX:<id>`).
- Read-only by design: changing an output or a send happens in the channel strip, and the window says so. Closing it uses the ordinary outside-click dismissal.
- The empty state names that there are no group or FX returns yet.

## Verification

- `cargo test -p opentrack device_connections` -> 3 passed: device-bus widths/devices/user counts with the grouped track excluded from Stereo Out, group output resolution and send counts, and the window drawing Stereo In/Out, FX Return and its send count.
- Inventory regenerated: 277 leaves, 50 available, **172 state-dependent, 47 unavailable**, 8 informational. The project-open row was already state-dependent (the flag existed); the empty-shell row stays unavailable, so the count is unchanged — the change is that the row now opens a real window.
- App suite 605 passed / 4 known device-dependent roll failures / 9 ignored (618 tests).

## Not claimed

No desktop review of the window; the tests draw it headlessly. Multi-device/multi-port routing does not exist, so the window reports the single stereo device pair the engine actually uses.
