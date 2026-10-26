# T035 — Pitch navigation gestures

- Status: review_pending; implementer GPT-6 Luna.
- Scope version: transport-v10-pitch-time-v3; requirement: `pitch-vertical-zoom`.
- User direction: Middle dragging in Sample Editor moves vertically through pitch rows. Shift+wheel also moves the pitch view. Ctrl+Shift+wheel adjusts vertical pitch zoom. Existing horizontal navigation remains.
- Behavior: When Pitch & Time is active, middle-button vertical motion pans pitch while its horizontal motion continues to pan timeline time. Shift+wheel over the editor pans pitch; Ctrl+Shift+wheel adjusts pitch zoom around its current center. Plain wheel retains its horizontal time zoom behavior. View changes do not edit audio or dirty the project.
- Architecture: viewport-only Rust app interaction; bounded to existing Sample Editor and PitchViewport state. Audio analysis, source timing, event edit and playback code are untouched.
- Evidence: screenshot SE-05 and the current T033 screenshot remain visual references; these exact gesture mappings are user specified, not reference-observed.
- Acceptance: middle vertical drag changes visible notes and diagonal drag also pans time; Shift+wheel moves through notes; Ctrl+Shift+wheel changes semitone row height around the same center; plain wheel still zooms time. Gesture results are saved as view state and do not dirty audio/project state.
- Build: x64 local-ASIO build succeeds, OpenTrack closed before replacement. `cargo fmt --all -- --check` and `git diff --check` pass. No tests or live UI interactions were run; full review is waiting on the user.
- Executable SHA-256: `9A82DB868A64D4E153AB99DAD953AE33745F1D53071C74B42D157869509B766B`.

Review follow-up: middle panning is user-confirmed; wheel modifiers failed in the T036 review. T037 corrects their egui event routing and adds actual raw-wheel integration checks. The initial compile-only status did not establish working wheel gestures.
