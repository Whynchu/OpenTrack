# T036 - Project track navigation

- Status: review_pending; implementer GPT-6 Luna.
- Scope version: transport-v11-project-navigation-v1; requirement: `project-track-vertical-zoom`.
- User direction: add a vertical control to zoom Project track lanes, matching Sample Editor pitch navigation gestures and key combinations.
- Behavior: lower-right vertical slider scales all lane heights while retaining per-track height proportions; existing keyboard track zoom commands change the same shared zoom value. Shift+wheel scrolls vertically through tracks; Ctrl+Shift+wheel zooms lane heights; middle drag continues to pan both visible timeline dimensions. The timeline zoom control and plain wheel behavior remain separate.
- Architecture: Rust app view-state scale is serde-persisted with `ArrangeViewState`. Per-track custom sizes remain preserved and are multiplied by the view scale. Track zoom does not edit audio/project content.
- Evidence: user specification only; the Sample Editor mapping in T035 supplies the gesture precedent. No new reference capture was requested.
- Acceptance: user checks slider drag, keyboard zoom synchronization, Shift+wheel vertical track scroll, Ctrl+Shift+wheel zoom, middle-drag vertical/horizontal pan, custom track proportions, and ordinary Project seeking/zoom behavior.
- Build: x64 local-ASIO build succeeds with WASAPI retained. OpenTrack was closed before the build. `cargo fmt --all -- --check` and `git diff --check` pass. No tests or live UI interactions were run; runtime acceptance awaits the user.
- Executable SHA-256: `DE937705B2042F353FC55C8940E7B917E40DB5C1C86D93D9B45AC214EBD0F4B6`.

Review follow-up: user confirms middle drag works in Project and Sample Editor, but reports inconsistent wheel behavior and no Ctrl+Shift response. T037 supersedes the wheel implementation and unifies all three slider styles; the original build/format result above was not runtime acceptance.
