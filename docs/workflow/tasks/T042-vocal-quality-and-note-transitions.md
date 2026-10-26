# T042 - Vocal pitch quality and touching-note transitions

- Status: awaiting_user_review. Batch 1 accepted by the user on 2026-10-25. The user reported that the previous build still showed harsh pitch spikes; the latest correction is built but not yet reviewed. Actual implementer: GPT-6.1 Sol; no delegated or independent review.
- 2026-10-25 follow-up: a source audit localized a hard wet-to-dry step at edge-only transition endpoints. The endpoint crossfade, short audio handoff and one-unchanged-note boundary rule are implemented with sample-continuity tests. A dedicated note-drag audition voice and track-color cohesion were also added to this checkpoint; their listening/visual review is pending.
- Scope version: `transport-v16-pitch-time-editing-workflows`; requirements: `pitch-processing-quality`, `pitch-note-transition-blend`.
- Desired outcome: reduce reported bubbly/poppy pitch artifacts and blend touching edited note regions automatically, while retaining live nondestructive editing, musical pitch, timing, stereo behavior, original media, bounded callback work and no real-time allocation/I/O. Leave genuine gaps alone and keep Project event structure unchanged.
- Reference evidence: no Cubase resynthesis or transition details are inferred from the SE-02–SE-06 stills. Audio quality and adjacent-note blending are user-specified. No Cubase interaction or new reference capture was performed.
- Controlled source: read-only `C:\Users\Samue\Documents\EXAMPLE\Audio\Recordings\Vocal 01.wav`, SHA-256 `6457409352a59f3ba289fefbbe3dda95117c2c49f15bf38542fd291ef6bb07f0`; 48 kHz mono 24-bit PCM, 807,936 frames / 16.832 s. The existing EXAMPLE OPR was read but not modified. Its Vocal 01 event uses source offset 1.6 s, duration 15.232 s, saved 16 kHz analysis (1,524 points) and 22 note regions. All generated float WAVs are in ignored `builds/x64/T042-audio-comparisons/`.

## Implemented

- Added one shared Rust `PitchSynthesis` prepared plan used by live playback and the offline WAV renderer. Grain planning and allocations occur before callback playback; per-frame sampling reads immutable prepared arrays and does no allocation or I/O.
- Interpolated voiced F0 points in log-frequency rather than stepping every 10 ms. Confidence now fades to zero at detected unvoiced points so consonant frames return to the source signal instead of inheriting a pitched fallback.
- Initial T042 implementation crossfaded two note outputs over 10% of each note, capped at 30 ms. That version is superseded by the latest iteration below; its measurements are historical and are not evidence for the current renderer.
- Added coverage for interpolated F0/voicing fade, bounded touching transitions (including unequal note durations and full-window extension), untouched gaps, source pass-through outside an analyzed range, stereo pitch/timing, cancellation and source preservation.

## Latest iteration after user listening feedback

- The user-provided OpenTrack screenshots `Screenshot 2026-10-25 213249.png`, `213304.png` and `213317.png` showed abrupt pitch-trace jumps at adjacent note bounds. The later `edit.png`, `should not be to note center- should be to the actual note tails.png` and `unedited.png` screenshots identify that boundary pitch must follow actual trace tails and neighboring notes should share the slide. These are current-app evidence, not Cubase reference captures.
- Pitch targets now ease inside each pitch-edited note toward a shared target for its touching neighbor. The ramp reaches no farther than one sixteenth-note at the current project tempo, and is shortened on short notes so its two edges do not overlap. If both notes are pitch-edited, they meet at the midpoint of their edge targets; if one is unchanged, the changed note eases to the unchanged note's pitch.
- The Sample Editor pitch trace now uses the same neighbor-aware contour as playback instead of applying each note's pitch shift independently. During time-compressed notes, grain windows expand adaptively to keep adjacent grains overlapping, and the renderer favors voiced grains over the faster, pitch-raised dry signal. These are source-level corrections only; audible improvement is not yet verified.
- The shared boundary pitch now comes from analysis near the actual source tails instead of `detected_midi` note centers. An unedited note beside a pitch-edited note now renders only its edge ramp, so both notes meet at the same pitch while the unedited body remains source audio.
- Live processing and the optional offline renderer receive project tempo so the sixteenth-note limit follows the session tempo. The legacy offline API keeps a 120 BPM default; a tempo-aware API is also available.

## 2026-10-25 crackle correction and audition

