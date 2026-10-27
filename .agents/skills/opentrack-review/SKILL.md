---
name: opentrack-review
description: Review an OpenTrack implementation against its task specification, actual runtime behavior, and architecture constraints. Use for feature behavior or integration review; external references are optional task inputs.
---

Read `docs/workflow/current.md`, the task, `docs/workflow/architecture-agreement.md`, and feature verification guidance in `docs/workflow/README.md`.

1. Inspect the actual diff and supporting evidence before adopting the worker's conclusions. Record the actual reviewer model. Sol or stronger is preferred for interaction review; Astra/Sol for timing, threading, and persistence.
2. Check requirement by requirement. Trace UI controls to their state and processing consumers. Require audible output/persistence checks where applicable; builds and toggles alone are insufficient.
3. Exercise the feature in OpenTrack under relevant runtime conditions. Check edges, hit regions, context menus, hover timing, drag transitions, and accepted Mixer behavior when affected. Use external examples according to the task specification and record user-directed choices.
4. Reject missing, stale, or mismatched evidence. After changed behavior, invalidate relevant prior results. Never mark a user acceptance on the user's behalf.
5. Update task findings and the ledger with behavior, verification, limitations, and user acceptance status. Runtime passes do not substitute for the task's required listening, device, or user review.
6. Deliver the review build and supporting evidence when applicable. Screenshots may be supplied for a task but are not archived by default. Close OpenTrack after desktop review and stop at the user milestone checkpoint. If live inspection is unavailable, leave it pending explicitly.
