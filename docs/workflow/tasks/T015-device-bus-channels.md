# T015 ? Stereo In / Stereo Out channel strips and compact Device Setup

- Status: review_pending.
- Actual implementer: GPT-6.1 Sol; user review pending.
- Scope: audio-workflow-v4. New requirements: device-bus-strips, device-bus-processing, device-bus-persistence, compact-device-setup. Existing audio-workflow-v3 requirements and weights retained.
- User specification: both existing stereo buses need real Mixer controls and their own VST Audio Channel Settings; Device Setup should be about half its former height.
- Evidence: current user direction plus existing REF-A14-01/02/03 configuration screenshots and accepted Mixer/channel-editor structure. Bus-specific reference behaviors beyond these controls were not newly measured.
- Architecture: two persisted BusChannel values reuse the current channel-strip settings/editor, with reserved identities separate from the arrange Track list. No phantom arrange lanes. Validation prevents media, record arming or self-monitoring on buses. This bounded reuse avoids a broad Track refactor; extracting shared channel fields remains a possible later cleanup.
- Essential checks: actual edit-button routing; editor rendering without selected/any arrange tracks; processing gain/polarity/balance/mute/EQ/inserts; final output includes live MIDI; capture feeds monitoring and recording exactly once; defaults/migration/persistence/undo; compact Device Setup extent. Hardware, desktop, and visual acceptance remain pending.

## Implemented behavior

Both buses use the existing Mixer strip and Channel Settings editor. Their e buttons select the appropriate bus; the editor channel selector can move between tracks and buses; track e buttons restore track editing. The fader, balance, input gain and polarity, mute, EQ, built-in insert selection/parameters and bypass states share the same persisted settings between views. Existing built-in Compressor editor lookup now resolves bus identities too; its design was not changed. Selected/all channel presets include bus settings and distinguish buses from same-named tracks. Legacy track-only mixer presets remain accepted. Display aliases can be renamed in Channel Settings; current routing keys remain the canonical Stereo In/Out strings.

Capture processing is before the monitor bridge and recording queue, at the negotiated capture rate. WASAPI recording configures the bus before enabling capture; the shared ASIO capture processes once and fans out to both destinations. The Stereo In channel meter is now post bus processing, independent of audible track monitoring; Device Setup retains raw device peaks. Output processing runs after the track sum, live MIDI and diagnostic tone, before conversion/clipping, and Stereo Out meters the processed sum. Output mute/fader therefore also affect live MIDI and diagnostic tone. Graphs/coefficients are constructed on the control thread; capture uses a serialized processing gate and deferred graph retirement, without a callback mutex or file operations.

Device Setup defaults to 638?295 logical points (previously 638?550), minimum height 270 and maximum content height 330; its internal page height is bounded to 195..230 with vertical scrolling. Headless tests assert the rendered window stays below 370 logical points across repeated frames and all three pages. Physical display/scaling acceptance remains pending.

## Checks and limitations

97 workspace tests pass with local-asio (50 app, 36 audio, 11 core); default WASAPI workspace cargo check passes. Ten additional regressions cover this slice. No desktop application or hardware stream was launched for verification. OpenTrack was confirmed closed before build. EXAMPLE.opr was not opened or modified.

Sends DSP, automation playback/writing, Listen/control-room processing and independent bus Solo are not implemented and are disabled on bus strips. Bus recording/monitor switches are hidden because capture/monitoring belong to track routes. Physical topology remains the current first stereo device pair; additional buses, physical-port reassignment, Click/control-room buses and third-party VST hosting remain incomplete. The route labels distinguish device Input/Output 1/2 from To/From Tracks and do not offer self-feedback routes. This slice does not claim full reference parity or whole-DAW completion.

## User acceptance

1. Open a disposable project. Check the shorter Device Setup and scroll each page; close it.
2. Open Mixer. Click Stereo In e, then Stereo Out e. Check titles/selected channel, shared EQ/inserts/fader states and the editor's channel selector. Return to a track e; its independent settings must remain intact.
3. Feed input with track monitoring off; Stereo In still meters. Enable track monitoring, adjust input-bus gain/mute/balance/EQ and confirm audio plus meters. Record a disposable take: input-bus processing should be printed exactly once.
4. Play existing audio or live MIDI; change Stereo Out gain/balance/mute/EQ. Confirm final sound and L/R meters respond. Input-bus changes must not process existing recorded playback again.
5. Save/reopen the disposable project and verify both bus states; verify undo/redo. Do not overwrite EXAMPLE for recording tests.
6. User visual/audio/stability review remains the milestone stop. Unsupported controls must stay unavailable.

Release built successfully using `powershell -NoProfile -ExecutionPolicy Bypass -File build.ps1 -Architecture x64 -LocalAsio`. Artifact: `builds/x64/OpenTrack.exe`. HEAD remains `cbd9835` with pre-existing dirty work preserved; this slice is uncommitted.

Identity: x64 local-asio EXE SHA-256 A7685D081B704ACEE8530368849D3CF775D9442C14E1484F97338713150545F3; crates/opentrack-core/src/project.rs: 7369D17B024FAC83FCADABD919C9DC0EEA054229EFEABE35AD29C0315DA4CDC9; crates/opentrack-audio/src/bus.rs: AECC33B01BEDEDE0B0F91753AE0B58C51C384143519FA38EABF2252B50215AFA; crates/opentrack-audio/src/lib.rs: E07B3A26D60030646B3A5509CF7866C1869D6D64B3165467D903BEF584EF8F1B; crates/opentrack-audio/src/recording.rs: 6CA92AEF9EA27B4DCC333CA33125CECB615DD223AA2133E9FD3FD782755DDD83; crates/opentrack-app/src/app.rs: 55C35EEE280EA5EC47C75A53DE43B06B8EE8E26443E014C584868CA5DD45AEFE; crates/opentrack-app/src/mixer.rs: FD5814182C878BA6DA090FBF9702A1A92F6ED5F641B53EC3216A0370A4F2772D; crates/opentrack-app/src/channel_settings.rs: 817BB9C7128183F5F9D1CE48D80DFE0C2B76973757514071F8FA358CE4476388; crates/opentrack-app/src/audio_configuration.rs: 5DBA914AD6396A347B8482ED1B1666D857454362854C6400E6415B0A85510ABD; crates/opentrack-app/src/project_file.rs: 126AFC32E04C71062CDD1F26CFCC83F0ACDC81F17712F945C15F8F7D46DCCE2C
