# T054 — Realtime control performance, Stereo Out VST3 and startup splash

- Status: performance_user_accepted_stereo_out_runtime_review_pending_followup
- Scope version and requirement IDs: v1; user request (2026-10-27).
- Desired observable outcome: fader and EQ manipulation remains smooth during playback without static, dropouts or substantial slowdown; Stereo Out supports loadable/processable VST3 inserts like other channel strips; startup shows a stable centered splash at approximately 600×327 logical pixels for at least two seconds while initialization completes, preventing a white flash.
- Reference evidence: user report of static/slowdown during fader/EQ work; user report that Stereo Out VST inserts stub out; supplied 1920×1080 desktop capture shows a centered splash of approximately 600×327 pixels. Splash measurement is visual estimate from the supplied image.
- Dependencies and decisions needed: none. Keep current mixer/channel strip controls and accepted T053 group editing behavior. Preserve existing local-ASIO/WASAPI options and hosted VST3 routing semantics. The two-second splash is a minimum display duration, not a fixed delay before initialization begins.
- Code scope and architecture boundaries: playback-plan publication and control edits in `opentrack-app`/`opentrack-audio`; VST3 bus instance lookup, serialization, hosted processing and editor/parameter dispatch for the existing output bus ID; existing startup splash timing, sizing and first-frame initialization. Keep callback work bounded and allocation/lock free. Avoid rebuilding full source/clip/DSP state for every continuous fader/EQ pointer update; preserve EQ filter continuity and smooth gain changes.
- Acceptance checks: (1) During audio playback, continuously drag faders and EQ gain/frequency controls: no audible static/clicks/dropouts and no noticeable UI or playback slowdown; inspect callback underruns/CPU behavior before and after where available. (2) Add a VST3 insert to Stereo Out, load its native editor/parameters, hear its processing on the complete output mix, and verify bypass/removal/save-reopen. (3) Launch from a cold start and confirm the centered splash matches the supplied approximate size, remains visible for at least two seconds while initialization proceeds, and no white frame flashes before the application UI. (4) Confirm existing track/group VST processing and local-ASIO playback remain intact.
- Affected regressions: track/group faders and EQ, hosted VST3 instances and parameter/editor windows, Stereo Out strip processing and meter, output delay/latency behavior, project save/reopen, cold launch and app startup.
- Essential gates: audible clean playback through continuous fader/EQ changes; Stereo Out plug-in audio path reaches device output and persists state; stable splash appearance with no launch flash. A compile alone does not satisfy these gates.
- Review owner and milestone stop: user reviews live fader/EQ operation and Stereo Out plug-ins; user reviews cold-start splash. Stop for that review after implementation/build.
- User follow-up (2026-10-27): fader/EQ response was reported as substantially more responsive. Ozone loaded on Stereo Out, but its editor display appeared idle unless the pointer moved; splash isolation and loop pencil orientation are tracked in T055.

## Handoff

- Actual implementer/reviewer model: GPT-6 Luna; independent review pending.
- Source identity (commit plus dirty diff/artifact identity) and build hash: in progress.
- Changes and reasons: in progress.
- Commands/checks and actual results: in progress.
- Runtime environment and evidence paths: Windows x64; user-supplied splash screenshot is the task reference.
- Ledger IDs updated; report command/result: pending.
- Review findings, rework, elapsed effort if available: pending.
- Failures/unknowns/pending acceptance: pending runtime and user review.
- Next ready task: user review of the completed build.
