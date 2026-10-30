# T058 — Realtime performance and stability program

- Version: v2.1, consolidated 2026-10-29; user decisions recorded 2026-10-29 (hard ceiling retained, 64-frame acceptance target).
- Status: baseline instrumentation partially implemented; expanded measurement and reproduction approved. Hardware results remain pending.
- Review: GPT-6.1 Sol and GPT-6 Astra findings are incorporated below. This document supersedes the previous phase instructions and review annotations.
- Goal: artifact-free playback at 64 samples on the user's Mackie ASIO interface with guitar tracks, GATEWAY modelers, and CLA-76/Ozone on Stereo Out. Decision (2026-10-29): 64 samples at the project rate is the working point and the acceptance target, reported by the driver as approximately 5 ms. 128 samples is a fallback, not a success. 32 samples is a possible stretch only after 64 is proven and is never acceptance. Driver-reported latency excludes converter round trip and plug-in latency, so it is a lower bound until a loopback measurement exists.
- Next approved work: complete bounded per-stage diagnostics and release overhead comparison, run a cross-host control with the same chain and device, evaluate ASIO callback scheduling class, and reproduce the reported two-guitar/CLA-76 crackling case. Select subsequent optimizations from the evidence.

## Current evidence and implementation

The renderer scans track clips per sample, making total clip count a relevant
scaling concern. This does not yet explain crackles with only two guitar tracks.
Projects with hosted inserts select a different render path from dry projects;
baselines must distinguish host-path overhead from plugin processing cost.

Implemented source changes include:

- EMA renderer load and a cumulative renderer deadline-miss/collision counter, displayed in the status strip at 4 Hz and retained across plan replacement.
- Playback-plan rebuild deferral during audio-event gestures, with the existing plan active until release.
- ASIO Input Bus DSP bypass when monitoring, recording, and input spectrum are inactive; input probe metering continues.
- Mixer fader/pan edits compare scalar control snapshots instead of cloning every track EQ on each frame. Live mix changes update the existing plan without constructing a temporary changed-track vector; changes that alter processing state still request a rebuild. This removes avoidable UI-thread work, but its effect has not yet been measured.
- Replaced live-input monitor buffers and cleared note-audition snapshots are retained on the control side until the callback-active gate is idle. Replaced note-preview loop sources are also retired off-callback, preventing their large source buffers from being finally freed on the audio thread.
- Live EQ target snapshots are loaded as owned `Arc`s by the callback and replaced targets are retained by the control object until the callback gate is idle. The control-side reclaimer drains EQ retirement queues for the current plan and transition-linked retired plans, keeping final EQ snapshot cleanup off the render callback.
- The ASIO MMCSS scheduling A/B is opt-in with `OPENTRACK_ASIO_MMCSS=1`. It registers on the first ASIO callback, records registration/priority status and first-call duration, then requests same-thread reversion before stream stop. The copied diagnostics report the outcome. A one-second teardown timeout is reported and the stream is stopped to avoid hanging shutdown; that timeout invalidates the A/B result and leaves driver-thread cleanup unverified.
- The status strip keeps concise load/overrun counters visible and shows timing detail on hover. Timing summaries and formatting are cached at 4 Hz rather than sorted and allocated on every UI repaint; their remaining control-thread and callback impact still needs measurement.
- Release profile with opt-level 3, fat LTO, one codegen unit, and unwind panic handling.

The existing load metric excludes some ASIO input/output work and does not
measure callback scheduling delays. A renderer overrun is not proof of a device
dropout. Audible crackles may also come from signal discontinuities or clipping.
maolan-asio exposes the ASIO `kAsioOverload` notification as
`stream::event::OVERLOAD`; the ASIO owner loop now counts polling windows that
contain the event and reports the count separately from renderer deadline
misses. The owner loop polls every 10 ms and the event is a latched bit, so the
counter measures windows with one or more overload notifications, not
individual overload events. Renderer, backend, hosted-track, Stereo Out, and
WASAPI input timing are implemented. WASAPI capture discontinuity and timestamp
error flags are counted during recording and reported when capture stops. The
WASAPI render path reports fill timing but has no device underrun/discontinuity
signal yet. Live driver/input validation and the release instrumentation-overhead
comparison remain pending. No measured improvement or stable 128/64-frame
workload is established yet.

