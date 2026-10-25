# Third-party notices

The optional `local-asio` build uses the vendored maolan-asio 0.0.1 Rust host library under the BSD-2-Clause license. Its license is in `vendor/maolan-asio/LICENSE`; the local build copies it beside OpenTrack.exe as `maolan-asio-LICENSE.txt`. Source provenance and reviewed changes are in `vendor/maolan-asio/OPENTRACK-PATCHES.md`.

This host connects to a driver already installed by the user. The build does not contain a device driver or the Steinberg SDK. The standard build continues to provide WASAPI.
