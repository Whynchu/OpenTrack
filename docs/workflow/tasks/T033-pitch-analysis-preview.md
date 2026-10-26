# T033 — On-demand pitch analysis preview

- Status: review_pending; source/build complete, runtime and user acceptance pending.
- Scope version: transport-v8-pitch-time-v1; requirements: `pitch-analysis-on-demand`, `pitch-trace-preview`, `pitch-time-editor`.
- Implementer: GPT-6.1 Sol.
- Desired outcome: Clicking Pitch & Time for the current audio event starts real, cancellable, offline pitch analysis. Ordinary import, playback, project open, and Sample Editor open remain analysis-free. Show detected voiced frames over the source waveform; no audio is altered.
- Reference evidence: user-supplied SE-02 through SE-06 captures, archived in `_analysis-tools/reports/sample-editor-2026-10-25/`; trace, piano axis, colored regions, and local time ratios are observed. Their exact gestures, processing algorithm, correction audio, and persistence semantics remain unknown.
- Architecture: Rust `opentrack-audio` owns the isolated WAV decode and YIN-style F0 calculation. The app owns an explicit worker request, cancellation, event identity checks and drawing. No callback work, model runtime, new dependency, cache, schema edit, or fake note segmentation.
- Implementation: mono downmixed analysis view at 8 kHz; 1024-sample windows, 400-sample hop, 65–1000 Hz search range; timestamped confidence-bearing F0 points; unvoiced frames break the trace. Result remains transient to this Sample Editor session.
- Acceptance: open an event, confirm no automatic analysis, click Pitch & Time, observe a real trace on pitched mono audio, confirm unpitched sections leave gaps, cancel a long event, switch event during work, and confirm stale results do not appear. Reopen the same event and verify results are not silently regenerated.
- Regression and missing gates: Project/Mixer/audio behavior; output build retains local ASIO. Persisted analysis attachment, cache identity, undo/save/reopen, event note regions, editable pitch and time anchors, and audible resynthesis are not implemented. This preview is not a correction tool and has not been runtime or accuracy validated.
- Checkpoint: x64 local-ASIO build succeeds; OpenTrack stayed closed. No tests were added or run. Ask the user to review this build before beginning note segmentation, time correction, or rendering.

## Handoff

- Source: dirty working tree; see `git status`. Executable SHA-256: `EA6189FE5C4375C7322F582EB91F2FDAECB76C61158FBC5245E6997873500448`.
- Build: `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64 -LocalAsio`; build metadata confirms x64 and `local_asio: true`.
- Other checks: `cargo fmt --all -- --check`, `git diff --check`, and `python scripts/parity_report.py` pass. Workspace release compile passed. No tests or desktop interaction performed.
- Next: user checks the analysis action/trace on a disposable monophonic voice or single-note source. Then prioritize persistent shared analysis/cache identity and note-region editing; time-map editing follows as its own real-audio slice.
