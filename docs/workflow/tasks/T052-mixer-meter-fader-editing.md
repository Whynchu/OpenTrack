# T052 — Mono/stereo metering, compact fader rail and numeric gain entry

- Status: review_pending
- Implementer: GPT-6.1 Sol; source self-review, user visual/runtime acceptance pending.
- User outcome: mono Mixer tracks show one meter lane; stereo tracks preserve the L/R split. A more restrained rail has shorter physical travel with the same dB range. Double-clicking the current gain value opens numeric editing.
- Scope: Mixer and its shared Channel Settings fader. Preserve fader thumb artwork, strip layout, gain taper, +6.0206dB maximum, −30dB finite floor, bottom mute endpoint, fine Shift drag, unity reset, peak reset and track gain persistence/undo path.

## Changes and semantics

- Meter rendering takes the track's channel count. Mono uses one full-width lane displaying the maximum of the two post-processing meter values, with one full-width held-peak marker. Stereo uses two independent lanes and a 1px separator. Existing Project meter wrappers retain their behavior.
- The fader slot is a dark inset well with rounded ends and paired calibration ticks. Travel is centered in the existing area, inset from both ends and capped at 240 logical pixels. All pointer mapping and ticks use the same rail rectangle. The existing thumb is retained.
- Double-click the gain readout below the peak value. Enter or focus loss commits valid input; Escape cancels. Numeric input accepts an optional `dB` suffix and `-inf`/`−∞` for mute. Entered finite values clamp to the existing finite fader range. Invalid/nonfinite input keeps the edit active without changing gain. An unchanged legacy low-gain value is preserved on focus loss.
- Draft text is temporary UI state keyed to the channel. Commit updates the existing `Track::gain`, so the current audio consumer, project save, dirty state and history paths remain in use.

## Verification and handoff

- Acceptance checks: lane count and stereo separation; shorter rail endpoints retain the full gain range; normal/Shift drag; actual pointer double-click followed by Enter, Escape or invalid text; numeric range/mute parsing; existing Mixer bus edit targets.
- Checks: five Mixer checks, four meter checks and the existing gain-taper check pass (10 total). They cover actual double-click/text/Enter/Escape/Tab events, invalid input, unchanged legacy values, finite/mute bounds, mono/stereo painted lane geometry, normal/fine drag and bus edit targets. Formatting, whitespace and parity ledger/report validation pass.
- Build: x64 local-ASIO release succeeds; SHA-256 `7E7D3BF349F418ECD9528734CAD0FC58714655FF66B7570AD8923EFD498983D9`. Five existing dead-code warnings remain.
- Ledger: `mixer-fader-and-track-selection` extended with the current user requirements and source/check evidence; runtime remains pending. `scripts/parity_report.py` regenerated metrics.md successfully.
- Source base: `a097613` plus this task diff. No engine or project schema changes.
- Live meter appearance, audible gain and numeric editor focus review remain pending the user's running-app review.
- Earlier "Mixer highlight" change removed only the bright bevel stripe on small control buttons; active state colors and meter/peak indications were retained.
