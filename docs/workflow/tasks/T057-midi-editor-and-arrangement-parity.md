# T057 — MIDI editor framing, note preview and arrangement parity

- Status: review_pending — implementation updated; visual/runtime review still required
- Scope version and requirement IDs: v1; user request (2026-10-27).
- Desired observable outcome: MIDI Editor is a near one-to-one visual and interaction match to the Audio Editor, reusing its shared layout, controls, keyboard appearance, naming, and navigation. Intended differences: no waveform in MIDI and the MIDI expression/controller lane below the note area. MIDI note creation is available with primary left-button input in the MIDI Editor and does not require Pencil; middle-button drag is navigation only. Pencil remains required to draw MIDI parts in the Project arrangement.
- Reference evidence: User screenshots `Screenshot 2026-10-28 062247.png` (Audio Editor) and `Screenshot 2026-10-28 062346.png` (MIDI Editor), both 1919×469, dark theme and same lower-zone viewport. They directly show the visual gap listed below. Existing Audio Editor frame and keyboard rendering are in `sample_editor.rs` and `sample_pitch_view.rs`; MIDI editor and controller lane are in `app.rs`.
- Dependencies and decisions needed: none. Interpret shorter fresh default as a sixteenth note (0.25 beats); retain any saved user insert-length preference.
- Code scope and architecture boundaries: reuse current lower-zone and piano-roll data; keep MIDI content in project model; do not alter audio clips or plug-in routing.
- Acceptance checks: (1) Match Audio Editor toolbar order, icon-button sizes/states, spacing, timebase, tempo/meter, snap/grid/quantize controls, split-at-selection action, and cursor readout; substitute MIDI values in the same widgets/positions. (2) Match six-field info-strip geometry, typography, spacing, and active/disabled states; keep MIDI-specific field values. (3) Match overview-band height/placement, selected-track color header, 232 px inspector, keyboard/ruler alignment, content clipping, and pan/zoom behavior. Omit the audio waveform; put the MIDI expression/controller lane directly beneath the note grid, aligned to its beat axis and keyboard edge, never in a right-side column. (4) Reuse Audio Editor pitch-keyboard styling and conventions: piano key shapes/colors, octave note labels (e.g. `C#4`), narrow-row label visibility, and vertical zoom/pan model. Do not show MIDI key numbers after note names. (5) Notes use track highlight color; default insert length is 0.25 beats while saved preferences remain honored. (6) In the MIDI Editor, primary left-click creates a note in a valid empty cell with any tool selected; no Pencil prerequisite. Primary click/drag on an existing note selects, moves, or resizes it. Other mouse buttons never create/select/resize notes. Middle-button press/drag only pans and preserves note selection. Show a snapped ghost over valid empty cells. Pencil remains required for creating MIDI parts in the Project arrangement. (7) MIDI parts in Project match audio events' dimensions, shape, name band, selected outline/handles, and applicable selection/move/trim/split/delete interactions without gain/fade controls. (8) Compare both panels at the same viewport, theme, scale, and window size; retain new screenshots and resolve all listed discrepancies before user acceptance.
- Affected regressions: MIDI note editing, selected track synchronization, audio event gestures, project dirty/undo handling, lower-zone sizing.
- Essential gates: compile and build; live review of selection, drag/resize, scissors, drawing, and MIDI Editor layout remains user-owned.
- Review owner and milestone stop: user reviews after the updated build. The supplied captures remain gap evidence, not acceptance evidence.

## Handoff

