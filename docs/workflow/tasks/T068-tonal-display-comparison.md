# T068 - Tonal display comparison with Tonal Balance Control 2

- Status: implemented (first pass); running-app review pending.
- Scope version: 1.
- User observation (2026-11-01): Tonal Balance Control 2's frequency display looks different from OpenTrack's tonal view. The user wants to use it as a comparison to shape our own tonal representation so it behaves the same way.
- User decisions (2026-11-01): adopt the broad target bands, the smoothing and the weighting/tilt; keep OpenTrack's frequency span, line/target style and sizing.

## Scope

Compare the same source material in OpenTrack's Channel Settings tonal view and Tonal Balance Control 2, and record concrete differences before changing anything:

- frequency-axis mapping, labels and span (log frequency, 20 Hz - 20 kHz vs a narrower musical range);
- band count, band edges and smoothing (TBC2 uses broad target bands over a smoothed spectrum);
- level scale and tilt/weighting (the tonal balance reference is level-weighted, not a plain FFT magnitude);
- target vs measured presentation (single curve, filled difference, or band-by-band);
- how the display holds/averages over time and how it reacts to silence;
- line style, fill and colour.

Then decide which of those behaviours our tonal view should adopt, keeping OpenTrack's own colours, artwork and implementation.

## Delivered (first pass)

- **Broad target bands**: the target curve now paints four labelled regions - Low (20-200 Hz), Low-Mid (200 Hz-1 kHz), High-Mid (1-5 kHz) and High (5-20 kHz) - each as a translucent band spanning +/-3 dB around that region's average target level. The precise target curve still draws on top, in the existing orange.
- **Smoothing**: tonal curves are now smoothed over roughly a third of an octave (Gaussian radius 5, sigma 1.9 at the 96-band resolution) instead of about a quarter octave. Persisted analysis and targets keep their full 96-band detail.
- **Weighting/tilt**: the existing display tilt control (0/3/6 dB per octave, display-only) now also applies to the tonal curves, pivoted at 1 kHz, so a balanced mix can read flat. The measured-vs-target difference view is unchanged because the same weighting cancels on both sides.
- Frequency span, colours, line weights, graph size and layout are unchanged.

## Constraints

Original implementation only: no copied TBC2 assets, code or preset data. Studying the installed plug-in for a specific comparison is allowed because the user requested it. Our existing tonal targets (version-1 24-region, version-2 48-region and version-3 96-region profiles) must remain readable and comparable.

## Checks

- `tonal_weighting_is_zero_at_one_khz_and_tilts_symmetrically`: weighting is centred at 1 kHz, boosts lows and cuts highs, and scales with the tilt setting.
- `tonal_band_index_covers_the_audio_range_and_one_khz_sits_above_mid`: band mapping covers 20 Hz - 20 kHz.
- Existing tonal smoothing, target aggregation and OPR round-trip checks remain green.
- `cargo test -p opentrack --bin opentrack` passes 197 with the same 14 pre-existing ruler/sample-editor failures.

## Acceptance

- A written comparison with the same source in both tools (still to record).
- The chosen differences implemented in OpenTrack's tonal view, with the same source showing the intended shape.
- Existing tonal-target round trips still pass.
- User review in the running application.
