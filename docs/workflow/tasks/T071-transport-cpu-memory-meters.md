# T071 - Transport CPU and memory meters

- Status: accepted by user (2026-11-01).
- Scope version: 1.
- User request (2026-11-01): a CPU usage meter like the input meter on the transport panel, plus RAM usage.

## Scope

- Widen the transport panel from 837 to 1000 px and add a right-hand section with two segmented gauges in the input-meter style.
- **CPU**: OpenTrack's own process CPU usage, normalised so all logical cores busy reads 100%. Sampled every 500 ms from process kernel+user times.
- **RAM**: system memory load percentage on the gauge, with the used amount beside it (MB/GB). The tooltip adds total system memory and OpenTrack's own working set.
- No audio, project-format or persistence changes; the meters are read-only control-thread state.

## Architecture

`system_stats.rs` in the app crate samples `GetProcessTimes`, `GlobalMemoryStatusEx` and `GetProcessMemoryInfo` on Windows through the existing pinned `windows-sys` dependency (three feature flags added: Threading, ProcessStatus, SystemInformation). Non-Windows builds return no samples so the meters read zero and the app still compiles. Sampling is throttled to 500 ms and never runs from the audio callback.

## Checks

- `memory_readout_switches_between_megabytes_and_gigabytes`.
- `a_fresh_sampler_reports_no_load`.
- Full app suite and formatting checks.
- User confirmed on 2026-11-01: “the cpu meter is done.” Treat the T071 gate as complete and proceed to the roadmap. No additional runtime evidence was recorded in this packet; the acceptance statement is user confirmation, not an independently observed desktop check.
