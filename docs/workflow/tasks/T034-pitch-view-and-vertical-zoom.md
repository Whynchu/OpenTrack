# T034 — Pitch view and vertical zoom

- Status: review_pending; implementer GPT-6.1 Sol, no subagents.
- Scope: transport-v9-pitch-time-v2; pitch-trace-preview and pitch-vertical-zoom.
- User evidence: current OpenTrack capture `Screenshot 2026-10-25 155531.png` (1917 × 453) confirms the trace is visible on a real take; this is not detector accuracy validation. SE-05 `090626.png` (1878 × 442) establishes the separate keyboard, semitone rows, subdued waveform and clear pitch contour. Display scaling and surrounding window positions are unknown in these cropped images.
- User direction: add the lower-right vertical zoom for note-row height, retain existing horizontal navigation gestures, and improve this view's organization/readability.
- Changes: dedicated piano rail; black/white note-row shading; pitch view centered on the median voiced pitch; continuous vertical zoom over 6–72 semitones; lower-right +/- and drag slider; pitch-range scrollbar and keyboard pan; less prominent waveform; clipped single footer status. Display preferences persist separately from project edits. No new horizontal zoom bar.
- Boundaries: app display/view code and preferences only. Detector, waveform source data, event source ranges, playback and musical edits are not changed by the view controls. No note blocks or correction controls without real segmentation/rendering.
- Acceptance: vertical zoom increases/decreases semitone row height around the same center; keyboard drag and right scroll expose high/low pitches; waveform and timeline retain existing horizontal time coordinates; source trace is clipped, not clamped to the view boundary; zoom/pan do not dirty the project. View settings survive restart and invalid saved values are sanitized. Existing editor cuts, grid, transport, cycle/skip and mono display regressions must pass.
- Review: user checks the rebuilt x64 app; runtime visual comparison and hardware playback remain pending until observed.

## Handoff

- x64 build SHA-256: `47E12607B75E15C85F2225ECAE4B6590E3A27AD7FCEE197131F3F5EA0A880427`; local ASIO confirmed enabled in build-info.json. Built with `./build.ps1 -Architecture x64 -LocalAsio`. OpenTrack was closed before replacement and remains closed.
- Eight targeted release editor tests pass: zoom coordinate anchoring/sanitization, actual right-edge slider/scroll pointer gestures and view serialization, active pitch display at 6/24/72 semitones without project edits or eager analysis, mono/shared cursor, cuts, independent snap/navigation, and cycle overlays.
- Formatting and diff checks pass. Ledger/report validated with `python scripts/parity_report.py`.
- Scope `transport-v9-pitch-time-v2`: source coverage 75.0%; unknown reference coverage 86.5%; current full runtime/parity workflows remain pending. These figures include the prior transport scope and unfinished pitch/time features; they are not whole-DAW completion.
- Analysis remains session-only; note regions and audible pitch/time transformation remain outstanding under T030.

## Coverage and review evidence

| Evidence | Display / placement | Panels / theme | Action / source | Match status |
| --- | --- | --- | --- | --- |
| User T033 screenshot 155531 | 1917 ? 453 crop; scaling/window position unknown | Sample Editor; dark OpenTrack palette | Prior Pitch & Time analysis; sample_editor.rs | User observed trace; layout needed refinement |
| SE-05 screenshot 090626 | 1878 ? 442 crop; scaling/window position unknown | Sample Editor with pitch view; reference theme | Piano rail, row grid, trace/waveform and right-edge controls | Primary visual specification; exact gestures unmeasured |
| T034 build | Same-display capture pending | New piano/trace view and vertical zoom | sample_editor.rs and sample_pitch_view.rs; lower-right +/?/slider, pitch scrollbar, piano drag | Source and headless interaction checked; live appearance pending |

Unchanged supplied PNGs are archived outside Git at `G:/dev/Opentrack/_analysis-tools/reports/opentrack-comparisons/T034-pitch-view/`. No new OpenTrack screenshot or reference-app interaction was performed in this slice.
