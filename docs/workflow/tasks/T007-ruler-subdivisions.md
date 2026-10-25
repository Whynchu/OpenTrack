# T007 — Fractional ruler detail and native move cursor

Status: review_pending. Actual implementer: GPT-6.1 Sol. User specification: quarter-note, eighth-note and sixteenth-note representations appear progressively as zoom increases; zooming out reverses that refinement. Use the former four-way held cursor instead of the custom hand. The user confirmed the corner-extension feature works.

Scope remains transport-v3; adaptive-ruler, range-move, middle-pan and affected ruler regressions are refined without adding or reweighting requirements.

## Implementation

- Removed the custom fist renderer. Range hover retains the native pointing hand; held range movement and middle-button panning request native Move (Windows IDC_SIZEALL). No custom cursor image/handle remains.
- The existing 1/2/4/8... bar strides remain. As pixel space allows, ticks/grid lines refine to quarters, eighths, then sixteenths. Their lengths/strengths distinguish hierarchy. Marks remain anchored to musical positions when panning.
- Bars+Beats gains partial-measure labels when readable: quarter positions such as `5.2`, eighths such as `5.1.3`, and sixteenths such as `5.1.2`. Fractional labels use sixteenth-slot numbering consistently. Grid ticks can refine before labels have enough space, preventing overlapping numbers. Other ruler formats keep their existing format labels.
- Both refinement and coarsening use actual bar width and measured label width. The minimum fine tick spacing is 12 logical pixels; labels require their measured width plus 12, with a 44px minimum. Thresholds are implementation choices based on the user's requested behavior, not measured reference zoom thresholds.
- Raised maximum zoom from 8 to 256 and shared the 0.25–256 bounds between ruler dragging and Project +/-/logarithmic zoom rail, so sixteenth labels are reachable. Default zoom is unchanged. Finest display subdivision in this slice is sixteenths.
- The selected Snap grid remains Bar; display density does not alter snapping. No audio, project schema, range-resize logic, Mixer or tooltip changes were made. Existing time-signature/quarter-beat mapping is retained.

## Checks and identity

- Existing and final x64 builds passed; process absence checked before builds. No OpenTrack window opened and EXAMPLE untouched.
- Twelve timeline tests passed, including forward/reverse subdivision thresholds, 3-beat bars, label spacing and expected musical labels. Nine actual headless ruler tests passed, including inspecting emitted visible ruler text at successively finer zooms and returning to bar labels; existing seek/zoom, range movement/resize/Undo, middle pan and release behavior passed.
- Formatting/diff checks passed. The prior unrelated full-suite theme failure remains documented and was not changed or rerun.
- EXE SHA-256: `4FD865CF370BE82707BB910971E4AB99A5FE1D6B53D33DB2CF583698BD36335E`.
- `app.rs`: `FC05D7F3224DFFD3A51F87AA10F1D8F86EAA2FCEBA8C323360080FC6E46ACFB2`.
- `timeline.rs`: `27B80128DB4B7673813AC7BFA12972AAE7FC93D7F9B78B5F595D1A978F5EDCCC`.
- `project_controls.rs`: `F1A56FF32FE33EEC1A5344FA403684A4C3449484CC391D901C2ADBEBA4382472`.
- Base commit `cbd9835` plus existing uncommitted transport work. Current Windows visual acceptance and reference parity remain pending; no screenshot was taken because the user is performing the desktop check.

## User review

1. Move a range and middle-pan the ruler/canvas. Check that the held cursor is the native four-way move icon and that no custom fist appears.
2. Zoom steadily in using either the lower Project control or vertical ruler drag: bar labels should give way to quarter-note, eighth-note and sixteenth-note detail as space permits. Zoom back out and check that the sequence coarsens without crowding.
3. Pan at several zooms; check stable labels/grid alignment, playhead seeking, current Bar snap, corner resizing and range movement. Recheck the pinned ruler and horizontal scrollbar at the expanded zoom range.

Stop for the user's feedback before another substantial packet.
