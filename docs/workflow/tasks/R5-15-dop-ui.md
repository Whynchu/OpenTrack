# R5-15 — Direct Offline Processing UI

- Status: accepted
- Scope version and requirement IDs: revision 5, step 15
- Desired observable outcome: Audio > Realtime Processing > Direct Offline Processing edits one event's chain in a window, saving it as one history step and applying it as a second.
- Reference evidence (IDs/paths): `ChainDraft`, `open_offline_processing`, `show_offline_processing`, `save_offline_chain_draft` in `src/app/offline_processing.rs`.
- Dependencies and decisions needed: R5-13, R5-14 (accepted).
- Code scope and architecture boundaries: one window, one menu row; the render path is the one from R5-14.
- Acceptance checks: the window refuses without a selection; draft edits do not touch the project until Save; Save commits one "Edit Offline Chain" step; an invalid draft is refused; Apply saves and bakes with its own step.
- Affected regressions: the Realtime Processing entry moves from unavailable to state-dependent.
- Essential gates: undo per content change, no hidden bake, chain bounded to eight.
- Review owner and milestone stop: self-review; desktop layout review pending.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: the draft holds `(clip_id, chain)`; rows expose the single numeric parameter for gain/normalize/resample/stretch and a description for parameterless processors; reorder and remove act on the draft; "Add processor" respects the eight-entry bound; Save and Apply are separate so a bake is never implicit.
- Commands/checks and actual results: `cargo test -p opentrack the_editor_saves` → passed; app suite 562 passed / 4 known device failures / 9 ignored; inventory regenerated (77 unavailable rows).
- Runtime environment: Kubuntu Linux; window layout not yet reviewed on the desktop.
- Ledger IDs updated: README disabled-row count 77.
- Review findings: none.
- Failures/unknowns/pending acceptance: desktop review of the window and the audible result of a baked chain.
- Next ready task: R5-16.
