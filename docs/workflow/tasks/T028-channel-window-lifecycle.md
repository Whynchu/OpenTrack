# T028 — Channel Settings window lifecycle and application pinning

- Status: review_pending; live desktop confirmation pending.
- Implementer: GPT-6.1 Sol; no subagents or independent review.
- User direction: Stereo Out's Channel Settings sometimes fails to appear and later channel editors also fail. User clarified that no window appears. Always on top means above OpenTrack Project/Mixer, while application dialogs and other programs can cover it. Pitch & Time must begin analysis only on explicit activation and retain analyzed/editable data through save/reopen.
- Scope: audio-workflow-v12 adds channel-editor-reopen and application-relative-channel-pin; event-edit-v7 refines the existing unimplemented detected-pitch requirement with explicit activation and persistence. Baselines remain incomplete.
- Code: Channel Settings open commands in app, Mixer and Inspector; native viewport lifecycle; a small Windows owner adapter using existing windows-sys/raw-window-handle dependencies. No DSP, media or project schema changes.
- Architecture: keep one shared channel editor, fresh viewport identity after close, explicit visible/restore/focus on Edit. Native pin uses a normal-level window owned by the main window, not Windows global topmost. Suspend ownership while root dialogs are present; disable editor during confirmations. Restore ownership when dialogs close. Keep the last normal bounds when the window is minimized.
- Acceptance: Open Stereo Out, close and reopen it, then Stereo In and regular tracks via Mixer/Inspector/menu. Repeat while editor is hidden/minimized and after saving/reopening a project. Toggle Always on Top; click Project/Mixer, then open quit/discard, Preferences, Device Setup and native file dialogs, and switch to another application. Confirm editor remains accessible, dialogs are reachable and external apps can cover it. No audio/project edit or dirty state from window changes.
- Regressions: bus vs track targeting and Sends filtering, menu/Inspector/Mixer open paths, spectrum owner/tap switching, monitor/EQ/fader controls, saved workspace bounds and ASIO delivery.
- Review stop: User desktop review; no pitch engine or next substantial milestone in this slice.

## Findings and implementation

The existing bus controls target the correct bus in headless checks. No native reproduction or panic log established the exact source of the reported disappearing window; root cause remains unconfirmed. Two lifecycle weaknesses were corrected: close/reopen reused viewport identity, and open requests consumed focus without explicitly restoring visibility/minimized state. All four entry points now use a shared request method. Explicit reopen ignores a stale close event; a newly closed window receives a new viewport identity on its next open. Builder and open commands restore normal visibility/focus. Minimized native geometry is excluded from saved bounds. Native desktop confirmation remains necessary.

The previous Always on Top path set WindowLevel::AlwaysOnTop globally. It now uses normal window level plus Win32 owner relationships. The owner adapter enumerates only windows on the main window's own UI thread and matches the Channel Settings title. It suspends the owner during root dialogs and prevents editor interaction during confirmations. Other applications remain able to cover the editor. Native OS file-dialog ordering still needs manual confirmation.

Pitch analysis remains absent. The user-specified activation/persistence policy is recorded in AGENTS.md and sample-editor-study.md. No import/play/open operation gained an analysis job and the planned button remains disabled.

## Checks and limits

- ASIO-enabled release app tests: 97 pass, including Stereo Out/Input/track close/reopen with stale close events and real Windows ownership/disabled-state checks using hidden synthetic windows.
- First lifecycle fixture needed window-layout warmup and recursive shape inspection; corrected before final pass. Existing bus/content and editor tests continue to pass.
- Formatting and whitespace checks required; x64 local-ASIO build identity recorded in current.md and build-info.json.
- No OpenTrack/Cubase GUI, audio stream, EXAMPLE file or user project was opened or modified. App remained closed before build. No live desktop acceptance claimed.
- Next: user checks above, then separately scoped DSP-state smoothing/pitch-time processing planning.

Delivered x64 local-ASIO executable SHA-256: `3A682D584BD9FFAC78BFAB885599BC96752CF7B31569CD1219F715C6CC947C5C`. `build-info.json` confirms `local_asio: true`.

Publication: user reported the result looked clean and explicitly requested a Git push. This is positive visual feedback and publication authorization; individual native ordering and audio runtime gates remain as documented.
