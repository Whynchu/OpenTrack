# Video: FFmpeg integration plan and environment blocker

- Status: **blocked on the build environment and a packaging decision**; no dependency added yet
- Scope: user-approved video decoder work (2026-11-03), recorded before any code so the integration is not invented ad hoc
- Reference: `R5-28-video-decision.md` (FFmpeg FFI recommended, LGPL build), the six unavailable video rows.

## Planned shape

1. **Separate crate** `opentrack-video` behind an optional feature, so the rest of the app builds without FFmpeg and the ABI boundary stays isolated (the project rule for external ABIs).
2. **LGPL FFmpeg build** without GPL-only components, dynamically linked, pinned version; ship the required shared libraries beside the Windows binary and record their licences/notices in `docs/third-party-notices.md` before distribution.
3. **Decode path:** demux → decode → convert to RGBA (`swscale`) → bounded frame cache and timestamp lookup; no frame is copied into the project.
4. **UI:** `Devices > Video Window` shows the frame at the transport position (seek and cycle handled), `File > Import > Video File` stores the reference and extracts audio through the existing WAV import, `Media > Generate Thumbnail Cache` fills a bounded cache, and `Replace Audio in Video File` is only attempted after decode/encode round trips are verified.
5. **Verification:** decode named sample files (H.264 in MP4, at least), frame timestamps against the transport, seek/loop accuracy, missing or unsupported files refused with a reason, and a licence/packaging review.

## Blocker

- This machine has **no FFmpeg development files** (`pkg-config --exists libavcodec` fails) and `sudo` is not available to the agent, so `ffmpeg-sys`/`ffmpeg-next` cannot build here even though the network is reachable.
- Required on this host (or a Windows build machine):
  `sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libswresample-dev`
  or the Windows FFmpeg development package matching the chosen version.
- A product decision is also needed on **how FFmpeg binaries are shipped** (bundled DLLs versus requiring a system install) and which pinned version to support.

## Current behaviour

All six video rows remain unavailable with this recorded reason; nothing pretends to decode. The MMC, score, and everything else in the app build without FFmpeg.

## Next step once unblocked

Add `opentrack-video` with the optional feature, implement decode-to-frame plus a test that reads a known file's first frame and timestamp, then wire the Video Window and import rows.
