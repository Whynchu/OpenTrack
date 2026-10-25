# OpenTrack delivery workflow

## Recorded user intent — 2026-10-24

This is the maintained record of the user's directions, not a claim that the product already meets them. Amend it when the user changes direction; record the change in current.md rather than accumulating competing current instructions.

- Build a usable professional Rust DAW, prioritizing complete musical workflows, dependable behavior, and near-identical Cubase Project flow. Improve the design deliberately with the user. Continue polishing as functionality develops.
- Preserve our preferred colors; match positions, grouping, hierarchy, icons, menus, targets, and interaction where reference evidence exists. Never invent tooltip text or reference behavior.
- Preserve the accepted Mixer. Complete the current Project/transport review before another substantial milestone. Channel Settings work already performed remains reviewable; Compressor is excluded.
- Current work is the first T012 Windows audio implementation and T013 managed-project-folder slice. First hardware target is the Mackie Onyx Producer 2-2 USB. ASIO device I/O and VST3 plug-in hosting are distinct; the current path uses native WASAPI and does not require an ASIO vendor agreement.
- Audio direction: lowest stable latency with other apps audible by default; native WASAPI shared low-latency mode, explicit compatible fallback, exclusive mode opt-in. Current source includes device selection, input testing, basic per-track input/output selection, monitoring, and recording. The old build's inputs were user-reported unusable; verify Onyx on the new build. Measure actual hardware roundtrip and stability rather than treating buffer size as latency or build success as acceptance.
- T014 update: add an optional local backend for the installed Mackie ASIO Driver while retaining WASAPI for standard/shared use. The user wants to connect to their existing driver, not package its binaries. Restore Devices → Device Setup and VST Connections/F4; put recording bit depth/sample rate in Project Setup. Support selectable 16-bit PCM, 24-bit PCM and 32-bit float, including 48 kHz and supported additional rates. Display active device names and actual frame/buffer settings. Correct meter lag/background freeze and the missing Project meter beside its color strip. Supplied screenshots establish hierarchy and layout; hardware capability and meter timing remain separate measurements.
- T014 review corrections: raw Stereo In meters remain active independently of track monitoring; track L/R meters follow pan/gain and Stereo Out meters the actual sum. Mono defaults to a mono physical input, stereo exposes the full Stereo In choice, and all views share routing. Existing-project sample rate/record encoding must be discoverable and editable. Configuration windows follow the supplied tree/page and compact tabbed table structures; source or compilation does not establish visual parity.
- T015 direction: Stereo In/Out need independent persistent Mixer controls and VST Audio Channel Settings wired to real audio, including input processing for new recordings and final-output processing. Device Setup should be roughly half its earlier height, with scrolling for longer pages. Track behavior and the accepted Mixer layout remain protected. Unsupported bus automation, sends and control-room operations must be clearly unavailable.
- T016 direction: undoing a new recording removes its recorded WAVs as well as events; preserve redo through temporary recovery and never delete imported media. Draw growing signed per-channel waveforms during recording and detailed continuous shapes that reveal more detail at close zoom.
- New projects create a named project folder containing the `.opr`, `Audio/Recordings`, `Audio/Imports`, and `Metadata`. Older projects prompt to reuse their current folder or choose/create another one. Recording picks a numbered take path automatically; external WAV imports are copied into the project. The `.opr` filename controls the main-window title. Keep migration/runtime validation on a disposable copy of EXAMPLE.opr.
- A fresh launch shows the menu bar without opening a project or Project/Mixer/transport panels. File → New uses the native Windows folder picker for the parent location; older or misplaced projects prompt for a current, new, or existing project folder. Canceling the folder step returns to the no-project shell.
- Cubase bundled effects/instruments need not be recreated 1:1. Their development is deferred. Approach future plug-ins individually with the user, potentially studying purchased Waves products. Hosting third-party plug-ins remains a separate functional requirement.
- Ruler: seek continuously under the pointer; snap the cursor to the selected grid on release; allow horizontal scrub and vertical zoom together without an exclusive direction lock. Down zooms in, up zooms out; preserve the anchor. Middle drag pans.
- At the top ruler zone, show the observed pencil-like range cursor. Left-to-right draws a purple repeat region; right-to-left draws a red/orange skip region. Loop control must affect actual playback. Exact skip playback and range-endpoint snapping remain unmeasured until investigated or explicitly designed.
- Over an existing range in that top zone, show a hand. Dragging translates the entire range while preserving its length, direction/mode and pointer grab offset. Outside the range, retain pencil drawing. This was explicitly requested in the T005 follow-up.
- Corner triangles resize their respective left/right boundaries with the opposite boundary fixed. The user confirmed extension works. Keep the native pointing hand on range hover, but switch to the native four-way move cursor while moving a range or middle-button panning. The user rejected the custom closed hand; remove it (T007 supersedes T006 cursor styling).
- As the Project zoom increases, refine the ruler/grid from bars to quarter notes, eighths and sixteenths, with partial-measure labels when readable. Reverse that progression and retain wider bar strides when zooming out. Display subdivisions do not silently change the selected Snap grid (T007).
- Grid Type and Quantize Type are functional Project controls (T008). Bar snaps to measures, Beat to quarter-note beats under the existing time model, and Use Quantize to the selected straight/triplet/dotted note division. The same enabled snap interval governs seeking and range drawing/moving/resizing. Choices persist with workspace preferences; Snap off remains free movement.
- Working toolbar fields use strong enabled text rather than disabled-looking gray. Panel buttons reflect real open/active state in blue and toggle closed/off on the next click, with tooltips describing the next action (T009). Preserve the previously requested purple/orange Cycle state colors. Use a conventional black object-selection pointer and consistent original vector power/tool symbols. The user rejected visually awkward toolbar glyphs: keep icons simple and elegant, with consistent light strokes, compact silhouettes and clear spacing (T009 follow-up).
- Clickable and directly draggable UI controls should show a consistent, subtle hover response before activation across Project, Mixer, menus, and VST Audio Channel Settings. Preserve each control's active, disabled, recording, and other semantic state colors (T010). The File/Edit application menu should use the installed Windows Segoe UI TrueType font at a clear whole-point size.
- Preserve both drag-edge auto-scroll and playback-follow scrolling as separate requirements. Hold at either edge to keep scrolling; move away to stop; preserve pointer/playhead alignment. Existing drag scrolling was user-reported working, not an unfixed bug by default. Navigation continues beyond bar 50 and uses available width. Ruler labels/ticks adapt to zoom.
- Preserve separate grid/track/background context menus, scrollbar and pinned-header behavior, and existing Project track/Inspector requirements in the linked checklist.
- New project files use `.opr`; opening also supports `.otp` and `.rwp`. Title follows the filename (`Project- EXAMPLE` for EXAMPLE.opr). Closing an unchanged project must not request saving. The user authorized saving EXAMPLE when closing, but use a disposable copy for destructive verification and preserve the original.
- Keep one OpenTrack instance. Close it through its UI before building and at the end of a desktop review. A missing window inventory does not prove the process is absent. Respect current desktop authorization and coordinate desktop sessions with the user's activity.
- Prefer dependable bounded work suitable for extended Luna Max execution. Reserve architecture, ambiguous behavior, and visual judgment for stronger review. Preserve the objective across handoffs and compaction.

