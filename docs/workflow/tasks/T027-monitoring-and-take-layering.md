# T027 — Monitor-only playback and take layering

- Status: review_pending
- Scope version and requirement IDs: audio-workflow-v11; `parameter-change-click-reduction`, `monitor-exclusive-track-playback`, `overdub-topmost-take`.
- Desired observable outcome: Parameter changes during playback avoid audible discontinuities; a monitored track outputs its selected live input without also playing that track’s recorded clips/instrument; overdubs stay as separate events while only the newest/topmost take is displayed and heard in overlap. Intentional X-crossfades remain supported. Dedicated take-lane controls are deferred.
- Reference evidence: User-specified behavior; no additional Cubase inspection or reference claim.
- Dependencies and decisions needed: Existing track monitor/input selection, take append order, clip painting order, and intentional crossfade metadata.
- Code scope and architecture boundaries: `opentrack-audio` callback/runtime playback plan transition, per-track mix policy, and Project recording-preview paint order. Keep clip/source persistence and undo intact. Audio callback remains bounded and lock-free.
- Acceptance checks: Build and open one app instance; with other tracks playing, enable monitoring and confirm only that track changes to live input. Disable monitoring and confirm recorded audio returns. Record another take over an existing event; confirm new event paints on top, the overlap plays only the new take, and both takes remain available in project state. Confirm explicit X-crossfade still blends. During playback, adjust fader and EQ repeatedly and listen for clicks, gaps, or level jumps. Repeat at the target device/buffer settings.
- Affected regressions: Track and master meters/routing, mute/solo, EQ/insert processing, monitored recording, streaming clips, source fades, X-crossfade, take undo/discard, playback-plan replacement and callback retirement.
- Essential gates: Audible live parameter adjustment; monitor-exclusive playback; real record/overdub and take recovery; no callback crash/dropout. Headless tests do not pass these gates.
- Review owner and milestone stop: User listening/review; stop here before take-lane editing or a new audio-editing milestone.

## Handoff

- Actual implementer/reviewer model: Codex current session; no independent review claimed.
- Source identity: Dirty working tree; x64 build `A80F304B4504614D4AF7B80D8ADBFD365EDE2C8F9BC0D1E723D2A1A8574CBA7B`.
- Changes and reasons: Playback-plan replacements crossfade old/new DSP output over 10 ms to reduce steps when rebuilt fader/EQ/insert state changes. Monitor selection suppresses that track’s clip and instrument playback while preserving live input and track DSP. Overlap mixing selects the latest active event, except a deliberate fade-out/fade-in X pair. Recording preview paints after arranged clips.
- Commands/checks and actual results: `cargo fmt --all`; `cargo test --release --offline -p opentrack-audio --target x86_64-pc-windows-msvc` — 44 passed; `cargo test --workspace --release --offline --target x86_64-pc-windows-msvc` — 95 app, 44 audio, 16 core passed; `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64` — passed; `git diff --check` — passed (Git reported expected line-ending conversion notices).
- Runtime environment and evidence paths: Windows x64 release. OpenTrack was absent before build and remains closed. No project or hardware was opened. No live audio or visual capture.
- Ledger IDs updated; report command/result: Added three source-complete/runtime-pending requirements and listening gates in `audio-workflow-v11`; regenerate with `python scripts/parity_report.py`.
- Review findings, rework, elapsed effort if available: First targeted run exposed a too-short synthetic source in the overlap fixture; corrected. Full suite passes. No audible acceptance.
- Failures/unknowns/pending acceptance: Confirm real fader/EQ smoothness; verify monitoring route and input selection in the GUI; record real overdub to verify top display/playback and retained undo. Meter transition response and complex DSP state under rapid edits need listening. Whether all processor-state changes benefit from the same 10 ms linear crossfade remains a listening question.
- Next ready task: User live review, then plan pitch/time detection and non-destructive note editing separately from audible correction DSP.

## User review

Open the rebuilt app once with a disposable project. Start playback on two tracks. Toggle monitor on one track and confirm only its live input is heard while the other continues. Turn monitor off and confirm that track’s clips resume. Record an overdub over an existing event, stop, and verify the latest event draws on top and alone plays in the overlap; audition an intentional X-crossfade too. Undo/redo the new take. Finally adjust gain and EQ several times while playing and listen for clicks or dropouts. Close OpenTrack when finished.

## Correction after user review — missing ASIO backend

The user reported WASAPI as the only backend, no monitoring audio, and failure to start playback. The A80F304B build was mistakenly built without `local-asio`. Startup intentionally retains the saved ASIO endpoint; that build cannot open it, leaves the audio engine absent, and Play retries the same unsupported endpoint. This explains the reported combination at source level; no live hardware reproduction is claimed.

Sol 6.1 corrected the build script to retain the previous backend feature choice per architecture in `build-info.json`, including migration from existing local-ASIO notice files. `-LocalAsio` explicitly enables the adapter; `-LocalAsio:$false` explicitly disables it. Fresh builds retain the existing WASAPI-only default. README and implementation workflow now document the feature selection. No saved endpoint or project settings were changed.

Checks: all 47 audio tests pass with `local-asio`, including shared ASIO capture/monitor/recording and bus-processing tests. The user closed OpenTrack before rebuilding. Standard build command reports local ASIO enabled. Audible monitoring/playback and installed-driver selection remain pending review of the corrected executable.

Corrected x64 local-ASIO executable SHA-256: `DCAF34686CCC5CC61B17E3F1482304E1875673E9575A2A766E63C76D1BB1AE50`. A second standard build retained `local_asio: true` from the build record and produced the same hash. OpenTrack remains closed; no hardware stream or project was opened.
