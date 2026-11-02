# R5-03 — Export and delivery contract review

- Status: accepted (review only; two behaviour notes for the user, no code change)
- Scope version and requirement IDs: revision 5, step 3
- Desired observable outcome: every delivery path derives its range and length from a defined, visible source, refuses clearly when that source is unusable, and never exports a different range than the user sees.
- Reference evidence (IDs/paths; observed vs user-specified vs unknown): `crates/opentrack-app/src/audio_export.rs` (`locator_range`, `Dialog`, `bounce_selection`, `start_audio_export`, `render_stems`), `crates/opentrack-app/src/app.rs` (`start_arranger_preview`), revision-5 packet step 3, user direction 2026-11-02 that export must use the locators.
- Dependencies and decisions needed: none for the audit. Two observations below need a product decision before any change.
- Code scope and architecture boundaries: review; the only code change already made in this revision is the locator-only mixdown range (`9a5e7dd`).
- Acceptance checks (action, expected result, fixtures, evidence to retain): read each export path and record its range source; confirm refusal paths; confirm no entry claims an unimplemented export.
- Affected regressions: none when no code changes.
- Essential gates (audible result, persistence, undo/dirty state, recovery as applicable): cancelled exports leave no published file; export never overwrites a source.
- Review owner and milestone stop: self-review; findings reported to the user.

## Findings

| Path | Range source | Refusal when unusable | Published output |
| --- | --- | --- | --- |
| File > Export > Audio Mixdown | Left/right locators only (`locator_range`) | Equal, inverted or negative locators: "Set the left and right locators before exporting" | Single WAV at the chosen format/rate |
| File > Export > Selected Tracks (Stems) | Same locators | Same | One WAV per audible track |
| Edit > Bounce Selection in Place | Union of the selected events on one track | Mixed tracks, no events, no usable range, unmanaged folder, playback graph not ready | One WAV plus a placed event, one history step |
| Arranger | Sequence length from the section order | Cycle/recording refuse preview; there is no arranger **export** entry | None: preview only |
| Track Archive export | Whole selected tracks, not a beat range | Unimplemented dependency report, incomplete package refusal | Directory package |

- Mixdown and stems share one range function, so they cannot disagree; the dialog shows the locator values and no editable range fields exist.
- Bounce-in-place intentionally uses the selection rather than the locators: it replaces the selected material in place, so a locator range would place the result somewhere the user did not ask for. The two rules are distinct by design and the menu wording matches.
- No File > Export row claims an arranger or sequence export, so nothing silently renders the wrong range.
- Cancelled mixdown/stem/bounce tasks return "export cancelled" and the staged destination is not published; source files are only read.

### Observations needing a decision (no change made)

1. **Stems include group and FX channels.** `render_stems` exports every track with content, each through its own channel. When a group or FX return is present, the sum of the stems therefore exceeds the mixdown. That matches the dialog's wording ("one WAV per track") but differs from a "sum of stems equals the mix" expectation. Options: keep as is and document, exclude group/FX channels from stems, or add a channel-selection step. Not changed here because it changes delivered behaviour.
2. **Arranger has no export.** Step 26 of revision 4 delivered an offline sequence transformer, but no menu entry renders a sequence to a file; only preview uses it. Either add an explicit "Export Arrangement..." path in a later step or leave it preview-only and keep the absence visible.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity (commit plus dirty diff/artifact identity) and build hash: `9a5e7dd` plus this documentation commit.
- Changes and reasons: no code change; the locator-only mixdown range landed in the review-fix commit. This step records the contract and two decision items.
- Commands/checks and actual results: `cargo test -p opentrack dialog_range` (locator range, three refusal cases, stems dialog) passes; `cargo test -p opentrack` 549 passed / 4 failed (the documented device-dependent roll tests) / 9 ignored.
- Runtime environment and evidence paths: source review on Kubuntu Linux; no desktop review of the dialogs in this step.
- Ledger IDs updated; report command/result: none.
- Review findings, rework, elapsed effort if available: the two observations above.
- Failures/unknowns/pending acceptance: whether stems should exclude group/FX channels, and whether the arranger needs an export entry, both need the user's decision.
- Next ready task: R5-04 (tempo/signature contract) once the user has seen R5-02 and these two observations.
