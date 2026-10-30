# T061 — Low-buffer neural amp and master-chain stress plan

- Status: user-created four-track GATEWAY bench identified; initial playback results reported; formal attribution matrix and soak not performed.
- User goal: make 64 samples artifact-free on the Mackie interface with multiple
  GATEWAY Neural Amp Modeler instances and Ozone on Stereo Out. Decision
  (2026-10-29): 64 samples is the working point and the acceptance target; the
  driver reports approximately 5 ms at that setting. 128 samples is a fallback
  reference, not the target. 32 samples is a stretch to evaluate only after 64
  passes the soak; it is not acceptance and must not delay the 64 result.
  Identify whether crackles come from the engine, a plug-in, device scheduling,
  or system contention before changing DSP architecture.
- Known setup: local ASIO, Mackie interface, crackling at 64 frames with two
  guitar tracks and CLA-76 on Stereo Out. The 5 ms figure is driver-reported and
  is therefore a lower bound: it excludes converter round trip and plug-in
  latency, so one loopback measurement should establish the true round-trip
  figure. Confirm sample rate, driver-reported period, Windows power mode,
  plug-in versions, and exact preset/quality settings before each measured run.
- At 48 kHz, 128 frames provide 2.67 ms per period and 64 frames provide
  1.33 ms. Recalculate from the actual sample rate; these are callback periods,
  not full round-trip latency figures.
- Scope: performance and stability evidence for playback, plug-in scaling, and
  live fader/db edits. Do not overwrite the user's project or change source
  audio. Save an isolated stress-project copy and use duplicated DI material.

## Preliminary playback evidence (user report, 2026-10-29)

One GATEWAY track reportedly plays without audible glitches at 32 samples;
two GATEWAY tracks reportedly glitch at 64 samples. The user has since created
a four-track GATEWAY stress project and reports pronounced pops and crackles
at 64 samples. Use this as the primary repeatable bench case.

Sample rate, project/build identity, plugin versions and presets, internal block
size, master inserts, and copied callback diagnostics for the four-track run are
not yet recorded. The 32-frame single-instance result and 64-frame multi-instance
failures change both buffer size and instance count, so they do not isolate the
cause. No loopback capture has been provided; do not attribute the result to the
renderer or plugins from these reports alone.

### Bench project inspection

Read-only inspection of `C:\Users\Samue\Documents\EXAMPLE\STRESS TEST.opr`
shows a 48 kHz project with four Gateway inserts. The active `Rhythm` audio track
has one Gateway and CLA-76 Mono; three additional audio tracks each have Gateway
but are saved muted. The `Guitars` group has CLA-76 Mono, and Stereo Out has
CLA-76 Stereo. The muted tracks contain copies of the same `Rhythm 01.wav` media.
OpenTrack skips muted tracks before rendering their hosted inserts when spectrum
capture is off, so the saved state exercises one Gateway plus the three CLA-76
inserts in the audible path, not four actively processing Gateways. This matches
the reported crackling but does not yet test four active Gateway instances.

For an actual four-instance run, use a disposable copy and unmute Audio 09,
Audio 10, and Audio 11. Keep the saved compressor chain for the first exact
reproduction; then bypass compressors in controlled follow-up runs to isolate
their contribution. Do not overwrite the user's original stress project.

At 64 frames, compare dry, one, two, and four GATEWAY instances using the same
project, duplicated DI source, plugin settings, and master chain. Keep plugin
editors closed during playback measurements. Collect the copied diagnostics
for each run: renderer misses, ASIO overload windows, callback collisions, ASIO
interval/duration p95 and maximum, and hosted-track block timings. If possible,
run the same four-instance chain in another ASIO host before choosing an engine
optimization.

## Test protocol

### 0. Make the run reproducible

1. Record OpenTrack commit/build hash, ASIO driver/device name, sample rate,
   driver-reported buffer size, internal hosted block size shown in diagnostics,
   block/period duration, Windows power mode, CPU, RAM, and background audio
   applications.
2. Record exact GATEWAY, Ozone, and CLA-76 versions, presets, oversampling or
   quality modes, lookahead/latency settings, and plug-in window state. Use the
   same settings throughout each comparison.
3. Create a disposable project with 8–16 minutes of guitar DI audio copied to
   the project media folder. Keep input monitoring, recording, and unrelated
   spectrum displays off for playback-only passes. Include one short exposed
   transient passage and one dense sustained passage.
4. Record a loopback output for artifact review. For every run, save callback
   load, renderer deadline misses, ASIO callback collisions, ASIO driver-reported overload windows, OS
   per-core CPU and process CPU, memory, ASIO settings, and notes about audible
   clicks, dropouts, UI stalls, or recovery. Record the implemented ASIO
  overload-window counter separately from renderer deadline misses; validate
  its behavior against the driver's notifications during hardware runs. WASAPI
  capture discontinuities and timestamp errors are separately counted when
  recording; the WASAPI render path still has no device underrun signal.