Historical build evidence: commit 506c139 plus the idle-input/release-profile
changes produced an x64 local-ASIO package in 8m14s, SHA-256
`97F6FC253AF76FBFC9BF630C939F6D541C45F886AF41AC6ECB14A419472802DE`.
This is an earlier checkpoint, not the identity of a future benchmark build.
Every new measurement must record its actual commit, dirty diff, and artifact hash.

Current diagnostic build checkpoint: release build with `local-asio` completed
from commit `f4efbe70625f82da20d1bd2ffc93cb7c6570bdb2` plus the current dirty
worktree in 6m38s. ASIO timing slots validate their sequence before accepting
a sample; overwritten or concurrently skipped samples count as diagnostic loss.
The status strip exposes concise live counters and caches detailed summaries at
4 Hz for hover inspection. Detailed timing includes interval p95/max and the
negotiated device period for direct deadline comparison; right-click copies the
full summary with device format details. ASIO callback gate collisions are
counted separately from driver overload windows and renderer misses.
`opentrack.exe` SHA-256:
`296A3EA209F44F63FD71EB9A2C07E55F99FE2A38E8F6813657265F3ADF1C4B48`.
Replaced input-monitor, note-audition, and EQ snapshots are retained and
reclaimed on the control side after callbacks are idle. This verifies an
optimized build only; no playback or instrumentation-overhead measurement has
been made from this artifact.

## Requirements

| ID | Requirement |
| --- | --- |
| R1 | Measure callback entry intervals and processing duration against the actual device period. |
| R2 | Preserve CPU compatibility and measure release-build changes before claiming gains. |
| R3 | Diagnose backend scheduling and queue behavior; preserve output timeline continuity. |
| R4 | Reduce clip traversal cost when profiling identifies it, including correct seek/loop state. |
| R5 | Evaluate block source reads with bounded cache-miss behavior and no callback I/O. |
| R6 | Reduce redundant control/DSP work while retaining smoothing and event timing. |
| R7 | Evaluate renderer consolidation with explicit behavioral equivalence gates. |
| R8 | Define recoverable processing failures and the limits of in-process containment. |
| R9 | Report device failure and recover without silently changing selected routing/backend. |
| R10 | Retain the device hard ceiling; no host soft clip, knee, fold-back or limiter (decided 2026-10-29). |
| R11 | Publish bounded diagnostics without callback allocation, blocking, logging, or GUI access. |
| R12 | Retain reproducible workload, compatibility, regression, and user acceptance evidence. |

## Scope and invariants

Primary code areas are `crates/opentrack-audio/src/lib.rs`, `streaming.rs`,
`recording.rs`, the ASIO integration, `wasapi_backend.rs`, the app status surface,
and release build configuration. Keep runtime concerns out of `opentrack-core`.
Evaluate any new dependency against the existing architecture agreement.

Host-owned steady-state callback work must be bounded, allocation-free, and
free of blocking locks, file I/O, GUI access, and synchronous logging. Audit
third-party processing separately; host guarantees do not prove plugins meet
those constraints. Retire plans, plugin state, and buffers on the control side.

Preserve source audio, project persistence, routing, plugin settings, latency
compensation, per-sample fades, smoothing, metronome and audition behavior.
Keep existing output processing for performance/equivalence comparisons.

## Stage 1 — Release measurement and reproduction (R1, R11, R12)

This stage is approved to proceed. `OPENTRACK_REALTIME_DIAGNOSTICS` enables an
opt-in bounded ring for renderer invocation intervals and durations; the UI
calculates and displays p95 and maximum values off the audio thread, along with
sample count and how many older samples have been overwritten by the rolling
ring. maolan-asio records full buffer-switch callback entry intervals and
duration around driver buffer conversion, the OpenTrack callback, and output
conversion; its owner thread drains samples off the callback. WASAPI records
event-wake intervals and fill durations from available-space query through
render, encoding, and device write. This is an event-driven fill span, not a
native WASAPI callback duration. When enabled, bounded rings also capture each
track's complete hosted render block and the Stereo Out block; p95, maximum,
sample count and overwritten count are summarized off the audio thread. Compare
diagnostic overhead with the ordinary release build. WASAPI input packet timing
is also implemented, spanning packet query, fetch, decode, release, and the host
consume path. Capture discontinuity and timestamp-error flags are counted while
recording and reported when capture stops. Verify timing and flags on a live
capture endpoint along with the rest of the diagnostic instrumentation.
Record telemetry loss if any bounded diagnostic buffer fills.

