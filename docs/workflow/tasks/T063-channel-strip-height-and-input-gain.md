# T063 — shared channel-strip height and input gain

- Status: accepted for the channel-strip visual refinement (2026-10-31); GitHub publication authorized.
- Scope version: 1.
- Desired observable outcome: tall channel strips use the extended fader travel rather than leaving centered gaps; input gain artwork is easier to see and operate in both editors.
- User evidence: `reference/extended bad mixer panel.png` and `reference/good mixer panel.png`. The supplied images are actual app inputs and were not copied or recaptured into the repository.
- R1: remove the fader travel cap while retaining end padding, gain range, mute, Shift precision and numeric editing.
- R2: Channel Settings and Mixer use the same fader renderer and input gain control; preserve routing, channel selection, bypass, record/monitor and bus editor behavior.
- R3: enlarge input gain from 24 to 34 px. Its graphic and hit area may extend into the mono/stereo band, but must not cover that text, the phase button or numeric editor.
- Architecture: bounded Rust UI work in `mixer.rs` and `channel_settings.rs`; no new dependencies, DSP or persistence format. Existing input-gain, phase and fader state remain the processing consumers.
- Acceptance: compare short/tall strips in both views; operate fader endpoints/precision/numeric entry, input gain coarse/fine/reset, routing and bus editor buttons. Preserve existing serialized control behavior. User accepted the visual refinement on 2026-10-31; agent live hardware/playback checks were not performed.

## Handoff

User requested the extended strip match the proportions in `reference/good mixer panel.png`, fixing the centering visible in `reference/extended bad mixer panel.png`, in both Channel Settings and Mixer. The shared fader renderer now uses all available height with its existing fixed end padding; the 240 px travel cap is removed. The existing gain curve, handle and meter/footer positions are retained. Both views now use one shared input-gain/phase/value control, with 34 px artwork and hit area (previously 24 px), partly occupying the mono/stereo band while leaving its text clear. Numeric fields and the knob do not overlap. Per-frame knob dragging, Shift fine adjustment and double-click reset are shared as well. The user SVG itself is unchanged; no schema or DSP changes.

Actual implementer/reviewer: GPT-6.1 Sol, baseline `fbee87f` plus the existing dirty working tree. Seven Mixer checks and 18 Channel Settings checks pass; local-ASIO compile, format and diff checks pass. Reviewed an offscreen raster of the actual Channel Settings and Mixer strip renderers at 430 and 650 px heights. Temporary rendering code was removed; the PNG stays only in `/tmp/channel-strip-height-review.png`. User accepted this channel-strip refinement and requested GitHub publication on 2026-10-31 ("this is fantastic! push to github."). Agent live desktop/hardware checks were not performed. Normal-directory local-ASIO release build passed and is staged at `builds/x64/OpenTrack`, SHA-256 `57d7be7e9678873904bce4ff7ae60bb6546e88292ebf7cad0c4d04b3ff33d0b7`.

## Follow-up: input-stage trim is wired into the engine (2026-10-31)

User report: the input-gain knobs in both channel views "don't work right - neither in function or their LEDs". Root cause confirmed in source: `Track.input_gain_db`/`input_phase_inverted` were editable and serialized, but `PlaybackTrackSettings` carried neither field, so only device buses (`BusRuntime`) applied them. Track and group rendering ignored both controls, which also left the knob's peak lights unresponsive.

Change (actual implementer: DeepSeek V4.1 Flash; baseline `cb5a8c6` plus local work):

- `PlaybackTrackSettings` now carries `input_gain_db` and `input_phase_inverted`; `opentrack-app` passes both when building the plan.
- A one-pole `GainControl` applies the trim (including polarity) ahead of the built-in EQ, inserts, spectrum tap and meter accumulation on both render paths: the hosted-block path (`render_hosted_track_block`, `render_hosted_group_block`) and the per-sample path (`mix_frame_with_policy`, tracks and groups). The pre-EQ level feeding the Dynamic EQ threshold ring therefore includes the trim.
- New `AudioEngine::update_track_input_gain` gives Channel Settings and Mixer a live path, so dragging the trim no longer rebuilds the playback plan per frame. Device-bus trim still rebuilds, because `BusRuntime` bakes its pre-gain at construction.
- `StereoMeter` now also publishes a pre-fader input-stage peak (`publish_input`/`take_input`), accumulated on both render paths. Both channel views feed the input-gain knob ring from that peak instead of the post-fader meter, so the ring reports the level at the trim, moves when the trim moves, and does not go dark when the fader is down. Device buses keep using their own bus meter.
- No persistence format change; existing projects load unchanged.

Checks: `cargo test -p opentrack-audio` passes 114 tests with 3 ignored, including new cases asserting 0/+6.02/-6.02 dB and inverted polarity reach the rendered samples, that the pre-EQ input level includes the trim while ignoring the output fader, and that the published input peak follows the trim with the fader silent. `cargo test -p opentrack --bin opentrack` passes 188 with the same 14 pre-existing failures in `ruler_interaction_tests` and `sample_editor` pitch-editing tests, reproduced unchanged on `cb5a8c6` without these edits; a new Mixer check asserts the ring lights are dark at 0, rise with the supplied input peak and never read the fader. Live listening, LED observation and desktop review remain open.

