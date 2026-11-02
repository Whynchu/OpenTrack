# R5-23 — Regions editor polish

- Status: accepted
- Scope version and requirement IDs: revision 5, step 23
- Desired observable outcome: the Regions editor supports multi-select, keyboard navigation and confirmed deletion while keeping region selection distinct from audio-event selection.
- Reference evidence (IDs/paths): `RegionEditor` and `show_region_editor` in `crates/opentrack-app/src/app/region_editor.rs`.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries: the editor module only; selection is a set of draft indices so new (id 0) drafts can be selected.
- Acceptance checks: plain click selects one, Ctrl/Shift extends; Select All and Escape clear; Arrow keys move a cursor and can extend; Delete opens a confirmation and only confirming marks drafts; Apply removes exactly the marked regions as one "Edit Regions" step; audio-event selection is unchanged; the window renders the confirmation text.
- Affected regressions: none.
- Essential gates: one history step, no event-selection change, cancel changes nothing.
- Review owner and milestone stop: self-review; desktop review of the keyboard path pending.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: `selected` became `BTreeSet<usize>` with a `cursor`, plus `toggle`, `select_all`, `clear_selection`, `move_cursor` and `mark_selected_removed`; the window handles Arrow Up/Down, Ctrl+A, Escape, Space, Delete, adds Select All / Delete Selected buttons and an inline "Delete N selected region(s)?" confirmation with Delete/Keep.
- Commands/checks and actual results: `cargo test -p opentrack region_editor` → 3 passed (2 new); app suite 573 passing with the four known device failures.
- Runtime environment: Kubuntu Linux; keyboard path exercised through the editor methods and verified for rendering.
- Ledger IDs updated: inventory regenerated.
- Review findings: the first test assumed the cursor started at zero after a selection burst; the test now sets it explicitly, which is also how a user reaches a known row.
- Failures/unknowns/pending acceptance: desktop review of the keyboard focus (the window must have focus for the key handling) and of Ctrl/Shift-click with a real pointer.
- Next ready task: R5-24.