- Audit finding: with one edited note touching an unedited neighbor, the unedited side was synthesized only for its pitch-transition edge, then switched to dry audio with no interpolation at the inner endpoint. A synthetic 220 Hz test produced a 0.55 sample step against a 0.035 natural source step.
- Correction: edge-only transition endpoints now crossfade wet-to-dry over 5 ms. The audio handoff itself is a 5 ms crossfade around the boundary while the long pitch ramps remain; the previous implementation mixed two independently synthesized voices across the whole ramp, leaving a full-level two-pitch interval.
- Boundary rule: when one note is unchanged, both sides resolve to the unchanged note's edge pitch; two edited notes still meet at their midpoint, matching the documented behavior.
- Tests: `edge_only_transition_endpoints_do_not_step_from_wet_to_dry` covers both orientations and was added before the fix so it failed at 0.55 then passed. `pitch_ramp_does_not_blend_two_voices_away_from_the_boundary` proves a single voice away from the boundary. `boundary_pitch_prefers_the_unchanged_neighbor` covers both unchanged-side cases and the both-edited midpoint.
- Drag audition: a dedicated `NoteAudition` voice loops the held note's current pitch/time source directly to device output, with 5 ms fade-in and source-replacement crossfades. It works with transport stopped, does not move the playhead, and does not rebuild the playback plan. Editor and audio tests cover the request lifecycle, audible stopped-transport output, and replacement continuity.
- Note colors: unselected notes now use the rounded current semitone row; selected notes use the track color. The Sample Editor and editing suites pass.
- Track-color cohesion: Project/Mixer/Channel Settings meters, EQ parameter bars, curve/nodes and owned spectrum inherit channel color; Stereo In/Out default to blue and expose the palette selector in the Mixer footer.

## Historical first-pass comparison findings

Baseline and candidate used the same Vocal 01 source, saved analysis, 48 kHz sample rate, mono channel, frame count, note IDs and edit parameters. Baseline artifacts were retained; candidate artifacts use the final shared renderer. The source file hash was rechecked and is unchanged.

- Source reference: baseline/candidate SHA-256 `36dc4f70b16c2ea115ef11764ecc45e1c3e66c6579ea781ec1128e7551dd4435`.
- Single note ID 16, +3 semitones: target-pitch median/p95 error was baseline 3.03/16.97 cents and candidate 3.58/17.91 cents. These pitch metrics are similar; they do not establish an audible improvement.
- Touching stress pair IDs 4/17, +2/-2 semitones at absolute source time `4.4766316 s`: within ±10 ms, peak sample step fell from 0.014161 to 0.008700 (38.6%); RMS sample step fell from 0.003037 to 0.002322 (23.5%). Peak/RMS levels remained effectively stable: baseline 0.154190/0.023444; candidate 0.154190/0.023367.
- The existing pitch-analysis metric reports a 1.9 k-cent outlier for the left stress note at 4.10 s and an octave-low outlier for the right note at 4.51 s. A separate short-window autocorrelation probe of the candidate audio measures about 320–324 Hz vs 329.8 Hz at 4.10 s and about 248 Hz vs 247.6 Hz at 4.51 s. The built-in analysis appears to select a subharmonic at these points; this does not prove the perceptual tone is correct. Treat vocal quality and note transitions as awaiting the user's listening review.
- Artifacts: `baseline/` and `candidate/` each contain `reference-source.wav`, `single-note-plus-3st.wav`, `all-notes-nearest-semitone.wav`, and `touching-notes-stress-plus2-minus2st.wav`. `measure.py` reproduces hashes, level and seam-step comparisons.
- Output SHA-256 pairs (baseline / candidate): reference source `36dc4f70b16c2ea115ef11764ecc45e1c3e66c6579ea781ec1128e7551dd4435` / same; single-note +3 `5768cf65dc06fa08d50abbe392400069f921801147535a27685a0e74733ae1b3` / `2d18a4b25f185e7af7c7eaa98e25674120f35876a82c8b20d05f0ce12448ecc3`; all notes nearest semitone `edbcffab9283e3698338252f946f72eb6485e05af5981750c21879b499e1125f` / `af4f7b525d0c3800bd063eee499f123250ef11265a78715d7cf8969db10c3778`; touching stress `63e8c0e38dcd4fe0fed2992b67ada26b3920200becd01d03e47ab92a97b2c324` / `5d311174c1d8eacc08e9141a0548f303ce12466b319591f6c5f67cb0640c28a`.

## Verification and review gate

- `cargo fmt --all` completed successfully.
- The current x64 local-ASIO release build completed successfully with WASAPI retained. Executable SHA-256: `AE2E6CCCA4DC6ACD70BF0842A00F23CE849E5C12BF73A26682751C758C677845`. OpenTrack was checked closed before building and remains closed.
- Full workspace tests pass: app 131, audio 63 (1 ignored manual comparison), core 17, plugin foundation 15+11. No generated-audio comparison or live listening check was run for this iteration. The user will review with their own audio. Audio quality and the updated note transitions remain unverified; the previous renderer's comparison metrics do not establish the current behavior.

## Handoff

- Batch 1 T041 acceptance is recorded in ledger evidence `t041-user-acceptance` / `t041-live-review`.
- Batch 2 source corrections and x64 build are ready for the user's own audio review. The previous seam metrics are historical only. Do not start Batch 3 until the user accepts this audio review.
- EXAMPLE OPR and source WAV remain untouched. Original Rust architecture, live nondestructive processing, local ASIO, WASAPI, accepted Mixer and unrelated work remain protected.
- Stop at this listening checkpoint. After user feedback, address any audio-quality corrections before deciding whether the Batch 2 gate passes; only then begin Batch 3.
