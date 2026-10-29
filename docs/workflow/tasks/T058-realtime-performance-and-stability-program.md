# T058 — Realtime performance and stability program

- Status: phase 1 measurement instrumentation implemented; runtime evidence pending
- Scope version and requirement IDs: v1 (2026-10-28); user request for "astonishing speed and reliability" at high quality. R1–R12 below.
- Desired observable outcome: the engine's playback cost tracks *audible content*, not project size; callback work stays bounded and allocation-free; a fault in one plug-in or one device event degrades to a visible, recoverable condition instead of silent audio death; the user can read actual DSP load and dropout counts while working.
- Reference evidence (observed vs user-specified vs unknown):
  - Observed in source (this audit, 2026-10-28): playback cost scales with *total* clip count per track, not with audible clips.
  - Observed: no `[profile.release]` section exists in any `Cargo.toml`; `build.ps1` runs plain `cargo build --release`, so builds use no LTO, 16 codegen units, and baseline SSE2 with no FMA.
  - Observed: render fill is sized by `get_available_space_in_frames()`, so a late callback renders a *larger* burst.
  - Observed: no render-side discontinuity counter; capture-side only.
  - Observed: output stage hard-clips every sample; no NaN/denormal guard; no `catch_unwind` at the callback boundary; a render write failure sets `failed` and stops silently.
  - User-specified: current low-latency feel is good and must not regress; quality must not be traded away for speed.
  - Unknown: actual DSP load headroom on the user's stage session; actual clip-count scaling on real projects. Both are measured by this packet, not assumed.
- Dependencies and decisions needed:
  1. **SIMD baseline.** `x86-64-v3` (AVX2+FMA) is proposed but excludes pre-Haswell CPUs. Decide: v3 baseline for the x64 build, or runtime-dispatched kernels. Do not silently drop older CPUs; record the choice.
  2. **Renderer unification (R7).** Proposed as required, because two renderers must stay behaviorally identical and every change is currently made twice. Confirm before implementation.
  3. **Quality scope (R10).** Soft clip and optional true-peak limiter are in scope; per-insert oversampling is a separate decision after R1–R6 land. No hidden quality defaults.
  4. **Tempo map.** The integer-sample precomputation in R4 is exact only while a plan has a single constant tempo (`plan.tempo_bpm`). If a tempo map is introduced later, R4 must become a precomputed piecewise-linear table. Record this constraint where the mapping lives.
- Code scope and architecture boundaries:
  - `crates/opentrack-audio/src/lib.rs` — plan construction, `mix_track_frame`, `mix_frame_with_policy`, `render_hosted_blocks`, `MixControl`, `ChannelProcessor`, `StereoMeter`.
  - `crates/opentrack-audio/src/streaming.rs` — block-oriented source reads.
  - `crates/opentrack-audio/src/wasapi_backend.rs` — fill clamp, underrun counting, device notification.
  - `crates/opentrack-app/src/app.rs` — status surface for load/dropouts.
  - `Cargo.toml` and `.cargo/config.toml` (or `build.ps1`) — release profile and target features.
  - Unchanged boundaries: the audio callback remains lock-free, allocation-free, and free of blocking I/O and GUI access (`architecture-agreement.md` invariant). No new dependencies without evaluation. Keep `opentrack-core` free of runtime/realtime concerns.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  - **A1 — Scaling diagnosis (headless).** Build a `PlaybackPlan` fixture with identical audible content and 50 vs 5,000 clips distributed across 40 tracks. Render 10 s offline in release. Record both times. Expected after R4: total-clip-count ratio ≤ 1.5×. Before R4 the ratio is expected to grow roughly linearly with clip count; record the actual number as the baseline.
  - **A2 — Renderer equivalence.** Render the same fixture (fades, an intentional crossfade pair, a muted clip, mono and stereo tracks, a group, solo state, loop boundary inside a block) through both the current per-sample path and the new block path. Expect bit-identical output; if not identical, every deviation is reported with its magnitude and cause, not waved through with a tolerance.
  - **A3 — Allocation and blocking audit.** A test hook counts allocations inside a rendered block and asserts zero during steady-state playback with hosted inserts. Assert no panic, no lock acquisition, and no file I/O on the callback path.
  - **A4 — Dropout visibility.** Induce a synthetic callback overrun and confirm the UI reports a dropout count and current DSP load; confirm the count returns to steady state without restarting audio.
  - **A5 — Late-callback containment.** Force a delayed callback and confirm the render fill is clamped to one period with silence padding, that playback resumes without a runaway burst, and that the event is counted as one dropout.
  - **A6 — Fault containment.** Inject a panic inside the callback and a plug-in `process` that returns an error. Expect one silent block (or insert bypass) with a red insert badge, and a live application in both cases.
  - **A7 — Device loss.** Change the default output endpoint or unplug the active device during playback. Expect automatic recovery (same period → next period → shared compatible) with a visible message, not silent audio death.
  - **A8 — Regressions.** `cargo test --workspace`; loop/skip boundaries landing inside a block still execute exactly; PDC, recorded input monitoring, metronome, note audition and hosted-insert latency unchanged; existing projects play identically by ear at the previously used buffer.
  - Evidence to retain: baseline and post numbers, fixture definitions, and the equivalence result. No screenshots required by default.
- Affected regressions: transport and loop/skip boundaries inside a block; crossfade and take-layering semantics; group routing and summing order; hosted insert PDC; mono/stereo conversion; recording and input monitoring; metronome and audition buses; project save/reopen (unchanged state expected); ASIO and WASAPI mode selection.
- Essential gates: measurable reduction in playback cost growth with clip count; bit-identical or fully explained output; zero allocations and zero blocking on the callback path; visible load/dropout reporting; recovery from induced panic, plug-in failure and device loss. Compilation is not evidence for any of these.
- Review owner and milestone stop: architecture review (Astra/Sol) before phase 1 starts; timing/threading review after phase 2; user listening and workflow review after phase 4. Stop at each phase boundary.

