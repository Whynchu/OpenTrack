# Help > Registration and Media > Reconstruct

- Status: accepted for source and automated checks; desktop review pending
- Scope: post-revision-5 feature work
- Desired observable outcome: Help > Registration explains that no registration exists, with no fake form or link; Media > Reconstruct re-reads the project's audio sources from disk and rebuilds the playback plan.
- Reference evidence: `show_registration` with its render test in `crates/opentrack-app/src/app/registration.rs`; `reconstruct_audio_sources` with its test in `crates/opentrack-app/src/app/reconstruct.rs`.

## Help > Registration

- A small informational dialog: "OpenTrack needs no registration." — no account, licence key, activation server or sign-in; every feature is available without one and the application does not contact a server for registration.
- Mentions the Apache-2.0 source and points to Credits and Copyrights for third-party licences.
- **No link**: the render test asserts the text contains no `http`.
- Close button and outside-click dismissal; the dialog joins the modal keyboard guard.

## Media > Reconstruct

- Collects every source the project references (media pool plus clip paths) and re-opens each with the ordinary audio source loader, replacing the in-memory decoded/streamed sources and rebuilding the playback plan.
- This is the honest meaning of "reconstruct" here: waveforms and playback read those sources, and nothing is cached separately. It is the recovery path after a file was replaced or repaired outside OpenTrack.
- Reports "Reconstructed N audio source(s) …" and lists up to three unavailable files with a count when a file is gone; an empty project reports instead of pretending.
- Missing sources are left out of the rebuilt map, so playback does not use a stale decode.

## Verification

- `cargo test -p opentrack registration` -> 1 passed: the dialog states no registration/account and contains no http link.
- `cargo test -p opentrack reconstruct` -> 1 passed: an empty source map is rebuilt from a real WAV, a deleted file is reported as unavailable with its name, and the no-project path reports.
- Inventory regenerated: 277 leaves, **56 available**, 175 state-dependent, **38 unavailable**, 8 informational (was 40).
- App suite 622 passed / 4 known device-dependent roll failures / 9 ignored (635 tests).

## Not claimed

No desktop pass over the dialog. Reconstruct does not re-run pitch analysis or regenerate anything that is not derived from the audio sources.
