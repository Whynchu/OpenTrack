# R5-32 — Revision 5 consolidation

- Status: accepted
- Scope version and requirement IDs: revision 5, step 32
- Desired observable outcome: one current record of what revision 5 delivered, what remains user-blocked, and the measured state of the build, with README, current.md, the native-format document, the generated inventory and the delivery handoff all agreeing.
- Reference evidence (IDs/paths): `docs/workflow/handoffs/revision-5-delivery.md` (created); `docs/workflow/roadmap-32-steps-v5.md` (close-out section); `docs/workflow/current.md` (rewritten); `README.md`; `docs/native-project-format.md`; `docs/reviews/generated-source-inventory.md`; `docs/workflow/accessibility-2026-11-02.md`.
- Dependencies and decisions needed: R5-01 and R5-03 to R5-31 (accepted); R5-02 stays open with the user.
- Code scope and architecture boundaries: documentation and inventory only; no product change.
- Acceptance checks: the inventory regenerates with no diff; README states OPR 38, 69 disabled rows and 149/152 dispatch; the native-format document lists v38 and migrates v1-v37; current.md opens with the revision-5 handoff; every step 1-32 has a packet; `git diff --check` is clean.
- Affected regressions: none.
- Essential gates: none beyond an accurate record.

## Measurements recorded

- Inventory: 275 project-open leaves — 41 available, 157 state-dependent, 69 unavailable, 8 informational; 18 shell leaves; no duplicate full paths.
- Shortcut dispatch: 149 of 152 command IDs; the three unbound are Video (`F8`), Open Score Editor (`Ctrl+R`) and Use External Sync (`T`), each naming a feature that does not exist.
- Source disabled labels: 46 unique `(label, shortcut)` pairs across `disabled_menu_commands` blocks (58 at the start of the revision).
- OPR: 38; migrations tested through v29-v38 including v37-v38 offline chains.
- Suites: app 579 passed / 4 failed / 9 ignored (592 tests); core 166; audio 171 / 3 ignored. The four failures are the `transport_locate_tests` roll cases, which fail identically on the pristine revision-4 checkpoint because `schedule_roll` needs an audio output device; no assertion was weakened.
- Release build: `builds/x64/OpenTrack`, SHA-256 `ae12f96f416ff662057745f346ebd44e4d08431c7c7bee3436c42cd926f17f33`.
- `cargo fmt --all --check` and `git diff --check` clean.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: `afcfa65` plus this consolidation commit.
- Changes and reasons: new delivery handoff; roadmap close-out; current.md rewritten to the revision-5 state; README refreshed (Tempo Track editor, Retrospective Record, record folder, Audio CD import, background archive import, Direct Offline Processing, locator-only mixdown, OPR 38, 69 disabled rows, 149/152, interchange decision); native-format document brought to v38 with the field history for v33-v38; R5-02 and this packet written so all 32 steps have a record.
- Commands/checks and actual results: `UPDATE_SOURCE_INVENTORY=1 cargo test -p opentrack generated_source_inventory_matches_source` → passed, no diff after regeneration; the suites, build, fmt and diff checks reported above.
- Runtime environment: Kubuntu Linux harness; no desktop, listening or hardware claim.
- Ledger IDs updated: README, current.md, native-format document, inventory, delivery handoff.
- Review findings: the native-format document still said version 33 and had no entries for v34-v38; the README still described OPR 37, the pre-revision-5 feature set and the old interchange position; the roadmap close-out initially said 32 steps "were executed", which overstated step 2 and was corrected to 31 implemented plus the user's open review.
- Failures/unknowns/pending acceptance: **R5-02** (user runtime review), audible checks (FX returns, arranger preview, virtual keyboard, retrospective capture with a controller, baked chain versus direct commands), Windows x64 local-ASIO, VST3 third-party verification, Linux VST3 editor embedding, and the four device-dependent roll tests.
- Next ready task: none. Do not append post-close slices; the next queue needs the user's runtime review and the two open export decisions (stems and group/FX channels; an arranger export entry).
