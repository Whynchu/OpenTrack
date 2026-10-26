# Third-party notices

The optional `local-asio` build uses the vendored maolan-asio 0.0.1 Rust host library under the BSD-2-Clause license. Its license is in `vendor/maolan-asio/LICENSE`; the local build copies it beside OpenTrack.exe as `maolan-asio-LICENSE.txt`. Source provenance and reviewed changes are in `vendor/maolan-asio/OPENTRACK-PATCHES.md`.

This host connects to a driver already installed by the user. The build does not contain a device driver or the Steinberg SDK. The standard build continues to provide WASAPI.

The VST3 foundation uses the `vst3` 0.3.0 Rust bindings (MIT OR Apache-2.0), `libloading` 0.8.9 (ISC), and the transitive `com-scrape-types` 0.1.1 crate (MIT OR Apache-2.0). These are ABI bindings and loading helpers, not Steinberg SDK source or sample code. No VST3 SDK headers, plug-in binaries, or Steinberg-authored files are vendored in this repository.
