# Upcoming work plan — 2026-10-25

This is the working plan after the T041/T042 handoff and the user’s latest
requests. It is a planning record, not acceptance of any batch or audio result.
`current.md`, the T041/T042 packets, and `ledger.json` remain the maintained
status sources.

## Status snapshot

| Track | Status |
| --- | --- |
| Batch 1 / T041 | User accepted |
| Batch 2 / T042 | Active, not accepted; latest build `F21F417145B6590D19270EA5BAD3F242D78B9727ADF1FF038B6C31CD395B98C4`; playback crackle still reported |
| Batch 3 | Pending; gated on Batch 2 acceptance |
| Sample Editor note color | New visual requirement; not yet in ledger |
| VST3 hosting | Plan and external probe complete; product integration authorized to start during this work |
| Codex goal tracker | Paused for review; not complete and not transferable |

OpenTrack was confirmed closed before this plan. Do not build while it is running
and do not replace `builds/x64/OpenTrack.exe` in place.

Build with local ASIO:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64 -LocalAsio
```

Preserve `C:\Users\Samue\Documents\EXAMPLE\EXAMPLE.opr` and its media. Use
disposable copies for destructive verification.

## Workstream A — Finish T042 Batch 2

The user clarified that the crackle occurs **on playback**, not while dragging.
There is currently no audible playback while dragging a note, although there
should be. Treat these as two separate problems:

1. Persistent playback synthesis quality.
2. Live audition/publication during a note gesture.

Do not start Batch 3 until the user accepts the Batch 2 listening review.

### A1. Classify the playback artifact

User listening protocol:

- whole-step shift on an isolated sustained vowel;
- release, stop, replay the same pass;
- shortened note;
- connected notes;
- original/corrected comparison.

Record whether the artifact is present after release/replay, at onset, sustain,
tail, or only during the edit. No assistant-generated WAV review is required;
this is the user’s own listening gate.

### A2. Add live audition during note drags

Audit result: the draft **is** published during the gesture and the app **does**
rebuild the playback plan on changed frames; the prepared pitch source is fresh.
The missing behavior is audition. When the transport is stopped, the callback
deliberately renders no event audio and there is no note preview voice. With the
transport running but the playhead outside the event, the edit is also inaudible.

Implemented: a dedicated note-audition voice is prepared on the control
thread from the dragged note range, looped for the gesture and stopped on
release. It lives beside the playback plan, fades the previous preview out over
5 ms when the edit changes, and does not move the project playhead, rebuild the
whole project plan per frame, or allocate in the callback. The editor publishes
the held note, and Sample Editor tests assert that a stopped transport still
requests audition and release clears it.

The transport-start fallback was not adopted because it would move shared
transport/playhead state.

Acceptance: with the transport stopped, body/edge dragging auditions the note
continuously with the same pitch/timing contour as the trace; releasing stops
audition and commits the final state without a jump. With the transport playing,
the drag remains continuously audible and does not create clicks from redundant
plan swaps.

### A3. Pitch-mark and grain-phase continuity

Files: `pitch_synthesis.rs:350-465` (`make_grains`), `:266-277` (mix).

Current mechanism selects the nearest positive source mark for each output
grain. A mark-index switch can jump grain source phase by about one period.
The 2026-10-25 synthesis audit did not reproduce a +2-specific defect on a
stationary harmonic source, so this is no longer the leading explanation for the
reported crackle; the two-voice transition and edge-only endpoint defects were
more concrete. Keep this as a real-vocal follow-up, not an assumed cause.

Investigate with a synthetic sustained vowel and a real-vocal fragment:

- mark spacing and mark-index changes over time;
- grain phase at destination boundaries;
- grain source jumps when the nearest mark changes;
- phase-aligned/continuous mark mapping versus nearest-mark switching;
- a bounded correction only after evidence.

Do not tune seam-fade lengths as a substitute for fixing phase continuity.

### A3b. Residual pitch-correction garble

The user still hears intermittent garble in some corrections. Codex is actively
working on the resynthesis path, so do not make speculative DSP edits here.
Long-term acceptance needs the exact source clip, note edit, sample rate,
playhead position and a before/after render for each case. Keep this separate
from the VST/insert work and from the T042 listening review.

### A4. Dry/processed mixing, voicing, and transients

Files: `pitch_synthesis.rs:266-277`, `estimate_pitch`, `voicing`.

Check:

- confidence fade at voiced/unvoiced boundaries;
- dry original plus shifted wet beating in sustained voiced regions;
- the `voice.sqrt()` bias during timing edits;
- consonant/transient frames returning to dry audio;
- whether a whole-step shift is mixing two incompatible pitches.

### A5. Time compression and short notes

Files: `make_grains`, adaptive `radius`, `source_per_destination`,
`note_source_time`.

Check:

- grain holes or excessive overlap when shortening;
- radius growth causing chorus/combing;
- short-note plans with too few marks;
- source mapping at destination boundaries;
- a shortening test on real vocal material.

### A6. Connected-note transition validation

Files: `pitch_time.rs:63-110`, `:171-186`, `pitch_synthesis.rs` transitions,
`sample_editor.rs:1021-1023`.

The 2026-10-25 correction is implemented and needs listening validation:

- the audio handoff is now a 5 ms crossfade at the boundary instead of blending
  two independently synthesized voices across the whole pitch ramp;
- a one-edited/one-unchanged pair resolves to the unchanged note's edge pitch,
  while two edited notes still meet at their midpoint;
- unedited neighbor edges fade wet-to-dry at the processed-region endpoints;
- sample-continuity tests cover both transition orientations and prove a single
  voice away from the boundary;
- actual source-tail anchoring, one-sixteenth-note maximum at project tempo,
  shorter spans on short notes, genuine gaps, and shared trace/audio contour
  still require the user's listening review.
- crossfade is bounded and does not double-process;
- no vertical trace jump at the boundary.

### A7. One bounded correction, build, and stop

- Choose one correction based on the evidence above.
- Add or update meaningful tests.
- Format, build x64 local ASIO, record executable/source hashes.
- Update T042, `current.md`, ledger evidence; mark stale runtime rows.
- Run `python scripts/parity_report.py`.
- Stop for the user’s listening review. Do not claim the crackle is fixed.

## Workstream B — Sample Editor note color

Current code: `pitch_editing.rs:446-456` selects the note palette from
`note.detected_midi.round()`. A vertical move changes `shift`, so the note keeps
its original source-pitch color.

User-approved rules:

- **Unselected note:** use the four-color note palette at the note’s nearest
  current semitone row:
  `((note.detected_midi + note.shift).round() as usize) % colors.len()`.
- **Selected note:** use the track color highlight, not orange. Use
  `project_controls::track_color(track)` and pass it into the pitch editor draw
  path. Preserve selection legibility with the existing fill/stroke emphasis.
- Fine pitch uses the nearest semitone; `.49` and `.51` round as expected.
- Time-only edge stretching does not change color.
- Multi-selected notes each use their own current center.
- Reset/Undo restores the original color.
- Split/join preserves the applicable center.
- The editor trace and note color remain consistent.

Tasks:

1. Add a pure helper for the note color index from the current center.
2. Pass the selected track color into `PitchEditor::draw`.
3. Use the target color for unselected fill/stroke and the track color for
   selected fill/stroke; do not use orange.
4. Add pointer tests for +2/−2 moves, fine pitch rounding, edge stretch,
   multi-select, Reset and Undo.
5. Add ledger requirement `pitch-note-color-follows-center` and bump the scope
   version explicitly.
6. Build and include in the next visual review. This is visual-only and does not
   replace the T042 listening gate.

## Workstream B2 — Track-color cohesion

User request: track meters, EQ parameters, the EQ graph, and the channel
spectrum should inherit the track color. Stereo In and Stereo Out should default
to blue and also be color-selectable. Normal track colors remain user-selected.

Scope:

- **Meters:** Project track meters, Mixer meters, and Channel Settings meters use
  the channel's track color. Preserve red above unity/clipping and any record
  semantic color. Do not recolor the meter plate/rail backgrounds.
- **EQ:** Parameter-bar fill/thumb, response curve, band nodes, hover accents,
  and the channel spectrum's own trace use the track color. The comparison trace
  stays orange. Disabled/bypassed elements stay muted.
- **Stereo In/Out:** default blue when no color is set; expose the same color
  picker used by normal tracks in the Project/Mixer bus strip. Persist the choice
  in the project. Do not route color selection through audio-plan rebuilds.
- **Normal tracks:** keep the existing palette and selected-track behavior.
- **Tests/visual review:** track color reaches every meter/EQ/spectrum call site;
  red clipping remains; bus defaults are blue; changing bus color persists and
  does not dirty audio state.

Status 2026-10-25: implemented. Project, Mixer and Channel Settings meters use
the channel color while retaining the red over-unity indicator; EQ parameter
bars, graph curve, band nodes and the owned spectrum trace use it; the comparison
trace stays orange; Stereo In/Out default to blue and expose the palette select-
or in the Mixer footer. Tests cover bus defaults/explicit colors and meter/EQ
color use. Live visual review is still required.

Collision note: `mixer.rs` and `app.rs` are shared with Batch 3's fader/selection
work. Do track-color cohesion first, or serialize it with Batch 3. `channel_settings.rs`
and `analyzer_view.rs` are otherwise independent of the pitch/note-color lane.

## Workstream C — Batch 3

The user authorized Batch 3 on 2026-10-25. Source implementation is complete;
the x64 build and live interaction/visual review are pending.

### C1. Contextual Project event scissors

Hover above the black event-name strip exposes the cut zone; cut at the actual
snapped guide; preserve event identity, selection and Undo.

### C2. Snap-aware cursor artwork

Scissors alone when Snap is off; pencil alone for loop/skip range drawing;
snapped cut guide follows the committed coordinate; no ambiguous mixed cursor.

### C3. Shared Project/Sample Editor wheel map

Plain wheel: time zoom. Ctrl+wheel: horizontal scroll. Shift+wheel: vertical
scroll. Ctrl+Shift+wheel: vertical zoom. Shared behavior and no text-panel wheel
leakage.

### C4. Rectangular Mixer fader caps

Replace the cap shape; preserve accepted Mixer layout, colors, hover, drag and
fine behavior. Visual review required.

### C5. Shared Project/Mixer track selection

Clicking a Mixer channel selects the same track in Project and Inspector;
selection is shared, not duplicated; no audio mutation from selection.

### C6. Batch 3 build and review

Format, build local ASIO, update ledger/current, run parity report, stop for the
user’s interaction/visual review.

Status: all five source requirements are implemented. App suite is 135 passing.
Build/review remains.

## Workstream D — VST3 audio integration

The user authorized starting VST work during the current effort. Keep it in a
separate ledger scope and separate files/worktrees so it does not collide with
T042 or Batch 3.

### Status

- Architecture/delivery plan: `docs/plugin-hosting-plan.md`
- External probe: `G:\dev\Opentrack\_analysis-tools\vst3-probe`
- Probe README: all commands/results.
- No Steinberg-authored files; own Rust host using the `vst3` crate bindings.
- No `vst-audio`; Apache-2.0/no CLA direction.
- VST foundation committed as `81227f5`: `opentrack-plugins` catalog/class-ID
  types and `opentrack-plugin-scan` folder/PE/dedupe/report foundation.
- `opentrack-vst3` now loads x64 modules and enumerates factory classes,
  including the `InitDll`/`ExitDll` handshake needed by WaveShell. The scanner
  has `--enumerate`; the installed folder produced 11 enumerated modules and 28
  unique classes (19 from WaveShell). Host context skeleton implements
  `IHostApplication`, `IMessage`, and `IAttributeList`.
- Not implemented yet: component/controller creation, bus/parameter/latency
  introspection, audio processing, editors, state save/restore, block graph,
  PDC, and per-module timeout/crash attribution. The scanner process is a crash
  boundary but not a security sandbox.
- Next VST code: instance smoke-test/bus introspection, per-module job-object
  isolation and timeout, cache persistence, then block graph/PDC before hosting.
- T043 now implements the actual hosting path: VST3s occupy the existing eight
  insert slots, use a `HostedProcessor` block contract, a 64-frame hosted graph,
  per-track PDC, opaque component/controller state, generic parameters and a
  native editor window. See `tasks/T043-vst3-insert-hosting.md`. The existing
  insert UI and signal chain remain the single path; built-ins are just other
  insert choices.
- TDR Nova smoke verification passed: 75 parameters, 184-sample latency, audio
  block processed, 3,608-byte component state, editor available. The app still
  needs its final build and live review; VST instruments/multi-out/sandboxing
  remain follow-up.
- UI integration implemented: scanned external effects appear in the normal
  insert category menus, the separate VST3 picker dropdown is gone, and
  Devices > Plug-in Information manages folders/catalog/scan failures. Folders
  and catalog persist. The folder scanner still loads modules in the app process
  until the isolated scanner manager is wired.

### Probe findings to carry forward

- `IMessage`/`IAttributeList` are mandatory; Ozone crashed on connect without
  them.
- Parameter edits need `IParameterChanges`/`IParamValueQueue`; controller
  `setParamNormalized` alone does not persist correctly.
- Single-component controllers are identified through `getControllerClassId`,
  not pointer equality.
- Activate only the first main bus; SD3 reports 16 “main” outputs.
- Latency changes with sample rate/block size; PDC must update.
- Process enough blocks to cover latency; Ozone at 64 frames needed 23 blocks.
- Tail is advisory; H-Delay reports 0 tail but has delay.
- State needs component + controller chunks and a fresh-instance warm-up test.
- Release processor/connection-point refs before `terminate`; factory before
  `ExitDll`.
- Scanner children need `SetErrorMode`; job object is still missing.
- Probe uses `Mutex`; product needs bounded real-time-safe queues.

### Phases

| Phase | Scope | Gate |
| --- | --- | --- |
| V0 | Probe/architecture validation | Done outside repo |
| V1 | License/notices; `opentrack-vst3` host context skeleton | No audio-graph change; review |
| V2 | `opentrack-plugins` catalog + `opentrack-plugin-scan`; folders, cache, PE check, class-ID dedupe, timeout/crash, job object | No audio change; review |
| V3 | Block graph + stable plugin runtime registry + PDC | Highest risk; do not mix with pitch work |
| V4 | VST3 effects, stereo main, state envelope, bypass, generic parameters, missing plugin | User audible/state review |
| V5 | Native floating editors | Visual review |
| V6 | VST3 instruments, live MIDI, `IMidiMapping` | Instrument review |
| V7 | Automation/offline/multi-out/sidechain/sandbox/CLAP; VST2 decision | Post-v1 |

V1/V2 may run in parallel with T042 because they do not touch the pitch graph.
V3 must wait for T042 to be accepted or be isolated in its own worktree.

## Delegation plan

Up to ten lanes are authorized. Start only lanes that do not conflict.

1. T042 synthesis: mark/phase continuity (`pitch_synthesis.rs`).
2. T042 live audition/drag publication (`pitch_editing.rs`, `sample_editor.rs`,
   `app.rs`).
3. T042 dry/wet, voicing and time compression (`pitch_synthesis.rs`).
4. Note color (`pitch_editing.rs`, `sample_editor.rs`, tests).
5. **Batch 3 integration owner** (gated): C1 contextual scissors + C2 snap-aware
   cursor first, then C3 wheel map, then C4 fader cap + C5 shared selection.
6. VST host context/license skeleton (new files only).
7. VST scanner/catalog (new files only).
8. Verification/ledger/git integration.

Collision rules:

- One agent owns one file set; no two agents edit the same file.
- Batch 3 agents may prepare tests/specs but not ship behavior before the gate.
- **Batch 3 must be serialized, not three parallel lanes.** C1/C2 share
  `audio_events.rs`, `system_tool_icons.rs`, `app.rs`, `pitch_editing.rs`, and
  `sample_editor.rs`. C3 shares `app.rs`/`sample_editor.rs`. C4/C5 share
  `mixer.rs` and the two `app.rs` Mixer call sites. Use one integration owner for
  `app.rs` and land the groups in order.
- Note color shares `pitch_editing.rs`/`sample_editor.rs`; do not run it beside a
  Batch 3 editor/cursor lane.
- VST agents must not touch the pitch/playback graph until V3 and T042 are
  coordinated. V1/V2 new-file work can run in parallel.
- Use separate worktrees or branches for code; keep this docs/plan checkpoint on
  `main`. The automated worktree tool was unavailable at dispatch time, so the
  first lanes are using disjoint files and must be reviewed before merging.
- Merge in the order: T042 audio fix, note color, Batch 3, VST V1/V2, then V3.
- Every merge updates `current.md`, the relevant task packet, and the ledger.

### Batch 3 audit result (2026-10-25)

- `project-event-contextual-scissors`: absent. Existing Split tool cuts anywhere
  in the event; contextual name-strip cut must be added for Select.
- `snap-aware-scissor-cursor`: partial. Cursor is always Crosshair+glyph; no
  scissors-alone when Snap is off and no pencil-alone for range drawing.
- `shared-view-wheel-map`: partial. Plain/Shift/Ctrl+Shift work; Ctrl+wheel
  horizontal pan is missing in both views.
- `mixer-fader-and-track-selection`: partial. Fader cap is rounded, not
  rectangular; footer click selects, but channel-body click does not, and Project
  multi-selection can disagree with Mixer/Inspector.
- T039 remains `review_pending`; coordinate Project/event-file changes before
  editing the same areas.

## Git and handoff

The current checkpoint is committed before parallel code work. The commit message
must state that T042 listening is pending and that the commit is not acceptance.
The plan and VST plan are committed separately with a clear upcoming-work
message and pushed to `origin/main`.

Every delivered task records:

- actual model/implementer;
- source identity and executable hash;
- commands/checks and actual results;
- runtime/listening/visual evidence separately;
- ledger requirement updates and stale-evidence changes;
- `python scripts/parity_report.py`;
- next ready task.

Do not self-approve audible or visual quality. Do not publish a build as accepted
while the user’s listening gate is pending.

## Paused goal tracker

The Codex goal is paused for user review. It is not complete:

- Batch 1 accepted.
- Batch 2 active and not accepted.
- Batch 3 pending.
- Note color and VST are additional tracks outside the original three batches.

The repository documents, not the paused tracker, are the durable source of
truth.