Distinguish renderer deadline misses/collisions, late callback entry, backend
error/discontinuity signals where available, and artifacts in captured output.
For ASIO, surface `kAsioOverload` and count poll windows containing one or more
notifications separately from renderer deadline misses; this is the driver's
authoritative indication that it reported an overload, but the latched event
cannot reveal how many overload events occurred in a 10 ms polling window.
Also count callbacks rejected by maolan-asio's single-callback gate separately;
these are host-observed callback collisions, not driver overload notifications.
The ASIO overload-window and callback-collision counters are implemented;
verify the overload count against driver notifications and report all three
counters separately. WASAPI capture discontinuity and timestamp errors are
reported on recording completion; a WASAPI render underrun signal remains
separate work.
Counters are cumulative; after recovery their increment rate returns to zero.
The UI must label the evidence it actually measures.

Start with the actual two-guitar failure using fixed media and sample rate. Before
any engine change, run three control measurements that are cheaper than profiling:

**0a. Cross-host control.** Run the same device, buffer, sample rate and plug-in
chain in another ASIO host on this machine. A comparable failure classifies the
cause as driver, device or system contention rather than host rendering cost; a
clean result shifts the leading suspicion to the host callback path. Record the
result before selecting any DSP work.

**0b. Callback scheduling class.** The WASAPI render thread enters MMCSS "Pro
Audio" with AVRT_PRIORITY_HIGH. The ASIO callback thread is created by the
driver after `ASIOStart`, so keep registration out of the owner thread. Add an
explicit opt-in A/B that registers once on first callback, records
success/failure and first-call duration, and leaves steady-state callbacks free
of registration work. AVRT requires `AvRevertMmThreadCharacteristics` on the
same thread that registered; the adapter must request teardown and observe
same-thread reversion before stopping the stream. If that handshake cannot
complete, keep the experiment disabled for that stream. Compare off/on with
identical project, workload, device and settings, and report startup cost
separately. See T061 §0b and the Microsoft AVRT API documentation.

**0c. Internal hosted block granularity.** The renderer cannot submit more
frames to an insert than arrive in one device callback. At a fixed 64-frame
period, configured sizes of 128 and 256 therefore still execute as 64-frame
calls and cannot distinguish host block-path cost. Compare 32 and 64 at a
64-frame period using `OPENTRACK_HOSTED_BLOCK_FRAMES`; the status strip reports
configured and effective sizes. To compare 128 or 256 effectively, the device
period must be at least that large, which is a separate period comparison. Do
not aggregate callbacks to manufacture larger insert blocks; that adds latency
and is outside this A/B.

Then, for attribution, use:

1. Dry playback.
2. An identity hosted processor to expose host block-path overhead.
3. CLA-76 on Stereo Out with the user's settings.
4. GATEWAY in isolation and increasing independent instances.
5. Ozone in isolation, then the intended combined chain.
6. Fader/dB and MIDI controller edits, followed by a separate live-input monitoring pass when evaluating guitar-playing latency.

Use short screening runs to locate failure boundaries. Repeat five-minute
measurements and longer soaks for candidate stable configurations, following
[T061](T061-low-buffer-neural-amp-stress-plan.md). Keep actual sample rate,
plugin versions/models/presets/quality settings, window state, machine settings,
and media cache conditions fixed. At 48 kHz, 128 frames provide 2.67 ms and
64 frames 1.33 ms per period; these are not round-trip latency figures.

Exit: retain a reproducible case, timing distributions, captured-output
observations, and a ranked explanation of host, plugin, or scheduling cost.
If attribution is unresolved, state what evidence is still missing.

