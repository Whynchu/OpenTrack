# Workflow setup review — 2026-10-24

Implemented by Astra in response to the user's request to make goals and metrics durable. No application source or project file was edited by this setup pass; pre-existing product changes remain in the working tree.

Delivered: updated AGENTS.md routing and product scope; one current-state entry point; user-intent attestation; incremental architecture agreement; model roles and bounded queue/escalation rules; three local skills; task template; ready T001 audit and dependent T002 snap pilot; transport-v1 ledger and deterministic reporting tool. Historical architecture/plugin/fidelity notes now point to the updated scope.

Validation:

- `python scripts/test_parity_report.py`: six tests pass, covering incomplete baselines, runtime evidence identity, reference parity evidence, essential gates/scenario readiness, unknown denominators/stale results, and workflow membership.
- `python scripts/parity_report.py --check`: passes; generated report is current.
- `git diff --check`: passes (line-ending warnings only).
- Three skill frontmatter/name/description/scaffold checks pass; workflow entry-point links resolve.
- Bundled Skill Creator quick_validate.py could not execute because PyYAML is not installed in the local Python environment. Simple frontmatter was checked directly; no claim that the bundled validator passed or that realistic unattended execution has been trialed.

The reporting tool validates declared evidence structure and calculations; independent review must still inspect the cited artifacts and invalidate stale results. It cannot establish truth from a path alone. Roles are instructions, not an installed scheduler or guaranteed automatic model switch. Repository skills are discoverable where supported; AGENTS.md also routes agents directly to the skill files.

The baseline is intentionally incomplete. Transport-v1 includes 14 requirements across five workflows and four essential gates. Its initial zero verified coverage means evidence has not yet been migrated/audited into this ledger, not that existing work is absent or rejected. T001 must incorporate existing evidence before publishing an implementation percentage. No whole-DAW percentage is established.

No app rebuild or desktop session was needed for this documentation/tooling slice. Workflow files are persisted in the repository working tree, pending Git review/commit. Next: T001, then the T002 pilot; validate the process on that bounded task before a long unattended queue.
