# maolan-asio

Pure-Rust ASIO **host** backend for Windows: enumerate installed ASIO
drivers, open duplex streams, and run the buffer-switch callback loop — with
no Steinberg SDK in the dependency tree and no C/C++ compilation step.

Part of the [Maolan](https://maolan.rs) DAW project; usable standalone.

> Interface definitions were derived from the publicly documented ASIO ABI
> for the purpose of interoperability. This crate contains no Steinberg SDK
> code. ASIO is a trademark of Steinberg Media Technologies GmbH.

## Status

Early development. Implemented so far (see the design plan in the Maolan
repository, `ASIO.md`):

- **M1 (done)**: ASIO ABI types (`abi`), COM apartment and driver
  instantiation (`com`), driver enumeration via the registry with 64/32-bit
  view handling (`registry`), and the `list_drivers` example.
- **M2 (done)**: driver open + initialization, identification, channel
  inventory, sample-rate and buffer-size negotiation (`driver`), the
  `AsioError`/`SampleType`/`BufferSizes`/`ChannelInfo` safe types, an
  in-memory fake driver for hardware-free tests, and the `device_info`
  example. Verified live against ASIO4ALL v2 (XR18: 18-in/18-out,
  Int32 LSB, 44.1 kHz).
- **M3 (done)**: duplex streaming — `createBuffers` with per-channel
  negotiation and preferred-size fallback, all four callback trampolines
  (`bufferSwitch(TimeInfo)`, `sampleRateDidChange`, `asioMessage`),
  empirical `outputReady` probing (JUCE approach), f32 conversion for
  Int16/Int24/Int32/Float32 LSB (`convert`), event bits
  (`Stream::take_events`), and the `duplex_sine` example. Verified live:
  18-in/18-out @ 48 kHz through ASIO4ALL v2 for 5 s.
- **M4 (done)**: protocol-overview crate docs with a compiling quick
  start, root re-exports (`Driver`, `Stream`, `StreamConfig`,
  `CallbackContext`, `AsioError`, `StreamError`, ...), `control_panel()`,
  input validation, and rustdoc verified with `-D warnings`.

- **M6 (done, engine-side)**: Maolan engine adapter (`engine/src/hw/asio.rs`
  with a dedicated COM-owner thread and CoreAudio-style SPSC rings, Windows
  dispatch enum in `engine/src/hw/windows.rs`), selectable with
  `asio:<driver name>` device ids. End-to-end validation with
  `maolan-calibrate` loopback on the XR18 happens on real hardware.

## Testing policy

No test in this repository requires audio hardware or a real ASIO driver —
the full test suite runs on a bare CI runner. The host state machine is
written against a raw driver vtable so that tests can substitute an
in-memory fake driver. Validation against real drivers (e.g. FlexASIO,
vendor ASIO drivers) is a manual activity and is never committed as tests.

## License

BSD-2-Clause, see [LICENSE](LICENSE).
