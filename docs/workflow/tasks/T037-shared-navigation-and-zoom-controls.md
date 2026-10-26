# T037 - Shared wheel navigation and zoom controls

- Status: review_pending. Implementer/reviewer: GPT-6.1 Sol; no delegated or independent review.
- Trigger: user confirms middle dragging works in both views, but reports inconsistent ordinary/Shift wheel and no Ctrl+Shift wheel response. User requests the Sample Editor vertical slider's styling for all three existing zoom controls.
- Diagnosis: egui 0.36 redirects Shift wheel to X and consumes Ctrl wheel as zoom; reading only `smooth_scroll_delta.y` loses both gestures. Project previously also used native wheel scrolling rather than the editor's time zoom.
- Implementation: one original-event handler normalizes Point/Line/Page wheel units before those remappings. Plain wheel zooms time at the pointer; Shift wheel navigates vertically; Ctrl+Shift wheel zooms track height or piano row height. Project native wheel scrolling is disabled so it cannot apply a second action. Middle pan remains the existing interaction.
- All three sliders use the same Rust control, copied from our Sample Editor visual convention: rectangular muted thumb, accent on hover/drag, recessed rail, +/- buttons and double-click reset. Orientation and ranges remain view-specific.
- Evidence: user report tied to T036 build DE937705; T034/T035 current Sample Editor style is the chosen application precedent. Exact modifier mappings are user specified.
- Checks: raw event regression checks exercise egui's actual Shift/Ctrl preprocessing, plus Project and Sample Editor integration frames. They assert the correct dimension changes without project edits. Full application tests pass. Existing arrange tests now account for the 20 px rail reserved in T036; the old hard-coded coordinates crossed different snap boundaries after that layout change.
- Acceptance pending: user checks all three wheel gestures over both views, middle pan, slider hover/drag/buttons/reset, seeking and snap. No desktop app was opened for this slice. User's middle-pan approval is preserved as historical evidence; the rebuilt interactions need review.
- Build identity: recorded in current.md and T038 after the combined x64 local-ASIO build.
