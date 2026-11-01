# Modular architecture and parallel feature delivery plan

Status: active planning, 2026-11-01. The source-level UI inventory is recorded in `docs/reviews/ui-implementation-inventory.md`; implementation scope remains to be selected.

## Outcome

Make OpenTrack easier to extend while completing real musical workflows. Reduce the responsibilities concentrated in app.rs, establish explicit feature ownership, and use parallel workers where dependencies permit. Retain the existing Rust crates and UI framework; introduce no new dependency for this work.

## Evidence and limits

The inspected app.rs contains approximately 19,761 nonblank lines and is about 853 KB. It includes application state, project history, background jobs, command dispatch, dialogs, window lifecycle and feature orchestration. Existing view modules already provide useful separation; build on them. File size is a symptom: the main concern is unrelated responsibilities sharing broad mutable state.

Source inspection found a generic unimplemented-command fallback (app.rs), planned Preferences pages, missing send DSP (mixer.rs), disabled Channel Settings sections, and unsupported Hardware Direct Monitoring. These are initial findings, not a complete feature inventory or runtime assessment. Historical architecture notes contain outdated capability descriptions; current source and task evidence must govern decisions.

Existing local changes in app.rs, current.md and T070 must be preserved. The latest T070 notes still describe an unresolved plug-in hang. Stabilize or explicitly isolate that baseline before moving its code.

## Proposed boundaries

| Area | Responsibility | Boundary |
| --- | --- | --- |
| App shell | Startup, frame scheduling, composition and shutdown | Calls feature coordinators; holds top-level service ownership |
| Project session | Active project, history, dirty state, selection and edit transactions | One authoritative project and mutation path |
| Application commands | Resolve commands, validate and execute edits, publish effects | Shared by menus, shortcuts, Project, Mixer and editors |
| Feature modules | Transport UI, workspace, MIDI editor, Sample Editor, Mixer integration, preferences | Own transient feature state and expose explicit inputs/actions |
| Services | Project/media I/O, analysis jobs, scan jobs, native-window and plug-in integration | Own lifecycle, cancellation, failure reporting and platform boundaries |
| opentrack-core | Persistent musical data, stable identity and domain operations | No GUI state or device handles |
| opentrack-audio | Sample-clock execution, recording, routing and DSP | Bounded communication; no callback I/O, waits or GUI work |

Proposed app modules can live under src/app/ initially. Extract pure types/helpers first, then give each area cohesive state and narrow methods. Moving methods into files while every method still reaches all of AppData is only an intermediate step. Avoid a generic service framework, event bus, trait for every helper, or one crate per window. Add abstractions only where a real caller or platform boundary requires them.

Views read the state they need and emit explicit actions. Coordinators execute those actions through shared commands/services. Audio playback position comes from the engine snapshot; UI state cannot become a second transport authority. Preserve the existing project format unless a feature actually needs new persistent data.

## Delivery phases

### 0. Establish the baseline and exact backlog

1. Record source identity, local changes, build configuration and unresolved failures; preserve ASIO support. This baseline was inspected during inventory: dirty app.rs/current.md/T070 changes and untracked recovery/stress-bench directories pre-exist.
2. Inventory command registrations against dispatch handlers, visible disabled controls and planned pages. Initial source inventory is `docs/reviews/ui-implementation-inventory.md`; menu command mapping and runtime dropdown/window review remain. Record each as working, partially implemented, unavailable, or intentionally deferred, with source and runtime evidence kept distinct.
3. Rank by musical outcome, dependencies, risk and user priority. Group controls into complete feature slices.
4. Give each selected slice a task packet: behavior, persistence, undo/redo, failures, owned files, interfaces and acceptance checks.

Exit: a reviewable backlog and a known baseline, without implying that the existing application passes all workflows.

### 1. Create room for parallel work

Choose two small extractions after inspecting dependencies: Channel Settings layout/preferences types and command-dispatch organization are candidates, not fixed commitments. First map their callers and state access. Keep app.rs wiring owned by one integrator.

