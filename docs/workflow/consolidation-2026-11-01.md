# Roadmap step 32: consolidation and re-plan

Date: 2026-11-01. Author: DeepSeek V4.1 Flash.

This reconciles the UI inventory, the roadmap steps, the task packets and the
architecture agreement against what is actually on `main`, and records what the
boundaries built in this revision did to coupling. Numbers come from the source
at `3fbc71e`; commands used are listed at the end.

## Step status

`Source` means the code exists and automated checks cover it. `Runtime` means it
was exercised in the running application. `User` means the user confirmed it.
Nothing here upgrades source evidence to runtime or user acceptance.

| Step | Status | Evidence | Open |
| --- | --- | --- | --- |
| 1 CPU/memory meters | User | T071 accepted | none |
| 2 Baseline audit | Source | recorded HEAD | none |
| 3 UI inventory | Source | `docs/reviews/window-selector-coverage.md` | re-measured below |
| 4 MIDI editor snap | Source + tests | T072 | interaction review |
| 5–9 Transport, ruler, grid, mixer slices | Source + tests | T001–T020 packets | runtime review |
| 10 Mixdown export | Source + tests | T074 | runtime review, parity |
| 11 MIDI file exchange | Source + tests | T075 | DAW fixtures, runtime |
| 12–13 Sends model and wiring | Source + tests | T076 | live/hardware review |
| 14 Verify sends | Open | — | needs listening pass |
| 15 Routing ownership | Source | `app/src/routing.rs` | now typed (step 22) |
| 16 Typed buses | Partial | `core::routing` covers input/output/send vocabulary; the buses themselves are still stereo | multichannel buses |
| 17 Automation contract | Source | T078 | bus targets |
| 18 Automation playback | Source + tests | T078 | live/hardware review |
| 19 Automation edit + write | Source + tests | T078 | desktop review |
| 20 Verify automation | Open | — | desktop/audio review |
| 21 Mixer review | Partial | — | runtime listening |
| 22 Device routing breadth | Source + tests | `core::routing`, `validate_routing` | non-stereo buses |
| 23 Recording reliability | Source + tests | journal + `repair_take` | kill-during-record review |
| 24 Plug-in hardening | Partial | T024 notes | failure isolation, editors |
| 25 Edit family | Source + tests | split/move/crossfade/duplicate/mute/select | repeat, crop, event grouping |
| 26 Add Track choices | Source + tests | instrument catalog | preset browsing, runtime |
| 27 Command unification | Source + tests | 8 commands across menu + keymap | Repeat/Crop/Invert/Lock |
| 28 Selection identity | Source + tests | `selection_identity_tests` | other surfaces |
| 29 Recording resilience | Source + tests | take recovery + autosave | hardware review |
| 30 One editing policy | Open | — | pick a user-valued setting |
| 31 CPR interchange | **Blocked** | `core::cpr` header inspection only | needs real Cubase fixtures |
| 32 Consolidation | This document | — | the re-plan below |

Steps 25, 27 and 28 were stale in the roadmap text: they still described split,
duplicate, mute, range commands and selection identity as open. They are now
written to match the delivered behavior.

## UI inventory

Measured in `app.rs`:

- 68 distinct menu commands are wired through `menu_command`.
- 165 distinct entries are still shown disabled, across 8 `disabled_menu_commands`
  lists (176 when this report was written; Lock/Unlock, Group/Ungroup, Repeat,
  Fill Loop, Crop, Invert, Events under Cursor and Move to Locator were implemented
  afterwards). They are honest placeholders: each is visible but inert.
- 52 command ids have a dispatch arm in `dispatch_reference_command`; the
  reference keymap has 152 ids, so 100 shortcut ids still report
  "Not implemented yet" when pressed.

The gap is therefore not "unknown UI" but "declared, inert UI". The re-plan below
proposes shrinking that list only where a step already exists to carry the work.

## Coupling measurement

This revision (7 commits, `a9a1480..3fbc71e`) added 3,750 lines and removed 240
across 16 files.