- Actual implementer/reviewer model: GPT-6 Luna.
- Source identity and build hash: commit `ceb1fa8` is the baseline. The updated T057 changes are the current commit candidate. x64 executable SHA-256 `9AC7F22791A60F171F87C671B27F5FC1D9221C2EFD41FE0A490FF0F95D0CE48D`.
- Changes and reasons: reused the Audio Editor's piano key renderer, note names, pitch viewport controls, toolbar buttons and numeric field widgets; added persisted MIDI pitch navigation; aligned the controller lane beneath the piano roll; made note creation independent of Pencil and restricted note drag editing to primary-button input. Kept arrangement Pencil for MIDI part creation. MIDI parts use audio-like selection, geometry, and applicable editing behavior. Replaced README screenshots with the supplied full-window workspace image.
- Commands/checks and actual results: `cargo fmt --all`, `cargo check --offline -p opentrack`, and `git diff --check` succeeded. `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -Architecture x64` succeeded with local ASIO enabled; packaged `builds/x64/OpenTrack.exe` SHA-256 is recorded above.
- Runtime environment and evidence paths: Windows x64; no live UI screenshot captured.
- Ledger IDs updated; report command/result: T057 and `docs/workflow/current.md` updated; successful `cargo check` and x64 release build above.
- Review findings and rework: formatting and compilation issues resolved before packaging.
- Failures/unknowns/pending acceptance: live GUI layout, overview navigation, middle-button pan, note hover/click/drag, and arrangement part interactions are not verified in this turn. User review remains pending.
- Next ready task: user review.

## 2026-10-28 visual review — rejected; parity gap recorded

The user supplied same-size (1919×469) dark-theme lower-zone captures:
Audio Editor (`Screenshot 2026-10-28 062247.png`) and MIDI Editor
(`Screenshot 2026-10-28 062346.png`). The MIDI screenshot is substantially
different despite the earlier claim that the frame had been matched. Do not
call T057 visually complete until the following gaps are corrected:

| Area | Audio Editor reference | MIDI Editor observed gap | Required direction |
|---|---|---|---|
| Main toolbar | Four compact icon tools, musical timebase, tempo/meter, snap/grid/quantize, split-at-selection, cursor readout in one row | Controls have different names/order; MIDI snap and length controls do not follow the same visual pattern; split-at-selection is absent | Reuse the same toolbar components, arrangement, dimensions, labels where shared, and enabled states; substitute MIDI values in the same positions |
| Info strip | Six consistently sized fields directly below toolbar | MIDI fields differ in labels/presentation and do not read as the shared strip | Keep MIDI data but use Audio Editor's exact field components, dimensions, and spacing |
| Track/part inspector | 232 px sidebar, colored track-name header, compact collapsible sections | Generic gray TRACK/PART/NOTE INPUT headings and metadata dominate | Match header color, width, typography, spacing, and section treatment; retain relevant MIDI content only |
| Keyboard | Piano-like keys with black-key overlays; octave labels such as `D#4`, no MIDI number suffix | Flat alternating pitch rows; labels such as `E · 124`, no octave naming | Reuse Audio Editor pitch-keyboard styling, sharp/octave naming, label clipping threshold, and navigation model |
| Note grid | Pitch view aligned to keyboard and time ruler; waveform belongs only to audio | MIDI grid has a different canvas arrangement; expression plot is a narrow column at the right | Use Audio Editor content geometry/alignment. Omit waveform. Put expression/controller lane below the note area, aligned to the same time axis |
| Navigation | Shared vertical pitch zoom/pan and horizontal time navigation, including middle-button drag | MIDI uses a full-height 128-row keyboard/scroll arrangement with different pitch scale and navigation feel | Reuse Audio Editor keyboard viewport conventions and middle-button navigation; adapt pitch range without changing the interaction model |
| Project MIDI part | Audio event bounds, selected/unselected treatment, label band and handles | Earlier user review found MIDI part size/shape unlike audio; latest source geometry adjustment has no visual evidence | Compare actual paint and hit bounds at equal zoom; matching lane height alone is insufficient |

Intentional MIDI-only differences are narrowly scoped: no waveform;
note-specific fields/content; and the expression/controller lane beneath the
note grid. They do not justify a separate visual language or navigation model.

### MIDI drawing input contract