### 0a. Cross-host control — before any engine work

Run the same device, 64-frame buffer, sample rate and plug-in chain in another
ASIO host on this machine. A comparable failure classifies the cause as driver,
device or system contention (USB/DPC latency, power plan, driver) and stops this
plan before any DSP architecture change. A clean result makes the host callback
path the leading suspect. Record the result explicitly; do not infer it from
pass criteria after the fact.

### 0b. ASIO callback scheduling class

The WASAPI render thread enters MMCSS "Pro Audio" with AVRT_PRIORITY_HIGH. For
ASIO, the driver creates the callback thread after `ASIOStart`. The A/B should
use an explicit opt-in and register once on first callback entry, recording
registration success/failure and first-call duration. Do not resolve the task
name or call registration on every callback. AVRT requires
`AvRevertMmThreadCharacteristics` on the registering thread, so the adapter
must request teardown and observe reversion on that callback thread before
stopping the stream. If it cannot complete that handshake, keep the experiment
disabled for that stream. Compare off/on with identical workload and settings;
report startup cost separately from steady-state timings. Windows documents the
same-thread requirement in
[AvRevertMmThreadCharacteristics](https://learn.microsoft.com/en-us/windows/win32/api/avrt/nf-avrt-avrevertmmthreadcharacteristics)
and requires reverting when the task is complete in
[AvSetMmThreadCharacteristicsW](https://learn.microsoft.com/en-us/windows/win32/api/avrt/nf-avrt-avsetmmthreadcharacteristicsw).

### 0c. Hosted block granularity at a fixed device period

At a fixed 64-frame ASIO period and with a deterministic identity hosted
processor, compare effective internal hosted block sizes of 32 and 64 by
setting `OPENTRACK_HOSTED_BLOCK_FRAMES` before launch. Configured sizes larger
than the device callback are capped by the available frames, so 128 and 256 do
not create larger insert calls at a 64-frame period. Test those sizes only at
matching or larger device periods and treat them as a separate period result.
Measure callback entry intervals, processor time, and captured output. Change
only the internal processing block size; keep the driver period, media and
plugin state fixed. This isolates host block-path overhead before the real
plugin matrix. If a candidate block size changes output timing or behavior,
record the result and restore the baseline before continuing.

### 1. Establish attribution at the 64-frame target

First use short screening runs to reproduce the two-guitar/CLA-76 case and
locate the load boundary. Run the matrix at 64 frames first. For the 128-frame
fallback, repeat only the conditions needed to locate the changed failure
boundary. The existing 256-frame setup is a reference condition, not a reason
to postpone the 64-frame investigation. For candidate stable configurations,
warm up for 60 seconds, then play the same five-minute section three times.
Keep machine settings and media fixed between comparisons. T058 v2 defines the
measurement prerequisites and implementation gates for this matrix.

| Pass | Track and insert load | Purpose |
| --- | --- | --- |
| A | Two DI guitar tracks, no inserts | Engine/device baseline |
| A2 | Same tracks with a deterministic identity hosted processor | Host block-path overhead |
| B | Same, CLA-76 on Stereo Out | Reproduce the reported case |
| C | One GATEWAY on one guitar track | Single modeler cost |
| D | Two, four, then eight GATEWAY instances on separate DI tracks | Scaling curve |
| D2 | Two dry DI tracks with Ozone on Stereo Out | Isolated master-chain cost |
| E | Highest clean GATEWAY count plus Ozone on Stereo Out | Target production chain |
| F | Pass E plus CLA-76 on Stereo Out | Compare against the reported chain and identify added cost |

Record plugin latency and compensate identically in all comparisons. Do not
open plugin editors during the playback-only measurements; run a separate UI
pass later so editor redraw cost is not mixed with DSP cost.

### 2. Measure live control impact

At the highest workload that passes playback-only at 64 frames, repeat while
moving one track fader/db control slowly, then quickly. If 64 is not yet clean,
run the same control pass at the 128-frame fallback so control-thread cost is
still characterized. Include MIDI volume/controller movement as a separate pass.
Note whether callback load rises, renderer deadline misses or ASIO overload
windows increment, and whether artifacts coincide with UI edits. This isolates
control-thread work from steady-state plugin DSP.

### 3. Stability soak and recovery

Run the target 64-frame configuration for 30 minutes with the same repeatable
passage. If it fails, fall back to 128 frames and report that as the fallback
result. If 64 passes and the driver exposes 32 frames, run a separate 32-frame
stretch soak and report it independently; a 32-frame result never replaces or
delays the 64-frame result. At each level, verify stable playback after a
stop/restart and project close/reopen. Only after baseline evidence is saved,
perform one controlled device stop/start or plugin bypass recovery check. Do not
combine device loss, plugin faults, and buffer changes in the same run.

## Pass criteria and interpretation

- Pass a buffer only when the captured output has no audible clicks/dropouts,
  the renderer deadline-miss and ASIO callback-collision counts remain zero,
  the ASIO driver-reported overload-window count remains zero, and the operating system shows no
  sustained single-core saturation. A driver overload is authoritative
  evidence that ASIO reported an overload even when the renderer metric is
  clean. The owner loop polls every 10 ms and the event is latched, so this
  counter measures windows with one or more overload notifications, not the
  exact number of events. Validate this counter against the live driver before
  using zero as pass evidence. The current app metric is an EMA render estimate, not a full ASIO callback trace;
  collect per-callback peak/percentile timing before using a percentage
  threshold as a hard acceptance gate. WASAPI needs its own signal before the
  same evidence can be required there.
- Report the maximum clean GATEWAY instance count with Ozone, and separately
  with Ozone plus CLA-76, at each buffer. Also report the first failing count
  and whether the failure follows an instance, a master insert, or live UI work.
- A 64-frame run is the acceptance target; a 128-frame pass is a fallback
  result and must not be reported as the outcome when 64 is the working point.
  Do not trade audible artifacts for a nominal buffer value.
- A 32-frame result is a separate stretch observation with its own soak, and is
  only attempted if the driver exposes 32 frames. It never replaces or delays
  the 64-frame result and is not part of acceptance.
- Latency results must name their basis. Driver-reported ASIO latency is a lower
  bound and excludes converter round trip and plug-in latency; a loopback
  measurement is the figure comparable across hosts and plug-in chains.
- If the same chain fails in another ASIO host on the same device and machine,
  classify that as evidence for plugin/device/system cost, not proof. If only
  OpenTrack fails, capture a minimal project and callback trace for engine
  profiling before optimizing.

## Measurement prerequisites and implementation gates

1. Use the opt-in timing capture for renderer interval/duration, ASIO full
   buffer-switch interval/duration, and WASAPI event-wake interval/fill
   duration. WASAPI fill duration spans available-space query, rendering,
   encoding, and device write; it is not a native driver callback duration.
   Per-track hosted block, Stereo Out block, and WASAPI input packet timing are
   implemented. Verify the input trace on a live capture endpoint.
   Compute percentiles off the callback and measure instrumentation overhead.
   Keep callback instrumentation allocation-free and nonblocking. Record the
   callback thread's scheduling class and priority alongside the timings where
   the backend exposes them safely. The opt-in ASIO MMCSS A/B is implemented
   with first-callback setup timing and same-thread teardown; a reported
   teardown timeout invalidates the A/B result and leaves cleanup unverified.
2. Capture a baseline before renderer changes. Continue T058's active-clip and
   block-first work only with output equivalence at loop boundaries, crossfades,
   group routing, monitoring, and hosted-insert latency. Surface the ASIO
   `kAsioOverload` event in the diagnostics; report overload poll windows, not
   individual events, until the wrapper exposes a non-latched count. Keep a
   WASAPI overrun/discontinuity signal as separate pending diagnostic work.
3. Profile the actual expensive GATEWAY/Ozone chain. Evaluate oversampling,
   quality, plugin scheduling and shared buses as separate options; retain
   plugin settings and compensation behavior unless the user changes them.
4. Repeat the same matrix after every optimization and publish before/after
   results with build hashes. Keep the final machine playback and listening
   review with the user.

## Handoff

- Actual planner: Codex; renderer, backend, hosted-track, Stereo Out block and WASAPI input packet timing is implemented. Live input/driver validation and hardware stress runs remain pending.
- Checks: source review of T058, callback-load/overrun reporting, and the ASIO
  overload event path. ASIO overload windows are surfaced in the status strip;
  live validation against the Mackie driver and input endpoint, the MMCSS A/B,
  plus release instrumentation-overhead comparison, remain pending. The current dirty-tree
  local-ASIO release build completed at commit `f4efbe70625f82da20d1bd2ffc93cb7c6570bdb2`;
  ASIO timing slots validate their sequence and report skipped samples as
  diagnostic loss, and the status strip caches timing details at 4 Hz, including
  callback interval maxima, negotiated device period, and ASIO callback-gate
  collisions as a separate count. Right-clicking the status counters copies the
  full diagnostic summary with device settings. Replaced input-monitor,
  note-audition and EQ snapshots are retained until callbacks are idle, so final
  buffer/source reclamation stays on the control thread.
  `opentrack.exe` SHA-256 is
  `296A3EA209F44F63FD71EB9A2C07E55F99FE2A38E8F6813657265F3ADF1C4B48`.
- Pending: obtain actual sample rate, exact plugin versions/settings, then run
  the disposable-project matrix on the Mackie ASIO device.
- Stop point: report the measured 64-frame target result and 128-frame fallback
  separately, then recommend the next optimization before deeper callback
  architecture changes. Evaluate 32 frames only if 64 passes the soak and the
  driver exposes 32; it remains a stretch result.
