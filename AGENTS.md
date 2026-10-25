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
- Channel Settings Always on Top is relative to the main OpenTrack Project/Mixer window. Use ordinary window ownership, allowing application dialogs and external programs above it; never use global Windows topmost status for this preference. Keep confirmation dialogs accessible and restore the pin when dialogs close.
- A blank Sky inventory or zero `MainWindowHandle` does not prove OpenTrack is closed. Recover or reset the direct Node/Sky session and check processes before launching another instance.

- Do not claim CPR read/write compatibility until real project files and round-trip behavior have been verified. The OPR (OpenTrack Project) format is OpenTrack's own format; earlier OTP and legacy RWP projects remain readable.
- Preserve accepted Mixer work. Compressor and new plug-in design are outside the current Project/transport slice. The user authorized committing and pushing the current development checkpoint on 2026-10-24, including the workflow and README screenshot. Publication is not M2 acceptance; subsequent feature acceptance still follows review checkpoints.
- Do not add dependencies or framework choices until they have been evaluated against product requirements.

## Sample Editor and pitch/time work

Pitch & Time activation (2026-10-25): pitch analysis starts only when the user clicks Pitch & Time for the current audio event. Opening/importing/playing audio or opening Sample Editor must not start pitch analysis. Persist editable pitch/time intent in the project and retain valid analysis by source identity/range and algorithm/model version in managed project metadata. Save/reopen restores existing analyzed/editable state without repeating analysis. Source changes invalidate affected results; re-analysis is explicit, cancellable and never overwrites newer edits. Waveform rendering and live Channel Settings spectrum are separate existing features.

User direction (2026-10-25): retain the Rust architecture for all first-party Sample Editor, analysis orchestration, segmentation/edit commands and rendering. Prefer Cubase-style Sample Editor hierarchy/layout and interactions over HachiTune/PitchNet UI. HachiTune and PitchNet are processing research references, especially neural pitch detection; do not transplant their C++/JUCE application. Evaluate pitch detection separately from pitch/time synthesis. Model/runtime ABI dependencies require explicit capability/license/resource evaluation before adoption. Keep one editor workflow, reached by audio-event double-click and Mixer/Sample Editor lower-zone tabs; track-label double-click remains rename. No fake detected notes or visual-only correction controls.

Sample Editor follow-up: omit AudioWarp entirely; ordinary cuts plus Pitch & Time cover the intended workflow. Use OS-owned arrow/tool artwork where available, with explicit fallback when Windows has no stock resource. Transport and snapped cut guides are light gray. Editor transport reads/seeks the same Project position, mono source/track audio draws one waveform and editor presentation retains track color. Reference organization/polish remains a review requirement, not satisfied by a functional prototype.

Media/editing follow-up: Delete targets selected audio events; a separately selected track row can be deleted when no event is selected. Normal deletion retains persistent Media Pool entries/files. Shift+Delete removes unused source paths with reversible file recovery; protect shared sources, never overwrite a file on restoration, and keep saved-file discard recovery after history trimming. Sample-rate changes review mismatched media and explicitly confirm non-destructive playback conformity, retaining originals/timing/pitch. Sample Editor has its own persisted finer snap settings, compact left-aligned fields, empty-space navigation and combined ruler seek/zoom/middle-pan/edge-scroll. Scissors artwork must anchor to the actual snapped cut guide, not lag at raw pointer coordinates.
