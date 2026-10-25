# Current assignment

Updated: 2026-10-25. Implementer: GPT-6.1 Sol; no subagents or independent review.

## Current review - T028

[T028](tasks/T028-channel-window-lifecycle.md) hardens shared Channel Settings close/reopen and restores hidden/minimized editors on Edit. All Mixer/Inspector/Project/menu entry points use the same request. Closed editors get a fresh viewport identity; stale close events cannot cancel an explicit reopen. The exact cause of the user-reported Stereo Out disappearance remains unconfirmed without live reproduction.

Always on Top now means a normal Windows window owned by the main OpenTrack window. It can be covered by external programs. Root dialogs suspend ownership; confirmations disable editor interaction. Live dialog/focus/external-app review is pending.

Pitch & Time activation and persistence are now explicit in AGENTS.md and sample-editor-study.md: analyze only on the user's Pitch & Time click for the selected audio, retain valid results and editable intent across save/reopen, and explicitly re-analyze invalidated sources. Pitch detection and audible correction remain absent; no analysis engine or enabled placeholder was added.

## Verification and build

97 ASIO-enabled release app tests pass, including repeated Output/Input/track close/reopen with stale close input, and Windows owner/global-topmost/confirmation checks on hidden synthetic windows. x64 local-ASIO release succeeds; build-info.json records local_asio=true. Prior audio tests (47) and core checks remain recorded in T027/T026. No OpenTrack GUI, Cubase, hardware stream or EXAMPLE project was opened. OpenTrack was absent before build and remains closed. No live visual or hardware acceptance claimed.

Current builds/x64/OpenTrack.exe SHA-256: 3A682D584BD9FFAC78BFAB885599BC96752CF7B31569CD1219F715C6CC947C5C.

## Prior work and next review

T020-T028 are included in the user-authorized publication checkpoint. The user reported the latest changes looked clean; detailed runtime gates below remain pending. T027 monitoring-exclusive playback, parameter crossfade and topmost take changes still need audible review; the accidental WASAPI-only T027 build was corrected and ASIO is retained by the build script. T026 group editing/shared editor loop and T025 media lifecycle/rate conformity remain available for review. Source preservation, reversible deletion, Rust-first architecture and no AudioWarp remain standing direction.

Check T028: Stereo Out then Stereo In/track editors, repeated close/reopen, minimized/hidden editor, Always on Top, quit/save/discard, Preferences/Device Setup/native file dialogs, and another application. Stop for feedback before a new substantial milestone. Exact native disappearance cause, live ordering and focus remain pending.

Metrics: audio-workflow-v12 adds the two window requirements; event-edit-v7 refines the existing absent pitch-analysis contract. Baselines remain incomplete; no whole-DAW percentage or parity acceptance inferred from tests/builds. User authorized committing and pushing this checkpoint; publication does not resolve the remaining runtime gates.