The user clarified that drawing MIDI notes inside the MIDI Editor must not
require Pencil. Primary left-click in an empty valid cell creates a note; the
same primary-button drag may create a longer note. Existing notes retain their
select/move/resize behavior. No mouse button other than primary may create,
select, move, or resize MIDI notes; middle-button drag is reserved for panning
and preserves the current selection. The snapped ghost appears over valid empty
cells without a Pencil prerequisite. Pencil remains required to draw MIDI parts
in the Project arrangement. Verify press, click, drag, and release so gesture
paths do not trigger duplicate add/select actions.

### Prior implementation claim correction

The previous T057 update said the Audio Editor frame had been matched and left
the task awaiting acceptance. The screenshots disprove that claim. The prior
implementation/build is a failed visual review, not acceptance evidence.
Rework by sharing Audio Editor primitives rather than approximating its
appearance, then inspect the rendered app at matching window geometry before
updating completion status.

## Updated implementation and verification — 2026-10-28

- Reused the Sample Editor piano-key renderer, sharp/octave naming, pitch-view
  controls, toolbar tools, and numeric fields in the MIDI Editor. Added a
  separately persisted MIDI pitch viewport and placed the controller lane below
  the piano-roll canvas.
- MIDI Editor note entry does not require Pencil. Primary-button input creates,
  moves, and resizes notes; middle-button drag pans. Project Pencil still draws
  MIDI parts in the arrangement. MIDI parts share audio-event selection and
  applicable undoable edit behavior.
- Replaced the README screenshots with the user-provided full-window image at
  `docs/images/opentrack-full-workspace.png`.
- `cargo fmt --all`, `cargo check --offline -p opentrack`, and
  `git diff --check` succeeded. The x64 build succeeded with local ASIO enabled.
  Packaged executable SHA-256:
  `9AC7F22791A60F171F87C671B27F5FC1D9221C2EFD41FE0A490FF0F95D0CE48D`.
- Live visual and mouse-gesture review was not performed in this turn. User
  review remains pending; compilation does not establish parity or gesture
  behavior.

## 2026-10-28 piano-row and controller-label polish

- User feedback: semitone keys did not read as separate notes and controller
  text collided visually with the keyboard/grid.
- The shared `sample_pitch_view::key_row` renderer now draws pixel-aligned
  one-physical-pixel separators after the key fills, across the full pitch row.
  The keyboard portion receives higher-contrast, key-specific borders so
  neighboring white and black keys stay distinct at compact row heights.
- The MIDI controller title now sits in a dedicated raised strip above the
  plot, starting at the beat-grid edge rather than inside the keyboard column.
- `cargo fmt --all --check` passed. `build.ps1 -Architecture x64` completed
  successfully with `local_asio: true`. Packaged executable SHA-256:
  `C0754BE00707498512ABCE805035A3DE548449F725554786FB001ABC8CD7F634`.
- Reopened `EXAMPLE.opr` in the rebuilt application and captured a fresh
  1920×1033 screenshot. The semitone separators are visible and the controller
  name is separated from the keyboard. The screenshot was shown in the review
  conversation and was not archived. OpenTrack was closed afterward.
- This targeted readability fix does not claim completion of the full T057
  audio/MIDI parity checklist. User review remains pending.

## 2026-10-28 overview navigator rework

- User feedback: the white visible-range outline crossed the `MIDI 08` inspector
  header and did not read as a separate timeline overview.
- The MIDI overview now has a labeled, dedicated lane above the piano roll. Its
  note map, click/drag navigation hit area, and viewport outline begin at the
  piano-roll beat-grid edge, leaving the inspector and keyboard gutter outside
  the range box. The mapping still follows the full MIDI part duration.
- `cargo fmt --all --check` passed. `build.ps1 -Architecture x64` succeeded
  with local ASIO enabled. Executable SHA-256:
  `7B2D90C27B3FBB66F81BCDC5787906909E16ABD6536F1E55C9D01C5C77AD9C8B`.
