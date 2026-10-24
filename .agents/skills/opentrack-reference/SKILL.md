---
name: opentrack-reference
description: Resolve named OpenTrack fidelity unknowns using existing Cubase evidence, authorized desktop observation, manuals, or targeted REA inspection. Use for bounded reference studies and specification work.
---

Read `docs/workflow/current.md`, the assigned task, and the measurement contract in `docs/workflow/README.md` from the repository root.

1. State the unanswered questions and evidence needed. Search existing notes/manifests first. Do not repeat a completed sweep.
2. Distinguish user specification, manual text, direct observation, inference, and unknown. Do not convert a user's desired improvement into a claimed Cubase observation.
3. Use current computer-use skill/tool instructions for authorized desktop inspection. Keep desktop interaction coordinated with the user. For REA, inspect only artifacts needed to answer the question; installed binaries remain evidence, not product assets/code.
4. Record action/target, before/after state, display conditions, capture/reference IDs, actual result, and unresolved exceptions. Preserve project files; destructive experiments use copies.
5. Stop once the packet's questions are answered. If tool recovery fails twice without new evidence, record the blocker and continue only independent authorized work.
6. Update the task and ledger with exact evidence. Run `python scripts/parity_report.py`. Hand off a concrete behavior specification and acceptance cases; never claim implementation or parity from reference study alone.
