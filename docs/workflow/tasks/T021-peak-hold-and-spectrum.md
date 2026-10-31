# T021 - Retained peaks and live channel spectrum

Date: 2026-10-25. Status: review_pending. Implementer: Codex current session; no model switch or independent agent review. User direction is the current request for held peak dB, red over-zero meters, live EQ spectrum, pre/post and orange comparison. These are user-specified additions; no new reference observation is claimed.

## Scope and implementation

Mixer and Channel Settings share per-channel session peak holds captured from raw callback peak accumulators, independent of animated meter decay. Track and Stereo In/Out identities remain separate. Holds display dB, turn red strictly above unity, and clear on click; silence does not clear them. No signal displays minus infinity. Project/Mixer/Channel Settings share the stereo meter painter: -60 dBFS through about +6.02 dBFS with a marked unity boundary, dim red headroom and bright red signal above zero. Higher peaks retain their real numeric value even beyond the painted scale. These diagnostics do not change project content or dirty state.

The EQ graph retains editable response nodes/curve and overlays actual audio spectrum, not a waveform simulation. Pre Inserts/EQ and Post Inserts/EQ select taps after input pre-gain/polarity, before and after channel EQ/inserts, before channel fader/pan. Selected channel uses the accent color; comparison uses orange for both live spectrum and the reference EQ response. The spectrum has a separate right-hand dBFS scale; the existing response axis remains +/-24 dB. Compare lists other tracks and device buses. Close or owner-channel change clears comparison, disables old captures and resets Post. Comparison/stage are transient, not serialized.

Spectrum capture is bounded: lazy control-thread allocation, 4096-frame atomic ring, pre/post stereo samples, no FFT, locks, file I/O or allocation in capture. Reader copies 2048 samples and performs an original radix-2 Hann-window FFT on the UI thread at most every 30ms. Stereo power averaging preserves anti-phase signals. Capture/sample-rate generations and slot sequences reject overwritten/inconsistent frames. Stopped/stale streams clear traces after 250ms. Analyzing muted tracks/buses does not unmute output. No third-party dependencies or schema changes.

Known gap: direct live MIDI already bypasses individual instrument track processing; its sound appears at the final Stereo Out analyzer, but not its individual track analyzer. Scheduled instrument audio and audio tracks use the track taps. Third-party VST hosting remains separate and is not implied by the existing editor title. No measured hardware analyzer latency, screenshot parity or live UI acceptance yet.

## Validation and handoff

129 release workspace tests pass (73 app, 43 audio, 13 core), with no compiler warnings. New tests verify peak retention/isolation/reset, stereo over-zero red geometry, actual analyzer control rendering on bus editors, close/channel reset/tap release, calibrated anti-phase FFT/pre-post gain/rate activation, and real +6 dB bus EQ spectral difference while muted output stays silent. Existing recording/event/ruler/fader/bus/persistence regressions pass. Build and source identity appended below after release build.

No app/hardware stream was launched, and EXAMPLE.opr was not opened or modified. OpenTrack was closed before building and will be left closed. Existing T020 source is preserved; this packet remains uncommitted pending review. Published checkpoint remains 6b6729f.

Review on the new executable: play/monitor an audio track, verify peak remains after silence and click clears it; increase gain to overload and check red number/red above-zero meter section on track and output. Open Channel Settings, enable/alter EQ and switch Pre/Post; spectrum must follow audio while response nodes stay editable. Compare another playing channel (orange), switch owner then close/reopen: both must restore Post/None. Check input/output buses, mono/stereo, stopped streams and unchanged project dirty state. Live timing, hardware and appearance remain pending. Stop for user feedback.

Release build succeeded: build.ps1 -Architecture x64 -LocalAsio. Identity: builds/x64/OpenTrack.exe: CC913709320C3677EDA3EA5F0BC1110BCF658B6094B73B6B439ECBA49C399733; crates/opentrack-app/src/app.rs: DC553EE40501FFC6D17FA83409290882E70970053456872D162EF9B038CC5F41; crates/opentrack-app/src/analyzer_view.rs: F998FC0FC7655C4D1E52D007D3591B022577096BA78B8CC57E3DD0BDDBAF1130; crates/opentrack-app/src/channel_settings.rs: E459A13954A296E1064686DA38A73825340565F91BF9982B6C6938ECE4B01D36; crates/opentrack-app/src/mixer.rs: 51877EA4BCBF7C2C1018418900FBEA3171CB8251019CB3327047486E8A3B4456; crates/opentrack-audio/src/lib.rs: 800ADA46976356E9907F212F2B98E890DFCBAA8349851E2EA631652BC3672A49; crates/opentrack-audio/src/spectrum.rs: DA401F8F10C20446B6EBD23819BEF37F7CBD36FC4F04708B7E65207ACEE31309; crates/opentrack-audio/src/bus.rs: 8B151AF5D2674A3C4225A987C55D83E76305CC3C1D7D433667BF447144469E1C; crates/opentrack-audio/src/recording.rs: 031566532F6AC64C3235820F698E320A56DA8C5E2BF96880064DCB9B97851E64

## Low-frequency display resolution follow-up (2026-10-30)

The Channel Settings live spectrum uses a 4096-sample FFT and 8192-sample
capture ring, doubling frequency-bin density. At 44.1 kHz this gives about
10.8 Hz per bin; at higher sample rates the absolute spacing scales with the
rate. The EQ response curve itself was already drawn using 257 logarithmic
frequency samples, so this specifically improves live spectrum detail. The
existing temporal release smoothing remains unchanged. This increases the
analysis window from about 46 ms to 93 ms at 44.1 kHz. Playback/UI timing and
visual review remain pending.

Linux x64 release with local ASIO was rebuilt after this change and installed
at `builds/x64/OpenTrack`, SHA-256
`a1e0e3f9bcec3ab6d382580ed788b0285583f60a3fe839bb83ee1af1b93b6aeb`.
`cargo fmt --all -- --check`, `git diff --check`,
`cargo check --offline -p opentrack --features local-asio`, and
`cargo build --offline --release -p opentrack --features local-asio` pass. No
tests or live audio/visual checks were run for this follow-up.