## Stage 2 — Targeted fixes selected from evidence (R2, R3, R6)

Choose the smallest demonstrated callback or control-path bottleneck first.
A renderer rewrite is not a prerequisite for a justified local fix. Compare
before/after measurements with equivalent settings and media.

The release profile already exists. Evaluate further compiler/SIMD changes
only against measured gains and compatibility; retain the current CPU baseline.
Runtime-dispatched kernels require feature checks and numerical review. Do not
silently adopt an x86-64-v3 minimum.

For WASAPI, inspect negotiated period, available capacity, queue padding, and
device position before changing fill behavior. There is no approved universal
one-period fill clamp or silence-padding policy. Define the timeline behavior
of a proposed policy and verify it under an induced delay. Fixed-size ASIO
callback behavior is a separate investigation.

Exit: measured improvement in the reproduced workload, passing affected
regressions, and no unexplained audio or latency change.

## Stage 3 — Rendering and scheduling candidates (R4–R7)

Proceed with a candidate only when profiling and its reviewed design justify
it. Candidates include indexed active-clip traversal, block source reads,
hoisted control revisions, and consolidation of hosted/non-hosted render paths.

Before implementation, specify initialization, forward playback, arbitrary
seek, backward loop, skip jump, short block, cache miss, and plan replacement
behavior. Preallocate state. A monotonic clip cursor needs a seek/reset path;
off-playhead clips still have indexing and transition costs.

Precomputed boundaries must preserve current timing and rounding semantics.
Keep fades and control ramps sample-accurate when preparing them per block.
Split blocks where timeline or content boundaries require it. Current plans
use constant tempo; piecewise-constant tempo permits piecewise-linear mapping,
while future ramps require a mapping derived from the supported tempo law.

Track parallelism is a separate measured scheduling candidate, with no fixed
minimum track-count rule. Respect routing dependencies, plugin-instance
ownership, and synchronization cost inside the deadline. Stereo Out processing
follows the sum and remains part of the dependent processing path.

Exit: scaling improvement where applicable, output equivalence, and host
allocation/blocking audit. Consolidation is adopted only after both plan shapes
pass; preserving two paths temporarily is acceptable during a bounded migration.

## Stage 4 — Fault and device recovery (R8, R9)

Specify failure classes before implementation. Rust panic recovery, plugin
error returns, slow calls, hung calls, native crashes, and device loss are
different conditions with different guarantees.

For recoverable host/plugin failures, define silence or dry/bypass behavior,
callback-gate cleanup, visible diagnostics, and control-thread retirement.
Review panic hooks and unwinding/destructors for real-time hazards. A watchdog
can flag or bypass a slow plugin after it returns; it cannot interrupt a hung
native call. Rust catch_unwind does not contain native access violations.
Process isolation requires its own design and measured latency cost.

Handle selected-device loss separately from a default-endpoint change. Preserve
explicit ASIO device/backend/routing choices; a switch requires a user-visible
choice. Recovery must preserve recording-file sample-rate consistency and
clearly report transport/recording state. Define a safe stopped state when
reconnection fails. Do not promise automatic cross-backend recovery.

Exit: induced recoverable failures follow the specified policy, with honest
limits for hangs/native crashes and demonstrated device recovery behavior.

## Quality design — Over-range clipping (R10) — decided 2026-10-29

Decision: retain the hard ceiling. The user chose the existing device clamp
unchanged, with no host soft clip, no below-zero knee, no fold-back shaper and
no host limiter. Current output behaviour is the specified behaviour, not an
unresolved preference. This is not a remedy for callback underruns and does not
change with the performance work.

The reasoning that closed the question: a continuous monotone transfer curve
bounded to [-1, 1] and identical inside that interval is forced to be flat
beyond its boundaries, so any curve satisfying the stated preference is exactly
the existing hard ceiling. The remaining options each change that preference —
a below-zero knee alters signal under 0 dBFS, and lookahead gain reduction
changes samples before the peak. Both were declined.

Consequence for oversampling: a rate-conversion container can legitimately
output beyond ±1 through inter-sample overshoot, so it can clip more than the
1x path would. Any future oversampling proposal must state whether it accepts
that or trims gain, and must not be described as quality-neutral at the ceiling.
No default sound change is approved by the performance plan.

