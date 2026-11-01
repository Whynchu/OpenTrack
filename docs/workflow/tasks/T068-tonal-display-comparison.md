# T068 - Tonal display comparison with Tonal Balance Control 2

- Status: proposed; design input requested.
- Scope version: 1.
- User observation (2026-11-01): Tonal Balance Control 2's frequency display looks different from OpenTrack's tonal view. The user wants to use it as a comparison to shape our own tonal representation so it behaves the same way.

## Scope

Compare the same source material in OpenTrack's Channel Settings tonal view and Tonal Balance Control 2, and record concrete differences before changing anything:

- frequency-axis mapping, labels and span (log frequency, 20 Hz - 20 kHz vs a narrower musical range);
- band count, band edges and smoothing (TBC2 uses broad target bands over a smoothed spectrum);
- level scale and tilt/weighting (the tonal balance reference is level-weighted, not a plain FFT magnitude);
- target vs measured presentation (single curve, filled difference, or band-by-band);
- how the display holds/averages over time and how it reacts to silence;
- line style, fill and colour.

Then decide which of those behaviours our tonal view should adopt, keeping OpenTrack's own colours, artwork and implementation.

## Constraints

Original implementation only: no copied TBC2 assets, code or preset data. Studying the installed plug-in for a specific comparison is allowed because the user requested it. Our existing tonal targets (version-1 24-region, version-2 48-region and version-3 96-region profiles) must remain readable and comparable.

## Acceptance

- A written comparison with the same source in both tools.
- The chosen differences implemented in OpenTrack's tonal view, with the same source showing the intended shape.
- Existing tonal-target round trips still pass.
- User review in the running application.

## Open questions for the user

- Which matters most: the broad-band target shape, the smoothing, the level weighting/tilt, or the frequency span?
- Should our tonal view show a filled target area (like TBC2) or keep the current line/target overlay?
- Should the tonal view be tied to the channel EQ graph size, or keep its own proportions?
