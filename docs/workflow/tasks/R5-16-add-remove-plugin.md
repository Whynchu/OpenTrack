# R5-16 — Add and Remove Plug-in for built-in processors

- Status: accepted
- Scope version and requirement IDs: revision 5, step 16
- Desired observable outcome: the two Realtime Processing entries add and remove built-in offline processors on the selected event, with clear refusals and no claim of third-party support.
- Reference evidence (IDs/paths): `append_offline_processor`, `remove_last_offline_processor` in `src/app/offline_processing.rs`; the Realtime Processing menu.
- Dependencies and decisions needed: R5-13, R5-15 (accepted). Third-party VST3 remains insert-based; the packet records that the offline chain is built-ins only.
- Code scope and architecture boundaries: two menu actions using the existing chain save path; no new persistence.
- Acceptance checks: adding appends to the chain as one "Edit Offline Chain" step; removing drops the last processor; empty chains and the eight-entry bound report instead of writing; both refuse without a selection.
- Affected regressions: the two menu rows move from unavailable to state-dependent/available with a submenu.
- Essential gates: undo, bound, refusal messages.
- Review owner and milestone stop: self-review.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: "Add Plug-in" is a submenu of the seven built-in processors; "Remove Plug-in" drops the last entry. Both go through `save_offline_chain_draft`, so validation, history label and locking behave like the window.
- Commands/checks and actual results: `cargo test -p opentrack the_menu_adds` → passed; app suite 563 passed / 4 known device failures / 9 ignored; inventory 41 available, 151 state-dependent, 75 unavailable.
- Runtime environment: Kubuntu Linux.
- Ledger IDs updated: README disabled-row count 75.
- Review findings: none.
- Failures/unknowns/pending acceptance: desktop review of the submenu; third-party offline processing stays unavailable by design.
- Next ready task: R5-17.
