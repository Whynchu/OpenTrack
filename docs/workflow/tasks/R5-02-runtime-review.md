# R5-02 — Runtime review of the manual-review fixes (user)

- Status: **open — user runtime review, not performed**
- Scope version and requirement IDs: revision 5, step 2
- Desired observable outcome: the four post-commit manual-review findings from revision 4 behave correctly in the running application, confirmed by the user on the desktop.
- Reference evidence (IDs/paths): the review fixes in `9a5e7dd`; `docs/workflow/current.md` "QA fixes - 2026-11-02"; `crates/opentrack-app/src/app.rs` (extra mixers as native viewports, relative widget ids, `toggle_inspector_panel`), `crates/opentrack-app/src/analyzer_view.rs` (grid and status line), `crates/opentrack-app/src/audio_export.rs` (`locator_range`).
- Dependencies and decisions needed: the user's desktop, an audio output device and (for the spectrum item) something playing on the master bus.
- Code scope and architecture boundaries: no code change expected during the review; each finding either passes or is reworked as a new fix.
- Acceptance checks (action, expected result, fixtures, evidence to retain):
  1. **Mixer click-through:** open Window > Mixer 2 and Mixer 3, drag Mixer 2 over the arrange and click inside it. Expected: clicks act on the mixer and never select or move events underneath.
  2. **Duplicate widget id:** open the docked Mixer and both extra mixers in one session and interact with the colour controls, type icons and automation parameter on several tracks. Expected: no "Second use of a widget" assertion, no flicker or wrong-colour swatches.
  3. **Spectrum status line:** open Media > Advanced > Spectrum Analyzer with no device (expect "no audio output device"), then with playback running (expect "live spectrum of the master output") and while stopped but device open (expect "waiting for audio").
  4. **Mixdown range:** open File > Export > Audio Mixdown with the locators set to a sub-range. Expected: the dialog shows that range read-only, there are no editable start/end fields and no "Use locators" button, and the rendered file is exactly the locator range. Equal or inverted locators must refuse with "Set the left and right locators before exporting".
- Affected regressions: none until the user reports a failure.
- Essential gates: audible/visible result; the mixdown range is checked against the rendered file, not only the dialog.
- Review owner and milestone stop: **the user.** DeepSeek V4.1 Flash cannot perform this pass on this host (no desktop session or audio device in the harness) and does not claim it.

## Handoff

- Actual implementer/reviewer model: fixes by DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); runtime review by the user.
- Source identity: fixes in `9a5e7dd`; the revision-5 build is `afcfa65` plus this documentation commit.
- Changes and reasons: none in this packet; it records the open review.
- Commands/checks and actual results: automated coverage only — `cargo test -p opentrack` exercises the locator range and its refusals, and one test draws both extra mixers in a single frame, which reproduces the duplicate-id assertion if it returns. Neither proves the desktop behaviour.
- Runtime environment and evidence paths: Kubuntu Linux harness without a display or audio device; the user's desktop run is the evidence this step needs.
- Ledger IDs updated: none.
- Review findings, rework, elapsed effort: not started; no findings yet.
- Failures/unknowns/pending acceptance: all four items above are pending.
- Next ready task: none on this host — R5-02 closes when the user reports the four results.
