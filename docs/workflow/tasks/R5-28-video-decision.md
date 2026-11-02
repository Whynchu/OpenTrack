# R5-28 — Video decoder evaluation

- Status: accepted (decision record; no dependency added)
- Scope version and requirement IDs: revision 5, step 28
- Desired observable outcome: a written comparison and recommendation for video, so the user can approve or reject a dependency knowingly.
- Reference evidence (IDs/paths): repository dependency rule (AGENTS.md), `File > Import > Video File`, `Devices > Video Window`, `Media > Extract Audio from Video File`.

## Comparison

| Option | Decode | Licence/complexity | Windows x64 | Verdict |
| --- | --- | --- | --- | --- |
| FFmpeg via `ffmpeg-next`/FFI | Full | LGPL-2.1+ when built without GPL parts; requires shipping/linking FFmpeg binaries and matching build flags; large surface | Works, needs DLL packaging | Feasible only with an explicit licensing and packaging plan |
| Pure-Rust demuxers (`mp4parse`) | Container only | Small, permissive | Fine | Not enough: no decoder, so no picture |
| Platform APIs (Media Foundation) | Full | No new dependency, but a large unsafe COM surface and per-OS code | Windows-only, real work | Possible long term; contradicts the portable build |

Frame-accurate sync (video follows the transport, including seek and cycle) is a separate engine concern from decoding, and both are needed before a `Video Window` could honestly exist.

## Decision

Do not add a decoder in revision 5. Recommendation if video becomes a product requirement: FFmpeg FFI with an LGPL build, pinned version, documented shipped binaries, and a licence review recorded before the first line of integration code; Media Foundation only if the product drops non-Windows builds. `Video`, `Video Window`, `Video File...` and `Extract Audio from Video File` stay unavailable with this record attached.