## Authority and memory

Current user instructions > AGENTS.md and this intent record > current.md and active task > historical reports. Historical observations remain evidence, but historical restrictions or build hashes are not current instructions. Keep current.md concise and replace stale status there. Record decisions with their reasons in task reports.

## Task execution

1. Read current.md, the assigned packet, its evidence, and relevant local instructions. Check Git status; preserve existing changes. Name the intended change and expected result.
2. If behavior is ambiguous, use a bounded reference question. Stop studying once answered. User-specified behavior can be implemented without claiming it was measured in Cubase. REA serves concrete unanswered questions; it is not a prerequisite for ordinary interaction code.
3. Before product edits, check the existing build and process state. Use `./build.ps1 -Architecture x64` for the release deliverable. Documentation/tooling-only tasks do not require an app rebuild or desktop session.
4. Implement one packet. Change necessary foundations only. Review boundaries and introduce no unexplained dependencies. Use meaningful tests for time conversion, transport boundaries, persistence, undo/dirty state, and other failure-prone behavior.
5. Run the packet's checks. For UI/audio work, compilation alone leaves live acceptance pending. Record exact build/source identity, environment, steps, expected/actual results, and artifacts. Re-run affected regressions after changes.
6. Update the ledger, regenerate the report, and write a handoff. A bounded worker may finish a prepared queue, but a milestone review remains a stop boundary. Commit/push only within explicit user authorization; a publication checkpoint does not establish milestone acceptance.

## Task packet contract

Use tasks/TEMPLATE.md. Each packet names its requirement IDs, desired behavior, evidence and unknowns, permitted code scope, architecture boundaries, acceptance checks, regressions, and completion conditions. A task is ready only when dependent decisions and executable expectations are clear. Keep infrastructure work bounded to enabling the current slice.

