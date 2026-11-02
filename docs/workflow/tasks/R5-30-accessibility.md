# R5-30 — Accessibility and keyboard pass

- Status: accepted
- Scope version and requirement IDs: revision 5, step 30
- Desired observable outcome: commands whose features now exist are dispatched, dialogs do not leak keystrokes to the arrange, and the accessibility report reflects the real numbers.
- Reference evidence (IDs/paths): `handle_shortcuts`, `modal_dialog_open`, `dialog_owns_input` in `app.rs`; `docs/workflow/accessibility-2026-11-02.md`; the generated inventory.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: one guard in the shortcut path plus the editors added to the modal set; report regeneration.
- Acceptance checks: Delete removes the selected event with no dialog open but changes nothing while the Regions or tempo editor is open; the report lists 149/152 dispatch with three named gaps; inventory regenerated.
- Affected regressions: any shortcut that previously worked while a dialog was open is now suppressed — which is the intended rule.
- Essential gates: no destructive shortcut reaches the arrange from an editor.
- Review owner and milestone stop: self-review; a manual keyboard-only pass remains pending.

## Findings

- `handle_shortcuts` already returned when egui wanted keyboard input, so typing in text fields was safe. It did **not** consult `modal_dialog_open()`, so with the Regions editor open a Delete keypress deleted the selected audio event behind the window. That is now fixed, and the Regions and Arranger editors were added to the modal set.
- 149 of 152 bundled command IDs dispatch. The three remaining are Video, Open Score Editor and Use External Sync, each blocked by an absent feature rather than wiring (score engine, video decoder, clock source).

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: guard added; editors registered as modal; new test presses Delete with each editor open; report and inventory regenerated; README counts updated.
- Commands/checks and actual results: `cargo test -p opentrack a_dialog_that_owns_input` → passed; app suite 580 passing with the four known device failures; accessibility report 149/152.
- Runtime environment: Kubuntu Linux.
- Ledger IDs updated: accessibility report and README.
- Review findings: the report had drifted to 147/5 and needed regeneration, which is why this pass exists.
- Failures/unknowns/pending acceptance: a manual keyboard-only walk of the new editors on the desktop.
- Next ready task: R5-31.
