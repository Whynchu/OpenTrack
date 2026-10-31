# T055 — Splash isolation, VST editor animation, and loop pencil cursor

- Status: revised_build_user_review_pending
- Scope version and requirement IDs: v1; user follow-up (2026-10-27).
- Desired observable outcome: startup shows only a centered, borderless splash for at least two seconds, then presents the normal OpenTrack window; Ozone's native editor continues repainting without pointer movement; the Project loop-draw pencil points toward the upper-left from the cursor hotspot.
- User feedback: continuous fader/EQ response is substantially improved. Ozone loaded on Stereo Out, but its display appeared static until pointer movement. The old splash overlaid the main app window, and the loop pencil cursor pointed down-left.
- Code scope and architecture boundaries: initial root viewport and startup UI; native VST3 editor paint dispatch; cursor-only SVG and cursor hotspot. Keep the main application UI out of the splash interval and retain the normal toolbar pencil icon.
- Previous implementation was rejected by the user: it presented the main app as a small splash-sized window and did not resolve Ozone animation. It must be discarded.
- Revised implementation: show a standalone 600×327 native splash using the README SVG unchanged, over a uniform dark grey background (#181818). Since the bundled resvg build omits SVG text, draw the wordmark in GDI using the SVG's Inter-first font family. Keep the main window uncreated until the splash closes after its two-second minimum; then create the normal maximized eframe window. The dedicated VST editor thread drains its full message queue, including thread-level WM_TIMER messages; periodic forced invalidation was removed because it did not resolve Ozone.
- Cursor implementation: a cursor-specific pencil SVG points upper-left; its point aligns with the OS pointer hotspot. Keep the ordinary toolbar Draw icon unchanged.
- User clarification: Ozone's EQ display and level meters must update during audio processing without needing pointer movement.
- Acceptance checks: (1) cold launch shows only the splash with no application background and keeps it visible for at least two seconds; (2) main window appears afterward maximized and remains usable; (3) play audio through Ozone on Stereo Out and verify its EQ display and level meters continue updating while the pointer is still; (4) activate Loop Draw and confirm pencil tip is at upper-left/hotspot; (5) verify ordinary toolbar Draw icon remains unchanged.
- Affected regressions: startup sizing/position and main window controls; Ozone/VST3 repaint CPU use and editor input; Loop Draw cursor accuracy and toolbar artwork.
- Essential gates: live cold-start visual check and Ozone animation while playback is running. Source/build checks do not establish these runtime outcomes.
- Review owner and milestone stop: user review after release build; do not mark Ozone animation accepted without live playback verification.

## Follow-up implementation (2026-10-27)

- Gateway host output parameters are now accepted into bounded per-block VST3 queues and published to the controller from the UI/control thread. The editor-open path synchronizes them and schedules 16 ms repaints. An installed Gateway smoke test loaded the local plug-in and processed an audio block successfully; visible Gateway I/O meter animation during playback still requires a GUI check.
- The x64 release embeds an `.ico` resource with multiple native sizes and uses matching small/large Win32 window icons. The release package script stages the executable, cleans only the verified `builds/<arch>` directory, and stores build metadata in hidden `.build-cache/metadata`.
- Startup's root window starts hidden and is shown maximized after its first UI frame. Transport, Channel Settings, Media Pool, and MIDI Editor viewports likewise wait for their first frame before becoming visible to reduce the opening flash.
- Verification: `cargo fmt --all -- --check`, `cargo check --offline -p opentrack`, x64 `build.ps1` release build, and the installed Gateway smoke test passed. `builds/x64` contains `OpenTrack.exe` and the two required ASIO/legal notices. EXE SHA-256: `4A559F7DB12BEC1E84E6464DAE948F314FEF6575CB3552F571BC1900ADEADBF2`.
- Runtime cold-start and Gateway meter appearance remain unverified because the Sky computer-use service returned `Trusted RPC service is not configured: sky`. Do not mark visual acceptance complete based on source/build checks alone.

## Handoff

- Actual implementer/reviewer model: GPT-6 Luna; independent review pending.
- Source identity and build hash: revised `builds/x64/OpenTrack.exe`, SHA-256 `2E9E5A634FF2E70DB8E8AF66E4E94ADA8A82B841FA462F53CEA359AA57BF7FB2`; build metadata confirms `local_asio: true`.
- Commands/checks and actual results: `cargo fmt --all -- --check`, `cargo check -p opentrack`, and `build.ps1 -Architecture x64 -LocalAsio` pass.
- Runtime environment and evidence paths: Windows x64; user will inspect the build.
- Ledger IDs updated; report command/result: T055 status in `docs/workflow/current.md` and this task packet.
- Failures/unknowns/pending acceptance: user reports the first revised splash lacked the expected OpenTrack branding, had black bars, and left the app minimized. The current revision uses the exact supplied artwork ratio on a single grey canvas and delays creation of the main window until splash completion. It has not been visually run because the Sky computer-use service was unavailable. Verify cold-start splash and maximized main window; verify Ozone analyzer and level meters during playback with pointer still. If Ozone remains static, inspect VST3 host run-loop/timer callbacks and audio context notifications.
- Next ready task: user review.

## Linux splash follow-up (2026-10-30)

- Cause: `startup_splash::show()` was implemented only for Windows; the
  non-Windows implementation returned `false`, so Linux launched the main app
  directly without showing the splash.
- Change: Linux now creates a separate borderless 600×327 eframe viewport with
  the same bundled artwork and dark grey background. The main Project viewport
  remains hidden until the splash viewport's first frame has been visible for
  at least two seconds, then it
  is closed and the normal maximized app is shown. This uses the existing
  eframe event loop, avoiding a second Linux windowing event loop.
- Verification: Linux x64 local-ASIO `cargo check` passes. A focused test
  verifies the two-second lower bound; runtime display check is pending.
- Windows startup code is unchanged. Windows release build and visual review
  remain separate checks.
- Additional checks: Linux release build with local-ASIO enabled succeeded;
  `.build-cache/release/opentrack` SHA-256
  `ce2a41eb838aa44f1f5e346bf44c43fd861ac52d5966c4008d3703b4af93faf0`.
  A seven-second debug launch reached the timeout without a startup panic after
  registering the splash wordmark font. The desktop automation inventory had no
  app surface, so the splash could not be visually confirmed; ALSA emitted the
  environment's duplex-device warnings during launch.
- 2026-10-30 follow-up: the initial Linux version showed the splash in a
  secondary viewport while eframe had already created the hidden main
  viewport. The user reported that this still opened an empty main window.
  Linux now launches the same executable in splash-helper mode and waits for
  that standalone eframe process to exit after the first-frame-based two-second
  minimum. Only then does the parent start eframe and construct OpenTrackApp.
  If the helper cannot start, startup falls through to the normal app.
- Verification: Linux local-ASIO check and the two-second timing test pass.
- Verification: Linux local-ASIO check and the two-second timing test pass.
  Running the packaged splash helper alone returned success after 2.105
  seconds, and because the parent is blocked on that process, main eframe
  startup occurs afterward. Visual appearance remains pending because desktop
  automation exposes no app surface.
- Packaged Linux x64 release: `builds/x64/OpenTrack`, SHA-256
  `1cfb63ec89994dfced7838bf3e2de13cbbdf120b04d5f0f25793f34eef2557d1`;
  `builds/x64/build-info.json` records `platform: linux-x86_64` and
  `profile: release`.
- Splash sizing follow-up: assign a splash-specific Linux app ID and explicitly
  request the fixed 600×327 point size with maximized/fullscreen disabled. This
  avoids inheriting the maximized state of the main app/window-manager class.
  A focused builder test checks those settings; visual size confirmation is
  pending.
- Bounded launch of the updated package's splash helper exited successfully
  after 2.206 seconds; this confirms startup behavior but not its on-screen
  dimensions.
