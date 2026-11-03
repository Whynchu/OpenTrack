# MMC master and Project Synchronization Setup

- Status: accepted for source, Linux checks and a Windows type-check; hardware review pending
- Scope: user-approved MIDI output path (2026-11-03)
- Desired observable outcome: Devices > MMC Master and Transport > Project Synchronization Setup open one dialog that chooses a MIDI output, sets the MMC device id, enables sending MMC play/stop/locate, and states that external clock sync is still unavailable.
- Reference evidence: `MidiOutputDevice`, `MidiOutputStream`, `mmc_message`, `mmc_locate` and their tests in `crates/opentrack-audio/src/midi.rs`; `open_sync_setup`, `send_mmc`, `send_mmc_transport`, `send_mmc_locate`, `show_sync_setup` and tests in `crates/opentrack-app/src/app/sync_setup.rs`.
- Dependencies: none new (WinMM through the existing `windows-sys` dependency).

## MIDI output

- Windows: enumerates `midiOut` ports, opens one with `CALLBACK_NULL`, sends 1–3 byte messages with `midiOutShortMsg` and system-exclusive messages with a prepared header (`midiOutPrepareHeader`/`midiOutLongMsg`, bounded 500 ms wait for `MHDR_DONE`, then unprepare). The handle is reset and closed on drop.
- Other platforms: no ports are listed and opening an output returns "MIDI output is only implemented on Windows in this build", matching the existing MIDI-input situation.

## MMC master

- `mmc_message` builds `F0 7F <device> 06 <command> F7` (stop, play, deferred play, record strobe/exit) with 7-bit masking.
- `mmc_locate` builds `F0 7F <device> 06 44 06 01 hh mn ss ff fr F7` at 25 fps from the transport position; negative, non-finite and past-23-hour positions are refused.
- Transport transitions send MMC play/stop only when the transport state actually changes; a stopped locate sends MMC locate. Both run only when MMC is enabled, and the port opens on first use.
- The dialog persists the output id (`midi-output-id`), the enable flag (`mmc-enabled-v1`) and the device id (`mmc-device-id-v1`, default 127), offers Refresh, and reports when no port is chosen or the selected port cannot open.
- **Use External Sync stays unavailable**: following an external clock needs a clock input and sample-accurate alignment, which this build does not have; the dialog says so.

## Verification

- `cargo test -p opentrack-audio mmc_` -> 2 passed (message bytes, 7-bit masking, locate timecode at 25/30 fps, refusals).
- `cargo test -p opentrack-audio midi_output_is_reported` -> 1 passed (no ports/off-Windows behaviour).
- `cargo test -p opentrack sync_setup` -> 2 passed (disabled/enabled sending, missing port, locate suppressed while playing, dialog controls).
- `cargo check --target x86_64-pc-windows-msvc -p opentrack-audio` -> finished, so the WinMM output code type-checks for the primary platform.
- Inventory regenerated: 277 leaves, **58 available**, 175 state-dependent, **36 unavailable**, 8 informational.
- Suites: app 624 passed / 4 known device-dependent roll failures / 9 ignored (637 tests); core 167; audio 175 / 3 ignored.

## Not claimed

No MIDI hardware or Windows runtime test: MMC was not observed on a real device, and Windows behaviour is verified by compilation only. External sync (following a clock) is not implemented.
