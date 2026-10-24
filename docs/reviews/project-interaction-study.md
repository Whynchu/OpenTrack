# Project interaction study

Date: 2026-10-23. User opened Cubase5 and explicitly authorized live study. This is observed evidence, not implemented OpenTrack behavior. Original Rust graphics remain required.

## Saved evidence

All reference images and manifests are outside Git: `../../../_analysis-tools/reports/opentrack-comparisons/Project-live-study/`. Captures are `Project-study-01` through `15`. Each manifest records the returned window, dimensions, origin and observation. Current Cubase root capture is approximately1906x1043; scaling remains unknown. Initial input occurred while the user was handing over the mouse; the user then confirmed it was idle. Later named probes were captured with Cubase foreground. No project data, colors, devices, arm states or audio were changed. Navigation returned to cursor1.1.1.0 and the initial bars1..93 zoom. Menus/dialogs were dismissed without choosing commands or saving.

## Important region distinctions

1. **Track color strip:** study13/14. The narrow colored strip is at the RIGHT edge of a track header, immediately before its black meter rail. Left-click at(353,195) opens a horizontal swatch bar, approximately378x67, beginning beside that track. The popup shows a first blue swatch and16 palette colors, plus a lower label showing Color1 in this state. First-swatch semantics remain unknown; do not infer them from color alone. The strip tooltip reads `Track Color Selector`; delay was not measured. OpenTrack currently paints a thin left-side type-color line without a color action.
2. **Blank Track List BELOW tracks:** study15. Right-click at(268,340), in the left track-list column, opens a flat add-track menu. Audio, Instrument and MIDI appear first; then Arranger, FX Channel, Folder, Group Channel, Marker, Ruler, Signature, Tempo, Transpose and Video. Browse Sounds follows, then Show All Used Automation and Hide All Automation. This menu does not include Duplicate/Remove Selected Tracks.
3. **Existing track name/header:** study11. Right-click at(280,190) opens Add Track as a submenu, followed by Select All Events, Duplicate Tracks, Remove Selected Tracks, Track Controls Settings; automation visibility; Disable Track, Set Record Folder, Auto Fades Settings; Create/Apply Track Preset. These are track-targeted actions. The menu must target the clicked track, with multi-selection rules still unknown.
4. **Empty timeline grid:** study09/10. Right-click below lanes or within an empty lane showed no lasting menu in these probes. Hold-to-open toolbox and preferences remain unknown. Do not reuse the blank Track List menu across the entire Project area.
5. **Toolbar Color Selector:** study02/03. It has a different vertical menu: Default Color, Color1..16, Select Colors. Select Colors opens Event Colors with standard colors, hue/sat/lum/RGB, Apply, editable palette slots, Reset/OK/Cancel. The dialog was cancelled. Applying/editing colors was not exercised.

## Navigation

- Study04: ruler left click(570,168) placed cursor13.1.1.0,24seconds at120BPM, stopped.
- Study05: left drag50px downward increased bar spacing from roughly16.4px to57px, changing view1..93 to about10..36, retaining cursor13.
- Study06: horizontal ruler drag246px moved cursor13 to17 with zoom unchanged. Whole-bar snapping was observed in this configuration.
- Study07:50px upward restored the wider1..93 view. Cursor mapped to27 at the pointer when the view clamped to project start. Exact zoom transfer law, edge autoscroll and modifier rules remain unknown; one measurement is insufficient for an exact formula.
- Study08: ruler right-click offers Bars+Beats, Seconds, Timecode, Samples,60fps(User); Time Linear and Bars+Beats Linear. Bars+Beats and Time Linear were checked.
- User explicitly described middle-button grab-panning. Sky's documented drag signature has no mouse-button parameter, so middle-button dragging was not injected or verified. Preserve this as a user-specified requirement, not a measured observation.

## Next bounded implementation and acceptance

Implement the right-edge clickable color strip and its horizontal palette, then the distinct blank-track-list and existing-track context menus. Reuse working Add Track and Remove Track routes; unsupported actions remain clearly disabled until implemented. Persist chosen track color and verify synchronization without claiming event-color inheritance or multi-track rules before observing them. Preserve the accepted docked/close-only Project and Mixer layout.

Handle ruler click/drag, zoom and grab-pan in a separate navigation slice with persisted zoom/scroll state, fixed track-header/ruler geometry and transport seek synchronization. Keep exact response law and middle-pan runtime acceptance pending until exercised. Rebuild, capture the same two-track states and compare against studies13/15/11 and04..08. Stop for screenshot review at the resulting milestone. No new product code or build was made during this study.


## Follow-up live study, 2026-10-23

The horizontal palette first swatch is grey and labelled `Default Color`, followed by 16 colors. This supersedes the earlier first-swatch unknown. Source now paints a grey clickable reset; build/runtime comparison pending.

