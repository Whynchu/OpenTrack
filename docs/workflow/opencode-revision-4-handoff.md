# OpenCode runner handoff: roadmap revision 4

Paste the following into OpenCode with the requested DeepSeek V4.1 Flash model selected. Use the actual provider/model identifier available in that installation; this handoff does not configure or launch OpenCode.

---

Continue OpenTrack development from docs/workflow/roadmap-32-steps-v4.md. Read that whole plan, AGENTS.md, docs/workflow/README.md, docs/workflow/architecture-agreement.md, and .agents/skills/opentrack-implement/SKILL.md plus opentrack-review/SKILL.md first.

The intended runner is DeepSeek V4.1 Flash. Record the actual model identifier. Planning baseline is 12a01e0, but inspect the current Git state before work and reconcile newer commits. Preserve all local changes and the untracked recovery/stress-bench directories.

Execute the numbered 32-step queue serially, starting with inventory reconciliation and review of the recently added media operations. Before implementing each step, create its task packet from the repository template with fixed acceptance criteria and exact source ownership. Keep the new feature in focused Rust modules; app.rs should compose it. Implement real consumers, atomic undo, cancellation, failure handling and backward-compatible persistence before enabling menu entries. Do not add dummy shortcut arms or report source coverage as runtime verification.

After each slice run its meaningful checks, update README/current.md/inventory/task evidence, and make a focused local commit. Build product changes with the platform script, retaining Windows x64 local ASIO. Documentation-only steps need no release build. Do not push unless I explicitly authorize publication for this run. Do not launch more than one app instance. Record unavailable runtime/hardware checks and continue ready independent work; stop dependent work when a correctness or architecture gate fails.

Follow the plan's exact bounded behavior and readiness gates. New schema/timing/graph decisions require a recorded architecture decision and the required review, not a guessed substitute. Never claim user acceptance or an independent reviewer that did not occur. If blocked, name the missing decision/fixture/device and continue another ready step. At step 32 reconcile outcomes and stop this revision; do not grow it into an unlimited sequence of extra slices.

Begin by reporting the actual source/model identity, Git status, measured remaining menu and shortcut gaps, and the first ready packet, then perform the work.

---

Publication is authorized for the remaining revision-4 run: push each completed, verified feature to origin/main, as confirmed in the user decisions below.

## User decisions for the remaining run (2026-11-02)

- The user authorized publication after each completed feature in this run. Commit and push each verified feature slice to `origin/main`; stop and report if a push fails.
- Any project-content adjustment should be undoable by default. Selection and view-only state should not create project-content history.
- CC Automation Setup shows one selected controller lane at a time, persists the lane-view choice in the project, and makes CC data edits undoable.
- FX channels are shared effect returns that accept sends from multiple tracks; reject feedback cycles.
- Arranger section and sequence edits are undoable; preview is playback-only, with recording and punch unavailable during preview.
- Folders are organizational only; collapsing a folder does not change playback or recording.
- If a proposed change would alter already implemented behavior, hold that change and ask the user to review it before implementation.
- The user accepted the recommendations above; continue the queue and ask only when a concrete unresolved choice or existing-behavior conflict requires input.
