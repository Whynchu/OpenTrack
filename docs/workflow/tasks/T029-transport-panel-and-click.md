# T029 — Compact transport and pop click

- Status: implementation complete; live visual/listening review pending.
- Implementer: GPT-6.1 Sol; no subagents or independent review.
- User request: tighten Transport to workflow-L1/02-mixer-transport-1.png and enable a pop-style E-flat accent/lower-octave click. Then prepare analysis work from supplied Sample Editor screenshots.
- Evidence: direct inspection of primary 837 x 69 transport crop; source coverage/unknowns in transport-panel-study.md. Sound follows explicit user direction; no reference-audio parity claim.
- Code boundaries: new app transport view, original audio metronome module, existing playback-plan/output mix, global app preferences and Transport command/menu. No new dependency, project schema, bundled effect or pitch processor.
- Acceptance: compare entire transport at the reference display conditions, verify group density and right/left edges; use CLICK and C, Control-click Setup/menu Setup, adjust click volume. Play/record at multiple tempos, loop and skip through boundaries, stop/start/seek and change audio rate/buffer; hear higher E-flat downbeat and octave-lower pop beats. Confirm no unwanted click in recorded input, shared Stereo Out gain/mute, unchanged Project dirty state on click toggles, and retained preferences after restart. Exercise existing transport and Channel Settings controls.
- Scope limits: marker bank and record/sync/tempo modes remain disabled; no precount/MIDI click or separate click output. General non-4/4 Project-grid mapping remains a known gap. No live acceptance inferred from source/tests.
- Review stop: user visual/listening check before another substantial feature; T030 is preparation only.

## Implementation

Extracted the panel from app.rs into transport_panel.rs. Replaced sparse undersized button text with larger grouped vector transport symbols; tightened record/locator/clock/click/marker/right-edge geometry to the reference. Displays real input/output meter levels. The compact output slider edits the same Stereo Out gain used by the Mixer; changes use ordinary project dirty/undo handling.

CLICK, Transport menu and existing remappable C command share metronome state. Control-click opens Setup with enable and level. Enabled/level persist globally; toggling click alone does not edit the project. Audio plan carries the click configuration, and the callback computes short pop samples against the wrapped/skipped timeline frame. Click joins the common external input before Stereo Out DSP and gain. No audio callback allocation, locks, files or UI timers were introduced. Disabling click/stopping yields silence from it.

The sound is an original 24 ms damped E-flat body plus inharmonic/noisy transient, rather than a sustained tone. Initial octave selection is E-flat 6/E-flat 5; the user supplied pitch class/octave relationship, not the absolute octave, so this is an explicit implementation choice for listening review.

## Checks and handoff

Synthetic checks: duration/accent/silence at 44.1/48/96 kHz; fractional tempo and 6/8 beat math; block-size invariance through loop wrap; master gain/mute and disabled/stopped silence. Headless egui CLICK/Control-click checks verify shared state, Setup and unchanged Project/no device opened. Existing full release suite and x64 local-ASIO build are required before handoff. First added UI fixture used an obsolete RawInput modifier field; corrected to the installed egui event API.

No OpenTrack GUI, Cubase, EXAMPLE or hardware stream was opened. User closed the app before build. Current screenshot/overlay and audible quality remain pending. Exact build/check identity will be recorded in current.md and ledgers. Publication is not requested for this slice.

Final checks: all164 local-ASIO release workspace tests pass (98 app,50 audio,16 core). `build.ps1 -Architecture x64 -LocalAsio` succeeds. EXE SHA-256 `5F27B9C7D4F3B9EE91388A4D19D62E1CFA7133AF454655369AEF811FA427DA94`. Live visual/audio review remains pending.