- Reopened `EXAMPLE.opr` in the rebuilt app and captured the 1920×1033 desktop.
  The outline no longer crosses the inspector header. The current part-fit view
  makes it cover nearly all of the overview lane; panning/zooming to a shorter
  visible range should make the locator's bounded range more apparent. OpenTrack
  was closed after the capture. The screenshot was shown in the review thread
  and was not archived.
- This addresses the dedicated overview placement/function only; overall T057
  parity and user acceptance remain pending.

## 2026-10-28 MIDI viewport, ruler and click-to-create follow-up

- User clarified the overview should map the whole MIDI part to a bounded view
  frame, with a separate time ruler below it using Project-like transport
  seeking, and that Project should display notes created in the MIDI Editor.
- The overview frame now maps from visible piano-roll beats to the full part,
  with click-to-jump and frame drag-to-pan. The ruler uses musical position
  labels, seeks on click, and shows the shared transport playhead. Arrangement
  note previews offset each part-local note by the part's project start and
  clip notes to the part bounds.
- Runtime checking exposed that empty-cell clicks only cleared selection,
  while note creation only happened on drag release. Empty-cell clicks now add
  a snapped note at the insert length; click-drag continues to set duration.
- `cargo fmt --all --check`, `cargo check --offline -p opentrack`, and the x64
  release build passed with local ASIO. Final executable SHA-256:
  `F5A6B4DBE4B28820141B5F828A835179975909029045BFDC0DB2ADC8344038FB`.
- In the rebuilt application, clicked four notes and confirmed the Notes field
  reached 4; saved `EXAMPLE.opr`. The four notes are visible in the piano roll.
  The arrangement clip preview appears too small at the current project zoom to
  verify visually, so that visual confirmation remains open. OpenTrack was
  closed after the check. Screenshot evidence is in the review conversation,
  not archived.
- Full audio/MIDI visual parity and user acceptance remain pending.

## 2026-10-28 mini-note readability follow-up

- Reduced the arrangement preview's minimum MIDI note width so adjacent short
  notes retain their beat spacing at project zoom. Both mini previews now map
  pitch positions across the notes' actual pitch range instead of reserving a
  fixed 12-semitone span.
- Rebuilt x64 with local ASIO enabled. Packaged executable SHA-256:
  `1C9199C91C892C9A2E3031652D7CC94A15843989EE56BBF7CA69CC721A15239F`.
- Closed OpenTrack through its UI, saved the four-note `EXAMPLE.opr`, rebuilt,
  reopened the saved project, and captured a fresh 1920×1033 screen. The
  Transport Panel was absent from the reopened app and no longer covered the
  editor. Four note blocks remained visible in the Project clip and piano roll.
  Screenshot is shown in the review conversation and was not archived.
- T057 is still in rework pending the user's review of the MIDI map and broader
  Audio Editor parity. OpenTrack remained open at this review checkpoint; it
  was closed before the later divider/rail package described below.

## 2026-10-28 Project/editor navigation and scrollbar follow-up

- Project scrolling now reserves the rightmost 18 px for the track-height
  control. The vertical track scrollbar stays in the adjacent lane, and the
  bottom horizontal scrollbar ends before the time-zoom slider so neither
  control blocks the other.
- Runtime review on the rebuilt x64 app: plain wheel over the left track list
  moved track rows while the ruler scale stayed fixed; plain wheel over the
  arrangement changed time scale while track heights stayed fixed. The
  temporary scroll and zoom changes were restored. MIDI plain-wheel zoom
  changed time scale without moving pitch rows. Direct Audio and MIDI overview
  frame drags shifted their visible ruler ranges; MIDI Fit Part restored its
  view afterward. The right-rail separation was visible in the Project
  screenshot. Modifier-wheel gestures still need direct user/runtime review.
