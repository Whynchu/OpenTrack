# T020 - Automation-area isolation, refined channel faders and bus editor layout

- Date: 2026-10-25. Status: review_pending. Implementer: Codex current session; no model switch performed.
- User review: the corrected EXAMPLE open worked; editing/fading worked fairly well. The remaining event issue is automation reveal growing the audio event into the reserved automation area. New requests: less aggressive Mixer faders, -30 dB useful attenuation with bottom mute, positive track gain, and no Sends in Stereo In/Out Channel Settings.
- Scope: event-edit-v2 refines event bounds; audio-workflow-v7 adds the explicit fader/bus-layout requirements. No whole-DAW score.

## Behavior

Arrange layout now separates a track's base media height from the additional 42-point automation area. Audio event painting and hit testing use only the base height; live recording previews and MIDI parts use it too. Revealing automation adds a separate background/divider and controls below the media area. It does not enlarge an event or accept an audio-event drop in that area. Explicit track-height resizing still changes the media height.

Mixer and Channel Settings share a dB fader mapping: approximately +6.02 dB (2.0 linear) at the top through -30 dB, with a small separate bottom mute zone. This replaces the previous -60..0 dB travel that also discarded positive headroom. Relative dragging preserves the grabbed position; Shift reduces drag sensitivity tenfold, and bypasses the narrow coarse unity detent for fine adjustments near 0 dB. Double-click resets unity. The bottom endpoint sets actual zero gain and displays minus infinity. Inspector and secondary channel controls accept the same headroom and mute state. Legacy quieter saved gains remain unchanged until edited; opening the Inspector does not clamp them to the new interaction range.

Core validation and OPR v22 allow channel gain 0..2; event gain remains its separate 0..4 range. All earlier supported versions remain readable through the interval migration. Playback/bus processing already multiplies the true channel gain; the live MIDI path's former unity clamp is removed too, so instrument monitoring responds to positive fader gain. The chosen +6.02 dB ceiling, small mute zone and coarse unity detent are OpenTrack implementation choices; the user specified -30 dB floor, bottom mute and positive headroom, not an exact upper ceiling.

Stereo In/Out editors omit Sends and Send Routing from rendered sections, rack-state controls, Customize View and Setup lists. Normal track editors retain Sends and the shared saved layout preference is preserved. Buses use the available width for the remaining Inserts/EQ/Fader sections. Send DSP and automation processing remain independent unimplemented features; this does not activate them.

## Validation and review

124 release-profile workspace tests passed (70 app, 41 audio, 13 core). The final fine-precision refinement passed both focused fader regressions. New checks cover the full Project grid with automation reveal/drop rejection, dB endpoints and legacy values, actual normal/Shift fader drags including leaving mute, positive gain persistence/validation and live MIDI samples. The existing bus-editor rendering test now asserts Sends is absent on both buses and still present on a normal track.

Release-profile testing was used after the requested cache cleanup so debug and x86 cache trees were not recreated. This is new product work; release dependencies are necessarily rebuilt. No OpenTrack window or hardware stream was launched by the agent. No screenshot/visual or audible acceptance is claimed. EXAMPLE.opr was not opened or modified during this slice.

Review: reveal/hide automation and verify audio events stay in the media portion; test moving events above/inside the reserved area. Move track/bus faders slowly and with Shift, check -30 dB then bottom mute, positive gain and double-click unity. Listen and compare meters, including live instruments; save/reopen a disposable project with positive gain. Open both bus editors: no Sends; open a normal track editor: Sends retained. Stop for user feedback before another milestone.

Release built successfully using build.ps1 -Architecture x64 -LocalAsio. Artifact: builds/x64/OpenTrack.exe.

Identity: x64 local-asio EXE SHA-256 0A33F1CDACFC10BED0067BEBE92B468931AB22F4492A230B8DDF37D157EC1362; crates/opentrack-app/src/app.rs: 159AF276DF6CCCE45A8575FD827169699ABC1955DD0F0394D7F837DCD63899F4; crates/opentrack-app/src/channel_gain.rs: 3BBEFD5D5CAD45013F4CFBED914B2321304E6816AE9D016DEB38542FA058A059; crates/opentrack-app/src/mixer.rs: EC3171148E57EB71BD842C5FB81ABEDAF867561080BCBF42189595D07F56E04A; crates/opentrack-app/src/channel_settings.rs: B1D06240D4071AA2778D9B017585A081774F44395AAA7BCB967B463C2EFCE1CC; crates/opentrack-app/src/project_inspector.rs: 957ACC6F15FDC88F2063DA169BF5EE0CC20D2F95D46006A4F479CA7EBEAED2DE; crates/opentrack-app/src/project_file.rs: 76220FA7A664A556B3776EE12183117BA580B480F6B929AFC1DCB7F4EDE62332; crates/opentrack-core/src/project.rs: 35B98732A5D50B852883C30FDD4171558FC8D6B4893B30A8ECFA17ACB64AEE78; crates/opentrack-audio/src/lib.rs: 55820CF979238640907E0F0CEA695E539C7FFD1ACC32ECC7D2A0A8883E428CC6 Prior published checkpoint: 6b6729f; this slice is uncommitted pending review.
