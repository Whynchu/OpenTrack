---
name: opentrack-review
description: Review an OpenTrack implementation against its task specification, Cubase evidence, actual runtime behavior, and architecture constraints. Use for independent fidelity or integration review before milestone acceptance.
---

Read `docs/workflow/current.md`, the task, `docs/workflow/architecture-agreement.md`, and the measurement contract in `docs/workflow/README.md`.

1. Inspect the actual diff and supporting evidence before adopting the worker's conclusions. Record the actual reviewer model. Sol or stronger is preferred for visual/interaction review; Astra/Sol for timing, threading, and persistence.
2. Check requirement by requirement. Trace UI controls to their state and processing consumers. Require audible output/persistence checks where applicable; builds and toggles alone are insufficient.
3. Compare reference and app under recorded display conditions. Check edges, hit regions, context menus, hover timing, drag transitions, and accepted Mixer regressions when affected. Record intentional improvements separately from reference matches.
4. Reject missing, stale, or mismatched evidence. After changed behavior, invalidate relevant prior results. Never mark a user acceptance on the user's behalf.
5. Update task findings and the ledger, then run `python scripts/parity_report.py`. Report failures, unknowns, gates, and scope version alongside metrics. Runtime passes do not imply reference parity.
6. Deliver the review build and absolute-path comparison images when applicable, close OpenTrack, and stop at the user milestone checkpoint. If live inspection or stronger review is unavailable, leave it pending explicitly.
