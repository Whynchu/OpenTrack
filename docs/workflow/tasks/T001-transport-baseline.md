# T001 — Transport specification and baseline

Status: ready. Scope: transport-v1, all ledger requirement IDs. Preferred owner: Astra/Sol.

Outcome: a bounded source/evidence audit that makes T002 ready and identifies the implementation seams and unanswered questions for the remaining transport packets. This is an audit, not approval to redesign the application.

Read current.md, architecture-agreement.md, Project interaction study and checklist. Use existing captures/manifests first. The user's 2026-10-24 directions are the specification for normal drag, release snap, directional range selection, colors, and adaptive labels. They are not automatically measured Cubase evidence.

Inspect show_project, transport clock/seek/cycle consumers, core range state, grid selection, persistence and dirty tracking. Record precise source locations and actual implementation status per ledger requirement. Existing code may already fulfill some requirements; do not rewrite working behavior without evidence.

Questions: which snap settings govern ruler release, tie behavior, and clamping; which normal ruler hit zone excludes range drawing and scrollbar; how zoom anchor and seek update together; whether skip is enabled by Cycle and what playback does at each endpoint; whether range endpoints snap; how playback follow differs from drag-edge scroll. Resolve only questions needed for the next packet. Existing user-reported drag scrolling is a regression obligation, not proof for the current build.

Acceptance: each requirement has an assessed source status with evidence, or an explicit unresolved reason; T002 contains concrete grid semantics, code seams, and fixtures; broader unknowns have named follow-up packets. Do not set baseline_complete unless every implementation row is assessed. No whole-DAW percentage. Update ledger/report and current.md. Live observations require the current computer-use workflow and preserved reference project. Do not repeat the full Cubase sweep.