| File | Total lines | Note |
| --- | --- | --- |
| `app/src/app.rs` | 22,111 | 3,093 (13%) are module-level `#[cfg(test)]` blocks; ~19,000 are code |
| `app/src/channel_settings.rs` | 4,337 | |
| `app/src/audio_events.rs` | 3,491 | edit gestures + commands |
| `core/src/project.rs` | 4,020 | includes routing validation and clip commands |
| `app/src/routing.rs` | 216 | was 92 |
| `core/src/routing.rs` | 372 | new |
| `app/src/recording_recovery.rs` | 371 | new |
| `app/src/project_recovery.rs` | 152 | new |
| `audio/src/recording_recovery.rs` | 299 | new |

Boundaries this revision established, and what they proved:

1. **Routing became a value, not a string.** `core::routing` owns parsing,
   labelling, destination lists and validation. Before, three call sites parsed
   `"Group:7"` independently and an unknown value became the stereo master; now
   one parser feeds the send rack, the cycle check, the monitor tap and the
   playback plan, and an unresolvable route is `Disconnected`. This is the
   clearest win: the app kept its string persistence and lost three duplicated
   parsers.
2. **Recovery is a persistence concern, not a UI concern.**
   `audio::recording_recovery` repairs files; `app::recording_recovery` owns the
   journal; `app::project_recovery` owns the autosave. `app.rs` only decides when
   to show the prompt and applies the result through the existing history path.
   All three are testable without an egui frame except the prompt itself.
3. **Automation stayed in its module.** The write capture and the range-replace
   command landed in `app/src/automation.rs` and `core/src/automation.rs`; the
   frame code gained one call, not a second copy of the curve logic.
4. **Edit commands stayed in `audio_events.rs`.** Eight commands share one
   shape (`finish_frame` → `begin` → mutate a candidate → `commit_after_frame`)
   and the menu arms call them; no new state was added to `OpenTrackApp` for
   editing.

What did not improve: `app.rs` still grew by 1,678 lines, most of it frame
orchestration, dialogs (`show_recording_recovery`, `show_project_recovery`) and
test modules. Moving a dialog out is a real extraction but it changes nothing
user-visible, so it stays queued rather than done speculatively.

## Test position

| Suite | Result |
| --- | --- |
| `opentrack` (app) | 289 passed, 0 failed, 9 ignored |
| `opentrack-core` | 61 passed |
| `opentrack-audio` | 145 passed, 3 ignored |

The app suite started this revision at 244 passed / 15 failed. The 15 failures
were tests that encoded the Windows pixel scale; the fixture now measures the
scale from the ruler the code draws, and the last six came from one real bug:
the pitch note's anchor handle covered the vertical centre of a 12 px note, so
body clicks were anchor moves.

## What the evidence says to do next

Ordered by user value divided by open risk:

1. **Runtime acceptance passes** for T074 export, T075 MIDI, T076 sends, T078
   automation and both recovery prompts. Five features are source-complete with
   no listening pass; that is the largest untested surface, and only a running
   app closes it. This needs the user at the machine with audio.
2. **Repeat / Crop / Invert** finish the edit family that steps 25 and 27 name.
   They are small, share the existing command shape, and remove three of the 176
   inert menu entries.
3. **Linux VST3 editor embedding** (`kPlatformTypeX11EmbedWindowID`). Large and
   plug-in dependent, but it is the one platform feature the user runs daily.
4. **Non-stereo buses** (step 16/22 remainder). The routing model already
   expresses wider channels and validation rejects them; the work is the capture
   and output buses, which is an audio-engine change with a listening gate.
5. **Step 30, one editing policy**: pick a Preferences setting whose consumer is
   already specified — the Sample Editor snap settings and the autosave interval
   are the two candidates with real behavior behind them.
6. **CPR adapter (step 31)** stays blocked until real Cubase fixtures exist.
   Nothing in this revision changes that.

A next roadmap revision should therefore lead with verification, not new
surface: the source is ahead of the evidence.

## Commands used

```
wc -l crates/*/src/*.rs
grep -c "menu_command(ui," crates/opentrack-app/src/app.rs
grep -oE 'menu_command\(ui, "[^"]+"' ... | sort -u | wc -l
grep -A14 "disabled_menu_commands(" ... | grep -oE '\("[^"]+", "[^"]*"\)' | sort -u | wc -l
git diff --stat a9a1480..HEAD
cargo test -p opentrack / -p opentrack-core / -p opentrack-audio
```
