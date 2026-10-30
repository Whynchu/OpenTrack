# T062 — Optional per-insert oversampling (deferred note)

- Status: **deferred — not approved, not scheduled.** This file records the
  design constraints established on 2026-10-29 so the reasoning is not lost. It
  is not a request to implement, and it must not be started while T058/T061 are
  open.
- Blocking prerequisites:
  1. 64-frame stability is demonstrated on the Mackie ASIO interface and passes
     the T061 soak. Oversampling multiplies plug-in CPU and would corrupt that
     attribution work.
  2. The CPU budget is known from T061's measured workload, so the instance-count
     penalty can be stated as a number rather than a guess.
  3. The ceiling interaction below has a stated answer. T058 retains the device
     hard ceiling, which constrains what oversampling is allowed to do.
- User intent: quality interest only. Oversampling is not a stability fix and
  must never be presented as one.

## Why it is worth doing at all

Nonlinear stages generate harmonics above Nyquist that fold back as inharmonic
content. At high gain that is audible as harshness or fizz. Host-side 2x around
GATEWAY is the most defensible quality feature on the list.

It buys nothing for linear processing. Oversampling an EQ or clean gain pays
filter cost for no benefit. Ozone already oversamples its own Maximizer and
Exciter internally, so wrapping it in a host 2x would double-spend CPU and may
double-oversample. Any proposal must name which instances it applies to and why.

## Constraints if it is ever scheduled

- **Per-insert, opt-in, 1x / 2x / 4x, default 1x.** At 1x the path must be
  bit-identical to today, so no existing project changes sound or cost.
- **No global or whole-mix oversampling, and no automatic selection.**
- **Half-band polyphase FIR, linear phase**, per instance, state preallocated on
  the control side. FIR tails denormalize, so FTZ/DAZ handling is required.
- **Latency must join PDC.** The container reports total latency — filters plus
  any plug-in latency — through the existing `hosted_track_latency` path so
  alignment survives. The insert must display it, e.g. `×2 · +0.32 ms`. Hidden
  latency is not acceptable.
- **Block interaction.** The plug-in is prepared at `rate × factor` and
  `HOSTED_BLOCK_FRAMES × factor`. At a 64-frame period with 2x it therefore sees
  128-frame blocks, which is often more efficient per sample than 1x — one
  reason the cost is sometimes below a naive 2x.
- **Ceiling interaction (unresolved).** Up/downsampling produces inter-sample
  overshoot, so the downsampler output can exceed ±1 and clip harder than the 1x
  path would under the retained hard ceiling. The proposal must either accept
  that explicitly or trim gain, and must not claim quality neutrality at the
  ceiling.
- **Budget reality.** 2x roughly doubles that instance's CPU plus filter cost. On
  a 64-frame target it is likely incompatible with a high GATEWAY instance
  count; it may fit at 128 frames. Measure both before proposing a default.

## Acceptance sketch (only if scheduled)

1. 1x is bit-identical to the current path on a deterministic fixture.
2. Aliasing measurement, not assertion: a high-frequency sine at high gain
   through GATEWAY at 1x versus 2x, reporting folded energy below Nyquist. A
   claimed improvement without this measurement is not evidence.
3. Reported latency equals measured latency, and a 1x track stays aligned with
   an oversampled track under PDC.
4. CPU cost measured against the T061 workload at 64 and 128 frames, stated as
   the exact instance-count penalty.
5. User listening review of the curve of benefit versus instance count.

## Handoff

- Actual planner: not started; note recorded 2026-10-29.
- Changes: documentation only. No product code, no default sound change.
- Next action: none. Revisit only after T058/T061 report a proven 64-frame
  workload.
