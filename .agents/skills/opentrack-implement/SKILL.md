---
name: opentrack-implement
description: Implement a bounded OpenTrack task with explicit behavior, architecture boundaries, meaningful checks, and a durable handoff. Use for prepared Project, transport, core, or audio implementation packets.
---

Read `docs/workflow/current.md`, the assigned packet, `docs/workflow/architecture-agreement.md`, and feature verification guidance in `docs/workflow/README.md`.

1. Check Git state and packet readiness. Preserve existing work. Resolve missing dependent decisions from the user's workflow and intended OpenTrack behavior; use external examples only when the task calls for them.
2. Confirm the existing build for product work. Keep one OpenTrack instance, close through the UI before building, and verify process state. Missing Sky inventory alone is insufficient. Use `./build.ps1 -Architecture x64`; retain local ASIO support for this user's Mackie setup. The script retains per-architecture backend selection in builds/<arch>/build-info.json. Verify local_asio is true on delivery; use -LocalAsio explicitly if necessary. A plain cargo build without the feature must not replace the user's ASIO-enabled executable.
3. Implement only the packet and necessary foundations. Shared controls issue shared commands; UI visuals do not stand in for engine functionality. Escalate architecture changes or ambiguous interaction judgments to Astra/Sol when available.
4. Exercise named acceptance checks and affected regressions. Tests should verify meaningful external outcomes, boundary conditions, or invariants. Record live checks as pending when desktop/hardware is unavailable.
5. Mark affected feature-verification results stale after behavior changes. Add evidence with source/build identity and run checks that demonstrate the requested behavior.
6. Hand off using `docs/workflow/tasks/TEMPLATE.md`: actual model, diff scope, checks/results, evidence, remaining gaps, review needed, next ready task. Do not self-approve visual parity or user acceptance. Follow explicit publication authorization independently of milestone acceptance.

For an authorized queue, checkpoint after each packet and continue only ready independent tasks. Two repetitions of the same failed approach without new evidence require escalation or a different ready task. Stop at the user milestone review boundary. Close OpenTrack when desktop review is done.
