# Project instructions

## Product goal

OpenTrack is an open-source Rust DAW for x64 Windows. Build dependable features around the user's musical workflows: recording, editing, arranging, mixing, and finishing music. Specify and review each feature by its behavior, persistence, audio/runtime constraints, failure cases, and user-visible acceptance. Use the running OpenTrack application as the primary design surface. Screenshots supplied for a task are optional inputs; do not retain screenshots or comparison captures in the repository by default.

Project interchange is a product capability. Support other DAW project formats through format- and version-specific adapters. Map supported concepts into OpenTrack, preserve unsupported structures and metadata as unchanged opaque stubs where the format permits, and report unsupported content. Make compatibility claims only after exercising real files and round trips for the named format and version. Feature ideas from other software can inform a specific task when useful.

Design first-party effects and instruments individually with the user and implement them as separately loadable VST3 plug-ins. Third-party VST3 hosting has documented compatibility limits; do not claim universal support. Keep goals separate from demonstrated behavior.

## Start and resume here

- Read [current state](docs/workflow/current.md), then the [delivery workflow and user intent](docs/workflow/README.md). These supersede historical scope/status passages in review reports and fidelity-plan.md. Current user instructions supersede all repository guidance.
- Use the active task packet and its linked evidence; do not reread every historical capture or repeat a completed study without a named unresolved question.
- Follow the [architecture agreement](docs/workflow/architecture-agreement.md) incrementally. Do not perform a broad refactor to avoid delivering the active workflow.
- Record active feature scope, implementation status, verification, user review, and known limitations in the task packet and `docs/workflow/current.md`. Never substitute code presence or compilation for runtime or user acceptance evidence. Choose verification that demonstrates the named feature's behavior and persistence.
- Use `.agents/skills/opentrack-implement` and `opentrack-review` for feature delivery. The optional `opentrack-reference` guide applies only when a task calls for a specific external example or document; it does not set a parity target. Read applicable SKILL.md instructions even if automatic discovery is unavailable.
- Luna Max executes bounded, specified tasks; Astra/Sol resolve architecture or ambiguous behavior; Sol or stronger reviews visual/interaction fidelity. These are routing preferences based on user experience, not capability guarantees. Record the actual model; do not claim to switch models when unavailable. See workflow rules for review and escalation.

## Implementation and interoperability

- Keep first-party application, audio engine, compatibility, and plug-in rewrite code in Rust. Document OS APIs and external plug-in ABI boundaries.
- Define new behavior from the user's workflow and the current OpenTrack application. Consult another DAW or external material only when it answers a specific feature or format question.
- Treat screenshots supplied by the user as task inputs. Do not archive them or derived comparison images by default; retain them only when the user explicitly asks.
- Use original code and fresh visual assets. Do not copy or ship another application's code, binaries, or visual assets.
- The primary application target is x86_64 Windows. Isolate compatibility work behind crates or process boundaries. An optional legacy 32-bit plug-in helper is a separate compatibility feature.

## Feature acceptance and compatibility

- Every feature task names the intended user outcome, expected behavior, persistence, important edge cases, and suitable verification. Review behavior in OpenTrack; use screenshots only when supplied for that task.
- Distinguish source implementation, automated checks, runtime behavior, and user acceptance. A successful build does not prove a feature works in the intended workflow.
- Keep user review checkpoints where they resolve a specific feature question. User-provided screenshots are optional inputs when they help the task.
- Keep only one OpenTrack instance open. Close it through its UI before rebuilding and at the end of each desktop review.
- Channel Settings Always on Top is relative to the main OpenTrack Project/Mixer window. Use ordinary window ownership, allowing application dialogs and external programs above it; never use global Windows topmost status for this preference. Keep confirmation dialogs accessible and restore the pin when dialogs close.
- A blank Sky inventory or zero `MainWindowHandle` does not prove OpenTrack is closed. Recover or reset the direct Node/Sky session and check processes before launching another instance.

- Verify CPR and other project-format support against real files and round trips for each named version. Preserve unknown data when the source format permits. The OPR format is OpenTrack's own; earlier OTP and legacy RWP projects remain readable.
- Preserve accepted Mixer work. The user-directed built-in Compressor removal is recorded in current.md; future effect design remains outside the current Project/transport slice. The user authorized committing and pushing the current development checkpoint on 2026-10-24, including the workflow. Publication is not feature acceptance; subsequent work follows its task's review needs.
- Do not add dependencies or framework choices until they have been evaluated against product requirements.

## Sample Editor and pitch/time work

Pitch & Time activation (2026-10-25): pitch analysis starts only when the user clicks Pitch & Time for the current audio event. Opening/importing/playing audio or opening Sample Editor must not start pitch analysis. Persist editable pitch/time intent in the project and retain valid analysis by source identity/range and algorithm/model version in managed project metadata. Save/reopen restores existing analyzed/editable state without repeating analysis. Source changes invalidate affected results; re-analysis is explicit, cancellable and never overwrites newer edits. Waveform rendering and live Channel Settings spectrum are separate existing features.

User direction (2026-10-25): retain the Rust architecture for all first-party Sample Editor, analysis orchestration, segmentation/edit commands and rendering. The user selected Cubase's Sample Editor organization as a feature-specific design input; this does not create a repository-wide comparison requirement. HachiTune and PitchNet are processing research references, especially neural pitch detection; do not transplant their C++/JUCE application. Evaluate pitch detection separately from pitch/time synthesis. Model/runtime ABI dependencies require explicit capability/license/resource evaluation before adoption. Keep one editor workflow, reached by audio-event double-click and Mixer/Sample Editor lower-zone tabs; track-label double-click remains rename. No fake detected notes or visual-only correction controls.

Sample Editor follow-up: omit AudioWarp entirely; ordinary cuts plus Pitch & Time cover the intended workflow. Use OS-owned arrow/tool artwork where available, with explicit fallback when Windows has no stock resource. Transport and snapped cut guides are light gray. Editor transport reads/seeks the same Project position, mono source/track audio draws one waveform and editor presentation retains track color. Review organization and polish in the running OpenTrack application.

Media/editing follow-up: Delete targets selected audio events; a separately selected track row can be deleted when no event is selected. Normal deletion retains persistent Media Pool entries/files. Shift+Delete removes unused source paths with reversible file recovery; protect shared sources, never overwrite a file on restoration, and keep saved-file discard recovery after history trimming. Sample-rate changes review mismatched media and explicitly confirm non-destructive playback conformity, retaining originals/timing/pitch. Sample Editor has its own persisted finer snap settings, compact left-aligned fields, empty-space navigation and combined ruler seek/zoom/middle-pan/edge-scroll. Scissors artwork must anchor to the actual snapped cut guide, not lag at raw pointer coordinates.
