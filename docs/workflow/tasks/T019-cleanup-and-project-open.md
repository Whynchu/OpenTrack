# T019 - Disposable cache cleanup and native-project open correction

- Date: 2026-10-25. Status: built, publication authorized; live open/event review pending.
- Implementer: Codex current session; no model switch performed.
- User direction: clean accumulated garbage/build caches and push while the user reviews. During cleanup, user reported EXAMPLE.opr returned to the empty startup shell without an error. Actual source path supplied: C:/Users/Samue/Documents/EXAMPLE/EXAMPLE.opr.

## Open failure and correction

The actual EXAMPLE document is native version 20. The current v21 reader handled explicit versions 1..18 and the current version, accidentally omitting 19 and 20. Open failure only updated a status message, which was invisible from the empty startup shell. A regression reproduced the rejection at version 19 before the fix.

The reader now accepts the complete default-compatible interval 14 through current-1, preserving the earlier special migrations and rejecting unsupported future versions. Missing bus/event fields receive neutral serde defaults. Failed opens also show a dismissible error window, including from the empty shell, and preserve the existing project state.

119 workspace tests pass (67 app, 40 audio, 12 core), standard WASAPI check passes, and the x64 local-asio release builds. The compatibility regression exercises versions 14..21 and checks files remain unchanged; an optional OPENTRACK_READ_ONLY_PROJECT probe additionally loaded the user's exact version-20 EXAMPLE with all three tracks and asserted its bytes were unchanged. The future-version error-window test renders headlessly without opening audio. This validates parsing/migration and error presentation, not a live desktop open or hardware result.

EXAMPLE original SHA-256 after the read-only probe: 54E4BDCF45BBA5D0D21DE5EB6008619AC9FB8673303FE89F0B5F8DD4C03CEDF4. The file and recordings were never saved or edited by the agent.

## Cleanup completed

Removed 8,757,268,963 bytes (8.156 GiB) of disposable build/test artifacts: .build-cache/debug, release, i686-pc-windows-msvc, x86_64-pc-windows-msvc, toolchain-recovery and rustc metadata. Verified every recursive target was a direct child of the resolved repository cache, within the intended workspace; checked no active Cargo/rustc process and no reparse points. Used native PowerShell file operations only.

Preserved 1,120,148 bytes of review/export/audit evidence by moving them outside Git to G:/dev/Opentrack/_analysis-tools/reports/opentrack-cache-archive/2026-10-25-001410. The archive includes cleanup-manifest.json with original paths, actions and byte counts. The cache directory is now absent and will be recreated by the next build. Durable review evidence must not live only inside this disposable cache.

Preserved builds/x64/OpenTrack.exe and verified its hash before/after deletion; also retained x86 deliverables, source/vendor dependencies, all saved projects/audio and existing reference/comparison folders. No global Cargo/toolchain cache or unrelated workspace tooling was deleted. Tests/build completed before cleanup, so verification did not recreate the cache afterward.

## Handoff

The current publication includes T018 audio event editing/style/startup changes plus this compatibility/error-display fix and cleanup record. User explicitly authorized pushing the current checkpoint; publication is not M2 or runtime acceptance. OpenTrack was not launched by the agent. Review by opening the exact EXAMPLE file in the corrected binary; check existing tracks/media and new event controls. Do not overwrite EXAMPLE during destructive tests. Stop for feedback.

Artifact identity: x64 local-asio EXE SHA-256 5AFF02C355A5260D2B575D54E961AB9E02BD269A49D6C9A212F645FCD4021A15; crates/opentrack-app/src/app.rs: 4F37B343A6C52F810227CB63CCD2CE058FEAEDE2C6DA9B862EBC3D0F94544BCA; crates/opentrack-app/src/project_file.rs: 5C0A7312FE8C8B23757503B09F8C8CBB6A9E68979EC804796BADB27A8C9B42A1; crates/opentrack-app/src/audio_events.rs: 48B81735834FC7B6B6DEAF5EEC3F48E804B49A45BD57FE82F386911E33499C48