Workers do not author their own acceptance criteria after seeing the result. Amend criteria with an explicit reason and reviewer decision. Independent review examines the actual diff, test results, screenshots/audio, and evidence, not just the worker's summary.

## Roles and unattended work

| Role | Preferred assignment | Responsibility |
| --- | --- | --- |
| Specification/architecture | Astra or Sol | Resolve ambiguity; establish interfaces and acceptance criteria |
| Implementation | Luna Max for bounded tasks | Implement, check, document, and checkpoint |
| Visual/interaction review | Sol or stronger | Inspect actual behavior and comparison artifacts |
| Timing/threading/persistence review | Astra or Sol | Review audio boundaries, real-time constraints, recovery and data integrity |
| Milestone acceptance | User | Approve demonstrated workflow and remaining explicit limitations |

These preferences reflect the user's experience, not universal model rankings. Use the actual available model and report it honestly. Model switching, scheduling, and subagent availability depend on the session. This file alone does not launch agents or grant additional permissions. If stronger review is unavailable, mark review pending and continue only independent approved work.

For an extended run, select one ready task at a time. Checkpoint after each: source/build identity, changes, verification, evidence, blockers, and next ready task. After two attempts with the same failure and no new evidence, stop that approach, document the blocker, and escalate or take an independent ready task. Do not loop through captures, broaden scope, or invent semantics to remain busy. Do not operate a shared desktop while the user is using it; code and headless checks can continue.

Record actual model, elapsed effort if available, review findings, rework, and task acceptance in each handoff. Use this evidence to revise role allocation. Do not assume a long run or high reasoning setting establishes quality.

## Measurement contract

`ledger.json` is the machine-readable source; `python scripts/parity_report.py` validates it and generates metrics.md. Schema version and scope version are separate. The initial scope is transport-v1, not the whole DAW. Its baseline is incomplete until source/build/runtime evidence has been audited. No overall DAW percentage may be inferred from it.

Weights: core workflow 3, common requirement 2, polish 1. Split requirements into observable atomic outcomes. Freeze membership/weights per scope version; increment the scope version and explain changes when adding/removing/reweighting requirements. Do not remove failures to improve a score.

For W = total requirement weight:

- **Implementation coverage:** sum(weight * source completeness) / W. Source completeness is absent=0, partial=0.5, complete=1. Unassessed is shown separately and suppresses this headline until the baseline is complete. Code evidence is mandatory for partial/complete. Compilation is separate evidence.
- **Runtime validation:** weight of passing requirements / W. Pass requires execution evidence tied to a build/source identity. Partial, failed, pending, and stale do not count as passes.
- **Workflow readiness:** sum(workflow weight for workflows whose requirements all pass runtime checks and whose essential gates pass) / total workflow weight. All workflows also require a recorded end-to-end scenario pass. Applicable audible results, save/reopen, undo/dirty state, and failure cases belong in those scenarios. This is readiness for review; user acceptance is separate.
- **Confirmed reference parity:** weight of requirements with a documented reference match / W. A match requires observed reference evidence and a passing OpenTrack runtime result. User requirements/manuals alone establish expectations, not a measured Cubase match. Intentional improvements can pass functional acceptance while remaining documented differences.
- **Unknown reference coverage:** weight without measured Cubase reference evidence / W. Report user-specified/manual/inferred/unmeasured separately from observed. Unknowns stay in the denominator. Reference coverage is its complement.

Also list failed/pending/stale results and accepted workflow count. A build, toggle state, screenshot, or worker self-report cannot stand in for audible, persistence, or interaction evidence. Any affected code or spec change invalidates relevant runtime/parity/scenario results to stale until rechecked. A scope with blocked essential gates cannot be called professionally ready regardless of its average score.

The broader DAW inventory will cover project editing and persistence/recovery; recording/routing/monitoring; MIDI; automation; external plug-in hosting; export; and performance/reliability. Those areas need their own inventories and essential gates before an overall denominator exists. Deferred bundled plug-in cloning is excluded.

## Review artifacts

Store current screenshots and comparisons under `G:\dev\Opentrack\_analysis-tools\reports\opentrack-comparisons\`, outside Git. Coverage rows record resolution, known scaling, window positions, panels, theme, reference ID, current image, source location, action/shortcut, and match status. Use primary `*-0.png` captures and `04-preferred-approved-0.png` for the approved overall layout; honor manifest exceptions. Show review images by absolute path in chat. Never claim parity solely from a build.

- T017 user direction: waveforms use their full lane height while retaining actual amplitude. Closing with Discard deletes new recordings made since the last save, preserves saved/imported audio, and cleans undo recovery. The user authorized pushing the current development checkpoint.
