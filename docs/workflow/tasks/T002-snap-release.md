# T002 — Snap the transport cursor on ruler release

Status: review_pending after correction. The user rejected the previous binary's snap behavior. Its partial self-reported checks do not establish verification. Scope: transport-v1, seek-pointer and snap-release. Actual implementer: GPT-6.1 Sol.

Later update: T008 activates Grid Type/Quantize Type and replaces the fixed Bar interval with the selected shared snap interval. Bar remains the legacy default; Use Quantize explicitly follows the selected note division. This task's earlier binary identities and fixed-grid findings are historical.

Outcome: normal left drag seeks continuously under the pointer; release snaps according to the confirmed active grid/snap settings. Snap behavior is independent of ruler label stride. Preserve viewport mapping. Simultaneous horizontal seek and vertical zoom remains a separately recorded source gap: current gesture code locks to one axis.

T001 must fill in: exact normal-ruler hit target; snap enabled/disabled rules; rounding/tie and project-start boundary; source seams; fixture setup and expected positions. These are readiness dependencies, not permission questions for the user when code/reference evidence can answer them.

Implementation scope: touched timeline coordinate/gesture code and shared seek command; necessary pure helper/tests. Preserve Mixer, range-drawing zone, middle pan, scrollbar, pinned header and right-click menu. Do not add a ruler tooltip.

Acceptance cases: grid on/off; exact grid point; off-grid release; release after horizontal and diagonal drag; release after edge scroll; project-start clamp; zoom levels whose labels skip bars; release outside original hit rectangle with captured drag. Verify expected musical position and marker, using the finalized T001 semantics. Check that view-only/seek activity does not create an unintended save prompt. Test pure boundary calculations and live pointer interaction; tie results to build/source identity.

Run the existing build before edits and x64 build afterward with OpenTrack closed. One app instance during review; close when finished. Use a disposable EXAMPLE copy if project content can change. Update ledger and report. Stop for independent interaction review; user milestone acceptance remains pending until the transport slice is demonstrated.

## User failure and correction — 2026-10-24

The user checked binary `025AA199F7FC0852B37DE5581915D79C12949BBFC7D317F7FC58C0CA0D1B36DA` and reported that the cursor did not snap. The earlier audit incorrectly treated the separate `1/16` MIDI Quantize Type as the ruler grid. The toolbar's actual Grid Type is `Bar`. The corrected code snaps click/release to `beats_per_bar` quarter-note beats. At typical zoom, the old 1/16 movement was about a pixel and did not match the displayed grid.

The correction ships with [T003](T003-directional-ruler-ranges.md), binary SHA-256 `39846420C9B1FABEC70651448FE78DBC06BEF63478F990B460747C1F6915B4B4`. A headless egui test drives the actual ruler press/move/release path and checks continuous movement before release, bar snapping afterward, and unsnapped release with Snap disabled. This is automated interaction evidence, not a live desktop acceptance. The user requested that the application remain closed for implementation and build. Live snapping, marker alignment and unchanged-close acceptance remain pending the user's check.

## Historical partial runtime checks — superseded

The x64 binary under test is `builds/x64/OpenTrack.exe`, SHA-256 `025AA199F7FC0852B37DE5581915D79C12949BBFC7D317F7FC58C0CA0D1B36DA`. Source identities: `app.rs` `CEC003FA65D026B7E751EF825381F17B56DF9883A697F355A4E591FE5C30467F`; `project_file.rs` `72A859C50ECE8BE7D5920F9B06894153405FFCB8EB8DE2E4379AA1F86CABFD16`; `timeline.rs` `8F2251E1A6EE66812DCF13AB0F4EAB385E293E717818C5013A50EF44B910F4FE`; `main.rs` `9308BB47C2FF30797669DDAA53B241E215B264CB3A37987A519CD2EDCFDB9E21`.

One OpenTrack process loaded `C:\Users\Samue\Documents\EXAMPLE.opr`; the title displayed `Project - EXAMPLE`. The maximized dark Project window showed Inspector left, Mixer bottom, and floating Transport, at 1920×1033 desktop resolution; display scaling is unknown. The example was not saved. These checks occurred on the identified binary:

- Snap on: a diagonal ruler drag from `(650,162)` to `(820,184)`, ending below the original ruler hit rectangle, moved the marker to the endpoint and displayed `35.1.1.0` / `0:01:08.000` (136 quarter-note beats). This confirms captured drag and snapped release for that case.
- Snap off: clicking `(530,162)` displayed `17.2.1.48` / `0:00:32.550` (65.1 quarter-note beats).
- Snap on: clicking the same point displayed `17.2.1.0` / `0:00:32.500` (65 quarter-note beats).

These historical checks were superseded by the user's failed check above. They do not establish correct snapping. At that time the computer-use bridge stopped exposing native-app methods. The user has since closed the application. No current desktop screenshot or final close/no-save-prompt result exists.

Still required: unsnapped horizontal drag release; both directions and held edge-scroll release; continuous pointer alignment; project-start clamp; zoom/label-density cases; no-change close after restoring the original Snap setting; current-build screenshot and coverage row; then close the single OpenTrack process. `EXAMPLE.opr` must remain unsaved. Do not begin the next substantial packet until these checks and the user review checkpoint are complete.

## Source and build verification

- `cargo test -p opentrack dirty_check_ignores_arrange_navigation_but_preserves_saved_preferences`: passed (1 test).
- `cargo test -p opentrack timeline::tests`: passed (5 tests).
- `cargo fmt --all -- --check`: passed.
- The x64 build completed after the old OpenTrack instance was closed. It was then opened once for the checks above.
- Ledger/runtime readiness remains pending; compiling and these partial interactions do not establish full transport parity.
