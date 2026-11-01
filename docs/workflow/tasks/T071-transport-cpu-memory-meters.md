# T071 - Transport CPU and memory meters

- Status: accepted by user (2026-11-01).
- Scope version: 1.
- User request (2026-11-01): a CPU usage meter like the input meter on the transport panel, plus RAM usage.

## Scope

- Widen the transport panel by a 132 px left inset and place the two meters to the **left of the input meters**, with visible "CPU" and "RAM" labels: a segmented gauge plus a numeric value for each.
- **CPU**: system CPU load (all cores, from `GetSystemTimes` idle/total deltas). The tooltip adds OpenTrack's own process CPU share.
- **RAM**: system memory load percentage on the gauge, with the used amount beside it (MB/GB). The tooltip adds total system memory and OpenTrack's own working set.
- Workspaces saved with the old 837 px transport width are widened on restore so the new meters cannot sit outside the window.
- No audio, project-format or persistence changes; the meters are read-only control-thread state.

## Architecture

`system_stats.rs` in the app crate samples `GetSystemTimes`, `GetProcessTimes`, `GlobalMemoryStatusEx` and `GetProcessMemoryInfo` on Windows through the existing pinned `windows-sys` dependency (three feature flags added: Threading, ProcessStatus, SystemInformation). Non-Windows builds return no samples so the meters read zero and the app still compiles. Sampling is throttled to 500 ms and never runs from the audio callback.

## Checks

- `memory_readout_switches_between_megabytes_and_gigabytes`.
- `a_fresh_sampler_reports_no_load`.
- Full app suite passes 205 with the same 14 known ruler/sample-editor failures; formatting checks pass.
- User report on 2026-11-01: the first version placed the meters off the visible transport width, so they were not seen. This revision moves them to a labelled left block and widens restored workspaces. Runtime review of that placement remains pending.