A separate circle below the Audio Track type icon at approximately (183,211) has tooltip `Show/Hide Automation`. Clicking it collapses the 42px Volume automation lane and moves lower tracks upward; a second click restores it. The Audio Track icon itself has tooltip `Audio Track`. Double-clicking the name at (286,190) opens a white inline editor with the name selected, rather than a separate dialog. Escape cancels. Selection shows dark name text on a light header; unselected Audio02 has light text on a dark header. Palette-dependent text contrast and remaining click regions still require observation. No renamed text or color choices were committed to the reference project.


## Track type icon hover study, 2026-10-23

Live hover-only probes confirmed `Audio Track` over the audio type icon (screen 190,190) and `Instrument Track` over the Prologue01 piano/keyboard type icon (189,316). The type icon sits above the separate Show/Hide Automation control. The audio glyph is a compact waveform-like silhouette; the instrument glyph shows piano keys. The type icon is not an automation arrow. Preserve independent hover targets and original vector artwork in OpenTrack. Screenshots/manifests: `Project-live-study/Project-type-audio` and `Project-type-instrument`, outside Git. Captures are 1906x1043 at origin(7,0); scaling remains unmeasured. Hover tool verified both pointer positions and waited1500ms; this establishes tooltip visibility after the wait, not exact onset timing. No click, key, scroll, project selection or playback changes occurred in this study. Product implementation deferred per user request.


## Empty-grid OpenTrack probe and user-opened CPR reference — 2026-10-24

An OpenTrack-only probe on the preceding x64 build clicked an empty timeline lane with Object Select and then right-clicked an empty grid position. No event was created, and the right-click produced no lasting menu. This agrees with the Cubase Project-study-09/10 observations that an empty timeline grid is distinct from the blank Track List background. The probe did not change the project. The exact pointer-to-lane mapping in that run was not independently captured, so repeat against the latest build with a screenshot-confirmed empty lane before marking runtime acceptance final.

The user opened a Cubase example project for visual comparison. The `.cpr` file is not currently supported by OpenTrack, so only the already-provided Cubase window capture was used as read-only visual evidence; no Cubase control was operated and no Cubase data was changed. The OpenTrack comparison project remains `EXAMPLE.opr`, whose observed title is `Project - EXAMPLE`. Current side-by-side and crop images live outside Git in `../_analysis-tools/reports/opentrack-comparisons/`; see `M2-review-report.md` for capture conditions and per-row status. The latest toolbar/icon build still needs its own runtime screenshot because Sky currently lists no usable desktop app/window.
## Direct Sky recovery and verified label/selection slice — 2026-10-24

Direct `node_repl` plus `@oai/sky` worked after resetting the stale Node session and importing the package again. An empty browser CUA inventory and zero process MainWindowHandle had incorrectly been treated as startup failure earlier; the user confirmed the GUI was already open. That diagnosis is withdrawn. Reset/re-enumeration exposed the current OpenTrack and native picker windows. Use the full 1920×1033 screenshot region, not the 16×16 icon region. Keep one instance at a time, close it through its UI before building and when done.

Final successful x64 build: SHA-256 `E955D248D03ABCD311FE4864D311C78D7B37880BB3326F12832F8C47257055C6`. Build, formatting and whitespace checks passed; two pre-existing unused-function warnings remain. New screenshot `M2-EXAMPLE-selection-final-0.jpg` and manifest are outside Git. Conditions: 1920×1033, origin (-1,-8), scaling unknown, Windows dark appearance, Project top/Inspector left/Mixer bottom/Transport floating, title `Project - EXAMPLE`.

The live review found and corrected a dark Instrument name on a dark header: name ink had been chosen from its green strip. Project now computes name ink from the actual header background. The final image shows readable Audio/Instrument names. A second bug caused track selection to prompt for save; both Project/Mixer selection flags and the workspace fingerprint are corrected. Final runtime check changed Instrument 03 to Audio 02 from the empty grid, right-clicked with no menu/event creation, and Alt+F4 closed without a prompt. No OpenTrack process remained. Earlier false prompts were closed with Save under the user's existing authorization; no event, name, gain, pan, delay, arm or monitor edits were committed in this pass.

New comparisons: `M2-final-vs-approved-full.png`, `M2-final-vs-approved-project.png`, and `M2-label-contrast-before-after.png`. Approved overall reference remains workflow-L1/04-preferred-approved-0.png; its Channel Settings/Compressor controls are excluded. Detailed right rail, remaining toolbar geometry/actions, light appearance and other checklist gaps keep M2 incomplete. Project work stays uncommitted; accepted Mixer commit remains ac758cc at origin/main.