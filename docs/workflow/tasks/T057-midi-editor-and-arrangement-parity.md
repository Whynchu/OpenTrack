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
