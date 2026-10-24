# Project instructions

## Product goal

OpenTrack- is a standalone Rust DAW for x64 Windows. Prioritize usable musical workflows close to Cubase 5, with deliberate improvements and our own colors. Match interaction, structure, positions, hierarchy, and aesthetic choices; functional completeness comes first and polish continues alongside it. Bundled Cubase plug-in recreation is deferred and is not required to be 1:1. Future effects/instruments are individually designed with the user; purchased Waves products may be studied when requested. Third-party plug-in hosting is a separate future requirement. REA is an evidence tool, not the product. Keep goals separate from demonstrated behavior.

## Start and resume here

- Read [current state](docs/workflow/current.md), then the [delivery workflow and user intent](docs/workflow/README.md). These supersede historical scope/status passages in review reports and fidelity-plan.md. Current user instructions supersede all repository guidance.
- Use the active task packet and its linked evidence; do not reread every historical capture or repeat a completed study without a named unresolved question.
- Follow the [architecture agreement](docs/workflow/architecture-agreement.md) incrementally. Do not perform a broad refactor to avoid delivering the active workflow.
- Maintain `docs/workflow/ledger.json`; run `python scripts/parity_report.py` after status changes and before handoff. Never substitute code presence or compilation for runtime, reference, or user acceptance evidence. Report scope version and unknowns with percentages.
- Reusable workflows live in `.agents/skills/opentrack-reference`, `opentrack-implement`, and `opentrack-review`. Read the applicable SKILL.md even if automatic discovery is unavailable.
- Luna Max executes bounded, specified tasks; Astra/Sol resolve architecture or ambiguous behavior; Sol or stronger reviews visual/interaction fidelity. These are routing preferences based on user experience, not capability guarantees. Record the actual model; do not claim to switch models when unavailable. See workflow rules for review and escalation.

## Reverse-engineering boundaries

- Keep first-party application, audio engine, compatibility, and plug-in rewrite code in Rust. Document OS APIs and external plug-in ABI boundaries.
- Use the reference application, local Operation Manual, and REA to record observable behavior, file compatibility, commands, and plug-in behavior. Distinguish observations, inferences, and unknowns.
- Keep the reference install and analysis tools outside this repository. Keep OpenTrack- source, notes, and original generated assets inside it.
- Use original code and fresh visual assets. Treat installed Cubase files and plug-in binaries as evidence, not material to copy or ship.
- The primary application target is x86_64 Windows. Isolate compatibility work behind crates or process boundaries. An optional legacy 32-bit plug-in helper is a separate compatibility feature.

## Compatibility and implementation

- Follow [the fidelity plan](docs/fidelity-plan.md): observe a complete reference workflow, specify it, implement it, and compare it before expanding it.
- Treat source inspection, compilation, manual documentation, runtime observation, and verified parity as different evidence levels. A successful build does not prove workflow or visual fidelity.
- Every fidelity batch must name its reference evidence and acceptance steps. Unmeasured layout and behavior stay unknown; do not invent defaults and label them Cubase behavior.
- Stop at the review landmarks in the fidelity plan for user involvement. Do not begin the next substantial UI batch until the current review is resolved.
- Keep only one OpenTrack instance open. Close it through its UI before rebuilding and at the end of each desktop review.
- A blank Sky inventory or zero `MainWindowHandle` does not prove OpenTrack is closed. Recover or reset the direct Node/Sky session and check processes before launching another instance.

- Do not claim CPR read/write compatibility until real project files and round-trip behavior have been verified. The OPR (OpenTrack Project) format is OpenTrack's own format; earlier OTP and legacy RWP projects remain readable.
- Preserve accepted Mixer work. Compressor and new plug-in design are outside the current Project/transport slice. The user authorized committing and pushing the current development checkpoint on 2026-10-24, including the workflow and README screenshot. Publication is not M2 acceptance; subsequent feature acceptance still follows review checkpoints.
- Do not add dependencies or framework choices until they have been evaluated against product requirements.
