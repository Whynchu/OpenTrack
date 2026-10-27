# Historical transport-panel study: T029

This note records an earlier feature-specific layout study. Its screenshot archive has been removed at the user's direction; it is historical context, not an active comparison or acceptance requirement. Current behavior is reviewed in OpenTrack.

| Area | Reference structure | Current source | Review status |
| --- | --- | --- | --- |
| Left edge | Narrow meter pair, recording modes and Auto row | transport_panel.rs, x18..127; live input telemetry | Source arrangement updated; screenshot pending |
| Locators | L/R flags, two dark numeric rows with secondary fields | x130..244; shared Project bounds and skip direction | Source arrangement updated; secondary fields remain placeholders |
| Main position | Large musical position beside secondary clock, narrow divider below | x251..520; live shared transport with rounded millisecond clock | Source arrangement updated; screenshot pending |
| Buttons | Previous, rewind, forward, next, cycle, stop, play, record; larger contiguous buttons | x251..520, y39..64; shared vector symbols and existing commands | Source updated; actual pointer/visual review pending |
| Click/tempo/sync | Compact CLICK status above tempo/signature and Sync row | x528..640; CLICK enabled and connected to audio, disabled unsupported tempo/sync modes | Headless click/Control-click tested; listening pending |
| Markers | SHOW/Marker title, 3 x 5 numeric bank | x648..737 | Layout only; marker engine remains absent |
| Right edge | Stereo activity, grid blocks, compact level slider | x746..818; real Stereo Out peaks and shared output-bus gain | Source updated; side-by-side and hardware check pending |

Panel content is 837 x 69 logical points with zero outer inner margin. Actual displayed pixel dimensions depend on Windows/egui scaling and need live capture. Current OpenTrack screenshot, same-display overlay and visual acceptance are pending; no desktop app was opened. Any future captures/comparisons belong under `_analysis-tools/reports/opentrack-comparisons`, outside Git.

The click sound is user-specified, not observed reference audio: original short pop with E-flat 6 downbeat (1244.51 Hz), E-flat 5 remaining beats (622.25 Hz), an inharmonic component and brief deterministic noisy transient, 0.4 ms attack and 24 ms total bound. Its level is independently adjustable. It follows the audio sample clock, enters Stereo Out before master processing, and does not enter capture recording through an internal software route. Physical loopback can of course record whatever the device routes. User listening decides the desired timbre and level.

4/4 is the initial integrated Project-grid target. The engine click math uses numerator/denominator and has a 6/8 synthetic check, but the existing Project ruler uses numerator-based quarter-note bar grouping in many places. Full non-4/4 grid/metronome alignment needs a separate timebase correction and is not accepted by the synthetic check. Precount, configurable subdivisions, separate hardware click output, MIDI click and tempo-map changes remain outside this slice.