- Formatting, `git diff --check`, and `cargo check --offline -p opentrack`
  passed. `build.ps1 -Architecture x64` succeeded with local ASIO enabled;
  executable SHA-256 is
  `A9E8DBAABCE422DF74567832D60CB2D4EEC5C22D43076AF4FE9A2A1455BB7982`.
- This closes the identified Project scroll/zoom lane overlap and documents
  the tested wheel separation. T057's wider Audio/MIDI visual parity and user
  acceptance remain pending. The reviewed Project selection was saved, and
  OpenTrack is left open on EXAMPLE with the MIDI Editor ready for user review.

## 2026-10-28 Expression resize and rail sizing rework

- The earlier response-based expression-lane drag did not resize at runtime.
  Replaced it with a primary-pointer press-origin gesture on the controller
  lane's top 14 px, tracking pointer deltas directly and suppressing controller
  event edits for that gesture. Dragging downward moves the divider down and
  shortens the expression pane. Height is clamped to 96–480 px, default 126 px.
- Slider selection, custom zoom-thumb, and pitch-navigation-thumb highlights
  use gray. Solid scrollbar width is 16 px. The Project track-scroll and zoom
  controls share a 20 px right rail; the zoom control is 94 px tall, matching
  the Audio/MIDI editor rail geometry.
- `cargo fmt --all --check`, `git diff --check`, and
  `cargo check --offline -p opentrack --features local-asio` passed. The
  standalone release compile passed, followed by
  `build.ps1 -Architecture x64`; build metadata confirms local ASIO is enabled.
  Packaged executable SHA-256:
  `30247E9AC3766CB67269409CC90920DDD6899B8E1DC4F1A1FFB503D66CED1BB9`.
- Runtime drag, slider colors, and rail geometry were not rechecked because
  the user requested no more computer use for now. The new divider interaction
  is not yet proven. T057 remains in rework pending runtime and user review.

### 2026-10-28 recovery from accidental app.rs reset

Actual implementation/review model: GPT-6.1 Sol. Replayed recorded FileChange
patches and direct text edits from the earlier October 28 sessions in an isolated
recovery directory. Recovered the Project body/readout layout, stacked right
scroll/zoom lane, MIDI inspector/velocity styling, Project note previews, MIDI
ruler time zoom, overview viewport, gray controls, and expression divider. Merged
only the affected view functions into the current source to retain the user's
confirmed-working background project/audio-source loader.

The dead group composite was caused by applying playback solo/mute filtering to
editing waveform geometry. An unrelated solo (MIDI 08 in the supplied screenshot)
removed every child envelope. Group previews now remain visible like individual
clip waveforms. Combined events still appear only while the group is collapsed
and its children are hidden.

Expression layout now allocates the full current controller height, reserves
horizontal scrollbar thickness, and uses separate note-grid and controller
responses. The divider remains at its dragged size after release. Fixed piano
painting and note clipping keep the keyboard gutter outside note gestures.

Validation: cargo check with local-asio and git diff --check pass. Two focused
headless regression tests pass: solo/mute waveform invariance and visible
expression header plus actual pointer-drag/release sizing. Repaired existing
test-only API signatures in app.rs/vst3_host.rs to compile them. A broader ruler
suite reports 8 passed, 8 failed, 1 ignored; multiple fixtures still hardcode the
former 198 px header while the product uses 264 px, with additional fixed pointer
positions. That suite is not claimed passing. Project raw wheel navigation and
middle-pan checks pass. Desktop review remains pending under the user's request
for no computer use. The recovery folder retains source/patch checkpoints.

Final recovery package: builds/x64/OpenTrack.exe, local_asio true, SHA-256
8EAD799A049B2A305756CA5AE464525B5B155F8387DFC895F42DB1E90BE90245.
Build completed 2026-10-28 21:07 local. The MIDI ruler uses the actual nested
scroll area ID; overview viewport dimensions now use the actual scroll output.
The focused waveform and expression drag/release tests pass after that change.