## Phases

Each phase is independently implementable and independently reviewable. Do not reorder; phase 1 produces the baseline that later phases are judged against.

### Phase 1 — Measurement (R1, R11)

- Callback duration vs period, EMA-smoothed, exposed as a DSP load percentage; monotonic dropout counter; per-stage timing available in debug builds only.
- Status surface in the app; no new windows.
- Establishes A1's "before" number and A3's audit hook.
- Exit: load and dropout are visible and respond to a real session.

## Current implementation checkpoint

The audio callback now publishes a smoothed render-time/load estimate and counts
callbacks that exceed their audio-period budget, including overlapping callback
collisions. The app status strip displays both values and refreshes at 4 Hz.
Metrics survive playback-plan replacement. Audio clip fade/gain/trim drags now
defer playback-plan reconstruction until release, avoiding repeated full plan
builds during a gesture while retaining visual edits in the project. The user
reports ASIO output at 256 frames on a Mackie interface; the period is about
5.3 ms at 48 kHz. Runtime load/overrun values and listening confirmation are
still needed before attributing the pops to the render thread.

The idle ASIO capture path now skips Input Bus processing when monitoring,
recording, and input spectrum are all disabled; input probe meters continue to
run. Release builds use the optimized profile below. The x64 local-ASIO package
build completed successfully in 8m14s with SHA-256
`97F6FC253AF76FBFC9BF630C939F6D541C45F886AF41AC6ECB14A419472802DE`. This is
build evidence only; no runtime CPU comparison or listening result has been
collected yet. The renderer load metric excludes ASIO input conversion/copying
outside the playback renderer, so it is an indicator rather than a full device
callback measurement.

### Phase 2 — Free wins (R2, R3)

- Release profile: `lto = "fat"`, `codegen-units = 1`, `opt-level = 3`, `panic = "unwind"` preserved. Target features per decision 1.
- Clamp WASAPI render fill to one period, pad with silence, count the underrun.
- Exit: measurable load reduction from the build change alone; A5 passes.

### Phase 3 — Block-first rendering (R4, R5, R6)

- Plan construction precomputes, per clip: integer `[start_frame, end_frame)`, source offset in frames, and a fade/gain envelope evaluated per block rather than per sample.
- Per-track monotonic active-clip cursor. Cost becomes O(active clips); a track with 5,000 clips off the playhead costs nothing.
- Block reads from streamed sources: one acquire per block, not one seqlock per sample.
- Block-split at loop/skip boundaries and at clip boundaries inside the block, preserving current semantics exactly.
- Hoist per-sample atomics: `MixControl` targets per block into a ramp; `ChannelProcessor` revision check per block; `record_spectrum` enabled flag per block; group routing flattened to an ordered node list.
- Exit: A1 ratio ≤ 1.5×, A2 equivalence, A3 zero allocations.

### Phase 4 — Unify renderers (R7)

- One block renderer for plans with and without hosted inserts. Retire the per-sample path.
- Confirm A2 still holds on both plan shapes.

### Phase 5 — Fault containment (R8, R9)

- `catch_unwind` at the callback boundary; NaN/inf sanitizing; FTZ/DAZ around plug-in calls.
- Per-insert watchdog with auto-bypass and a red insert badge that persists until the user re-enables.
- Device-loss detection and the recovery ladder, with a visible message.
- Exit: A6 and A7 pass.

### Phase 6 — Quality (R10)

- Replace the hard clamp with soft clip; add an opt-in look-ahead true-peak limiter whose latency joins PDC and is visible.
- Decide per-insert oversampling separately, with its latency accounted in PDC. Neutral by default.
- Exit: user listening review; no default CPU cost when unused.

## Explicit non-goals

- No global plug-in sandboxing. Crash isolation stays per-plug-in and opt-in, because a global sandbox trades away the small-buffer latency this engine currently wins on.
- No track-level parallelism in this packet. It depends on phase 3 and is only worth it above roughly 16 tracks; it belongs in its own packet with its own determinism check.
- No hand-written SIMD before phase 3. The algorithmic complexity is the larger problem.
- No framework, dependency or codebase-wide refactor.

## Handoff

- Actual implementer/reviewer model: Codex; phase 1 in progress by user request.
- Source identity (commit plus dirty diff/artifact identity) and build hash: commit `506c139` plus ASIO idle-input bypass and release-profile changes; x64 local-ASIO package SHA-256 `97F6FC253AF76FBFC9BF630C939F6D541C45F886AF41AC6ECB14A419472802DE`.
- Changes and reasons: callback timing and overrun visibility; defer plan rebuilds during audio-event drags; skip unused Input Bus DSP in the ASIO capture callback; enable optimized release settings.
- Commands/checks and actual results: `cargo check --offline -p opentrack --features local-asio` passed; x64 local-ASIO package build passed in 8m14s. Runtime measurement remains pending.
- Runtime environment and evidence paths: Windows x64; fixtures to be created under the audio crate's test suite, not in `builds/`.
- Ledger IDs updated; report command/result: pending.
- Review findings, rework, elapsed effort if available: no runtime measurement yet.
- Failures/unknowns/pending acceptance: A1 baseline unmeasured; current ASIO DSP load and overruns unknown; callback load does not include all host-side ASIO work.
- Next ready task: run the new build during the user's typical playback session and capture the DSP/overrun status, then continue phase 1 profiling.
