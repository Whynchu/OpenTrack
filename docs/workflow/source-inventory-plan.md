# Step 1: complete source inventory

Status: planned, 2026-11-01. This packet covers source inspection only. It is the first step of the broader audit; running-app review, prioritization and implementation are separate later steps.

## Outcome

Produce an item-by-item inventory of OpenTrack's menus, registered keyboard commands, windows, dialogs and selection controls. Give each item a source-backed status and show which areas were inspected, including areas with no gaps found. The existing [UI inventory](../reviews/ui-implementation-inventory.md) is a starting point, not an exhaustive register.

## Baseline

Record the current commit, `git status --short`, and the identity of relevant dirty files at the start and end of the audit. On initial planning, HEAD was `1b61c7a`; app.rs, channel_settings.rs and vst3_host.rs were modified, and the existing inventory, delivery plan, recovery directory and stress-bench directory were untracked. Preserve all existing work. Line numbers and conclusions apply to the inspected source snapshot, not automatically to a later build.

## Coverage and order

1. **Menus and context menus.** Enumerate every main-menu and context-menu entry, including nested lists, generated entries, project-open versus empty-shell menus and conditionally enabled actions. Record the full visible path, source location, enabling condition and action. Cover Project, track/event, Mixer, Channel Settings, editor, MediaBay and plug-in menus. Identify informational empty-state rows separately from missing features.
2. **Keyboard commands.** Enumerate the checked-in Windows keymap rows and generated command IDs, then map each to dispatch branches and relevant context-specific handlers. Record aliases, macros, shortcut context, fallback and any menu equivalent. A menu may bypass the keymap; do not infer a one-to-one mapping from labels. Check collisions and generated ranges against the ID-generation logic.
3. **Windows and dialogs.** Enumerate application and native viewport creation sites and their entry points, including conditionally created and modal windows. For each, list meaningful controls, destinations, disabled states and the actions they invoke. Include Project, Mixer, Channel Settings, transport, editors, MediaBay, Preferences, setup dialogs, plug-in windows, project/file dialogs and workspace/key-command windows.
4. **Dropdowns and selection controls.** Enumerate combo boxes, popup menus, selectors, lists and generated option families. Trace selected values through their state to a consuming operation, persistence and engine/device path where applicable. Record runtime-dependent catalogs as dynamic families, including how their entries are sourced, rather than pretending installed devices and plug-ins are fixed source entries.
5. **Reconcile and review.** Cross-check the four registers, previous findings and current task notes. Investigate discrepancies and stale documentation; distinguish a disabled menu entry from a separate working route to similar behavior. Inspect evidence behind uncertain conclusions. Summarize gaps by workflow, without expanding scope into feature design.

Each pass is completed and reviewed before moving to the next. Parallel read-only inspection within a pass may use Luna Low agents when available, with one owner consolidating and resolving overlaps. No concurrent source edits are needed.

## Record for each item

Keep a durable table or CSV alongside the summary inventory with: stable audit ID; area and full user-facing path; item/type; source file and line; creation/visibility condition; enabled condition; command ID or direct handler; state and downstream consumer; persistence/undo when relevant; status; evidence and any uncertainty. Link dynamic entries to their generator rather than inventing individual names.

Use these source statuses:

- **Implemented in source:** a handler and relevant consumer are traced; runtime behavior is unverified.
- **Partial:** some behavior is connected but a named part is missing or uncertain.
- **Unavailable:** explicitly disabled, placeholder, no handler or a confirmed missing consumer; specify which.
- **Conditional:** disabled only in a specified state, such as no project or active scan; not itself a stub.
- **Deferred/excluded:** a documented product decision excludes or defers it; cite that decision and do not assume an old reference menu is a requirement.
- **Unknown:** source path or behavior cannot yet be established; record why and the next inspection needed.

Source status is not runtime, audio, device or user-acceptance evidence. Avoid calling something broken merely because a particular menu route is disabled. Include potentially cosmetic controls whose state has no consumer, and separate data/UI persistence from audio processing.

## Coverage ledger and completion

Maintain a ledger listing inspected source files and entry points, covered menu/window/control families, generated/conditional cases, counts by status, and unreviewed or unknown items. Use the actual source snapshot to reconcile totals; do not claim completeness from a text search alone. The reviewer checks every item against source and resolves duplicates, ambiguous names, disabled state gates and incorrect source references. Retain unresolved items as explicit unknowns with a reason.

Step 1 is complete when all four source registers and the coverage ledger are reconciled, every identified entry is classified or explicitly unknown, and a concise report links the evidence and remaining uncertainties. Report the source identity and any files that changed during the audit. The running-app checks remain step 2 and are not represented as complete.

## Deliverables

- Detailed item registers and coverage ledger under `docs/reviews/`, linked from the existing summary inventory.
- A short report of counts, major gaps, stale documentation, and unknowns, with citations to the source and registers.
- A handoff recording the actual models used and baseline identity. No application implementation or refactor is part of this packet.