Separate behavior-preserving extractions from feature changes. Preserve public command IDs, project data, defaults, window ownership and lifecycle. Do not relocate the active plug-in hang code until its baseline is understood. Each extraction must leave a compiling, reviewable checkpoint.

Exit: at least two feature areas have clear file ownership and narrow integration points, with affected behavior verified.

### 2. Deliver independent feature slices in batches

The session supports four concurrent agents total. Use one integration/review owner and up to three workers. A backlog of ten tasks is practical; ten simultaneous agents are unavailable here. Small tasks may need fewer agents to avoid coordination overhead.

Provisional queue (scope and order finalized by Phase 0):

| Packet | Deliverable | Dependency / review |
| --- | --- | --- |
| P01 | Command/control inventory and prioritized backlog | Read-only audit first |
| P02 | Channel Settings configuration extraction | Check callers and persistence |
| P03 | Command dispatch boundary | Integrator owns app.rs changes |
| P04 | One agreed Preferences family, wired to consumers | P01; select behavior with user |
| P05 | One bounded unimplemented command family | P01 and P03 |
| P06 | One agreed Channel Settings section | P01; depends on real processing support |
| P07 | Typed routing/send design and acceptance packet | Map current graph and project state |
| P08 | Send processing and recall | P07; serialize shared graph changes |
| P09 | One next workflow selected from inventory | Automation, takes or media only after scope is specified |
| P10 | Integrated regression, runtime review and documentation | Runs after each relevant batch as well as final milestone |

These packets are scheduling units, not ten equal-sized jobs. P08 and any automation work may require several separately reviewed stages. Hardware Direct Monitoring and CPR interoperability require their own capability/evidence plans and should not be assigned as quick button implementations.

### 3. Continue modularization through feature work

When a feature needs a crowded area, extract the smallest cohesive boundary needed for that feature. Prioritize project session/history, workspace/window lifecycle, background jobs and MIDI-editor orchestration based on actual collision and defect evidence. Keep app.rs as the composition point. Use responsibility and dependency quality as success criteria rather than an arbitrary line limit.

## Agent and integration rules

- One worker owns each file set. Use isolated worktrees/branches from the recorded baseline; handle the existing dirty work explicitly before creating implementation branches.
- Agree types and interfaces before dependent workers start. Reserve app.rs and shared schema/audio-graph files for the integration owner unless ownership is temporarily transferred.
- Workers provide a focused diff, checks, source identity, limitations and integration instructions. They do not independently merge or publish.
- Integrate one change at a time in dependency order. Inspect the actual diff and rerun affected checks against the integrated source.
- Run a full release build after a batch rather than competing release builds in every worker. Preserve the Windows local-ASIO configuration.
- Stop or reassign a lane when it encounters a shared-file dependency; do not invent parallel work that increases merge risk.

## Verification and completion

Behavior-preserving extractions: compile, affected meaningful existing tests, focused regression checks and save/reopen where serialization or ownership is touched. Do not rewrite golden results merely to fit moved code.

Feature slices: demonstrate the complete control-to-command-to-runtime path, undo/redo and persistence where applicable, errors/cancellation and relevant boundary cases. Audio changes need offline signal checks plus listening/hardware review appropriate to the feature. Desktop interactions need running-app review. Compilation alone is not feature acceptance.

Track implementation, automated evidence, runtime evidence and user acceptance separately in each task packet. Update current.md when implementation begins; this proposal does not replace the active assignment. Publish only under applicable user authorization.

## First milestone status\n\nSource-only inventory started and recorded at `docs/reviews/ui-implementation-inventory.md`. Confirmed explicit gaps include disabled menu commands, multiple planned Preferences pages, three unimplemented Channel Settings layout sections, and disabled Project/device controls. No implementation or runtime review has started. Next: finish command-to-handler mapping, then prepare bounded task packets and select the first feature slice with the user.\n\n## First milestone proposed for approval

Complete P01, select the first two bounded extractions, and prepare three concrete feature packets with file ownership and acceptance criteria. Review that result before broad feature implementation. This makes the first architectural change small enough to assess and establishes a repeatable delivery process.


