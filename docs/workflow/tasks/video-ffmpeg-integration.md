# Video: FFmpeg backend and Video Window

- Status: accepted for source, a feature build and headless decode; desktop, packaging and Windows runtime review pending
- Scope: user-approved video decoder work (2026-11-03)
- Desired observable outcome: with the `video-ffmpeg` feature and FFmpeg libraries present, Devices > Video Window (F8) shows the decoded frame nearest the playhead and refreshes as the transport moves; without the feature the row stays disabled and the window explains the build lacks a decoder.
- Reference evidence: `crates/opentrack-video` (`VideoDecoder`, `RgbaFrame`, `backend_available`, decode test), `crates/opentrack-app/src/app/video_window.rs` (window, texture update, tests), `devices.video` dispatch, `video-ffmpeg` feature.

## Integration

- **Isolated crate** `opentrack-video` with an optional `ffmpeg` feature; the app enables it through `video-ffmpeg = ["opentrack-video/ffmpeg"]`. Default builds do not require FFmpeg and the menu row stays disabled.
- **Decode path:** `ffmpeg-next` 9 (`ffmpeg-sys-next`, FFmpeg 8): open input, best video stream, decoder from stream parameters, `swscale` to RGBA, tighten stride rows, return `RgbaFrame { width, height, rgba, timestamp_seconds }`.
- **Lookup:** seek to one second before the target (a failed seek falls back to decoding from the start), decode until the timestamp passes the target, bounded to 4,000 frames; the nearest frame wins.
- **Video Window:** opens a file (mp4/m4v/mov/mkv/avi/webm/y4m), decodes the frame at `position_beats * 60 / tempo`, uploads an egui texture, and refreshes when the position moves by more than 1/30 s. Decode errors are shown instead of a stale frame.
- **Menu/keymap:** `Devices > Video Window` availability is `opentrack_video::backend_available() && project_open`; F8 (`devices.video`) dispatches. Keyboard coverage is now 151/152 (Use External Sync remains).

## Licence and packaging

- `ffmpeg-next` is WTFPL; FFmpeg itself is LGPL-2.1-or-later when built without GPL-only parts, linked dynamically. The binary requires the matching FFmpeg 8 runtime libraries on the target machine.
- A distribution decision is still needed: bundle the required shared libraries beside OpenTrack (with notices) or document a system FFmpeg requirement. Until then the feature is a developer/opt-in build, not part of the default release binary.

## Building with the decoder on this machine (no sudo)

The dev packages were extracted to a local prefix; they are not committed:
```
sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libswresample-dev   # normal path
# or, without sudo:
apt-get download libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libswresample-dev \
                 libavcodec62 libavformat62 libavutil60 libswscale9 libswresample6
for f in *.deb; do dpkg-deb -x "$f" /tmp/opencode/ffmpeg-dev; done
export PKG_CONFIG_SYSROOT_DIR=/tmp/opencode/ffmpeg-dev
export PKG_CONFIG_PATH=$PKG_CONFIG_SYSROOT_DIR/usr/lib/x86_64-linux-gnu/pkgconfig
export BINDGEN_EXTRA_CLANG_ARGS="-I$(gcc -print-file-name=include)"
export LD_LIBRARY_PATH=$PKG_CONFIG_SYSROOT_DIR/usr/lib/x86_64-linux-gnu
cargo test -p opentrack-video --features ffmpeg
cargo test -p opentrack --features video-ffmpeg video_window
```

## Verification

- `cargo test -p opentrack-video --features ffmpeg` -> decode test passed: a generated Y4M clip decodes a 2x2 RGBA frame, opaque alpha, BT.601 red (`r>180, g<60, b<60`), timestamps 0.0 and 0.04, and missing files/negative positions are refused.
- `cargo test -p opentrack-video` (default) -> the "not built" test passes.
- `cargo test -p opentrack --features video-ffmpeg video_window` -> 2 passed: the window explains a build without the decoder, and a real Y4M opens, decodes and uploads a texture.
- Inventory: 277 leaves, 63 available, **177 state-dependent, 29 unavailable**; Video Window is state-dependent on the backend.
- Default suites: app 633 passed / 4 known device-dependent roll failures / 9 ignored (646 tests); core 167; audio 175 / 3 ignored.

## Not claimed

No desktop review, no Windows FFmpeg build/runtime, no transport-synced playback of long videos or audio extraction yet; the other video rows (import, replace audio, thumbnails, extract audio) remain unavailable and need the packaging decision before they can be honest.