## Acceptance and evidence

| Check | Evidence and expected result |
| --- | --- |
| A1: clip scaling | Repeated warmed release renders with identical audible content and 50 versus 5,000 clips across 40 tracks. Record absolute times and distributions. Target ratio after a traversal optimization: at most 1.5x; explain misses. This is separate from the small-session crackle diagnosis. |
| A2: output equivalence | Deterministic fixtures cover fades, crossfade pairs, muted/overlapping takes, mono/stereo, groups/solo, seeks, loop/skip boundaries, cache misses, plan swaps, and PDC. Expect bit identity when arithmetic is preserved. Any arithmetic change needs an explained error bound specified before acceptance. |
| A3: callback constraints | Audit host-owned steady-state, plan transitions, and defined failure paths for allocations, blocking and I/O. Measure third-party behavior separately. An allocation hook alone does not prove absence of locks or I/O. |
| A4: timing visibility | Controlled execution delay and late entry are reported by the corresponding metrics. Capture backend signals and audio separately. The implemented ASIO `kAsioOverload` counter is authoritative driver-reported overload evidence and counts 10 ms poll windows containing one or more latched notifications, not individual events; verify it against a live driver. WASAPI needs its own signal. Cumulative counters remain stable after the fault stops. |
| A5: backend continuity | An induced late WASAPI fill exercises the proposed queue policy without runaway work or unintended timeline changes; ASIO is checked separately. No pass requires an unconditional fill clamp. |
| A6: processing failure | Inject supported host panic/plugin error cases and verify specified output, gate cleanup, retirement and UI reporting. Hung/native-crash containment is not claimed without an isolation design. |
| A7: device recovery | Exercise selected-device loss/reconnection and default-endpoint change separately. Verify explicit routing/backend choices, transport state and recording-file consistency. |
| A8: regressions and hardware | Run affected tests and workspace checks, listing pre-existing failures separately. Verify metronome, audition, monitoring, recording, control edits and project reopening. Report maximum clean plugin workload and first failing workload at each buffer, with user listening review. |

Use independent equivalent processor instances/state for comparisons; never
concurrently process one live plugin instance through two renderers. Control
plugin noise/randomness or define a suitable comparison method separately.
A candidate stable configuration needs no observed audible artifacts and no
observed callback deadline misses during the declared run. Do not generalize
that finite result to universal stability. Retain build identity, workload,
settings, duration, timing distributions, counters, recordings/observations,
and before/after results. Screenshots are not required by default.

## Delivery and remaining decisions

Implement and review one bounded stage or candidate at a time. Measurement
precedes optimization selection; later stages are not an inflexible ordering
that blocks evidence-backed local fixes. Timing/threading changes require code
review and relevant behavioral evidence. Hardware listening and user workflow
acceptance remain separate from compilation and automated checks.

The Sol and Astra reviews are incorporated in this v2 plan. The document cleanup
changes no product code and establishes no new runtime result. Decisions recorded
2026-10-29: the hard output ceiling is retained (R10 closed, no host limiter),
and 64 samples at the project rate — reported as approximately 5 ms by the
driver — is the acceptance target rather than a stretch goal.

Remaining choices are the evidence-backed optimization, any device/backend
fallback policy, and any parallel/isolation design. Oversampling is noted but not
approved: see [T062](T062-optional-insert-oversampling.md), which is blocked on a
proven 64-frame workload and on a stated answer to the hard-ceiling overshoot
question. Driver-reported latency excludes converter round trip and plug-in
latency; a one-time loopback measurement should establish the real round-trip
figure so acceptance is not stated in units that flatter the result.

The next ready actions are release instrumentation-overhead measurement, then
in order: cross-host control, hosted block-granularity A/B at a fixed 64-frame
device period, the minimal reported ASIO reproduction, and the broader T061
chain matrix. The opt-in ASIO MMCSS A/B is implemented, but needs live Mackie validation;
a teardown timeout invalidates that run. Make 64 frames clean and complete its
soak before any 32-frame stretch observation.
