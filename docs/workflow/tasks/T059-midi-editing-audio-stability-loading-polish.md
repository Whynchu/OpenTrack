# T059 - MIDI editing, audio stability, and interaction polish

Status: active

## Goal

Improve MIDI note selection and drawing, investigate playback glitches during
live control changes, make popup behavior predictable, make loading progress
reflect real work, and remove white-box flashes during first render.

## Work plan

### 1. MIDI editing

- Add a pencil icon angled toward the upper left, matching the existing Tabler
  cursor style.
- Add Draw, Select, and Range tools to the MIDI editor. Right-click opens the
  tool picker only when the pointer is over the note grid.
- Make Draw mode create notes and Select mode select existing notes without
  creating notes on empty space.
- Add marquee selection and preserve the relative timing and pitch of selected
  notes when moving the group.
- Review deletion, copy, undo, and runtime interaction after the first pass.

### 2. Audio stability

- Reproduce playback-only glitches with a known project and note their timing.
- Repeat while adjusting track dB/faders, then while changing MIDI note
  controllers.
- Inspect callback timing, lock/allocation work, parameter handoff, and render
  rebuilds for each scenario. Add measurement before changing callback behavior.
- Make a narrow fix, then compare the same scenarios.

Current performance work: selected-note velocity uses atomic per-note targets
and a short smoothing ramp. MIDI note and controller drag gestures now defer
playback-plan rebuilds until release, removing per-frame rebuilds during those
gestures. The single final rebuild still needs playback measurement on the
user's setup.

### 3. Popup dismissal

- Inventory menu, context-menu, and popup behavior.
- Make outside-click dismissal consistent.
- Confirm whether inactivity timeout is wanted before adding automatic timeout.

The user chose outside-click dismissal only, with no inactivity timeout. A
shared handler is now applied to common transient/configuration windows: project
load errors, Event Colors, track add/rename/delete, key command/capture/conflict,
Workspace Organizer, About, VST3 parameter windows, Device Setup, Project Setup,
VST Connections, and Plug-in Information. Menu bars/context popups retain egui's
existing outside-click close behavior. A focused egui test confirms inside
clicks keep a window open and outside clicks close it. Transport, Preferences,
channel setup, and other specialized windows still need inventory/coverage.

### 4. Project and plugin loading

- Map actual project-load stages and replace placeholder progress with stage
  progress or an indeterminate state where work cannot be measured.
- Profile audio source preparation and plugin discovery/initialization separately.
- Preserve the working asynchronous project loader and keep loading updates
  responsive.

### 5. First-render flashes

- Reproduce the white-box flash on app-owned windows and plugin windows.
- Trace window creation, surface clearing, texture/shader setup, and plugin
  readiness as applicable.
- Avoid exposing the uninitialized surface until the first usable frame is ready.

First-render mitigation in source: the dedicated VST3 editor host now creates
its frame and child surface hidden, attaches the native plugin view and applies
saved bounds, then shows the frame. Standalone VST probes retain visible-on-
create behavior. This addresses the blank host surface before attachment, but
the plugin's own first usable paint and app-owned window flashes still require
runtime verification and further tracing.

## Current checkpoint

The MIDI source now includes the top-left-pointing pencil cursor, Draw/Select/
Range toolbar controls, a note-grid-only right-click picker, marquee selection,
and multi-note movement with shared deltas. Project inspector gain/pan now use
live mix targets instead of rebuilding the playback plan. Equalizer revisions
advance only when EQ data actually changes, so routine fader updates no longer
make the callback reconfigure unchanged filters. Selected-note velocity edits
now update atomic per-note targets, with 5 ms smoothing for sounding voices,
instead of rebuilding the playback plan on each slider drag. A focused audio
unit test passes. MIDI structure and controller-event edits still rebuild the
plan after drag release; playback impact has not been measured on the user's
setup. (During drags, the previous playback plan stays active.)

Project loading no longer displays a fixed progress fraction. The worker now
reports project read/validation, referenced-source discovery, each audio-source
open, and a list of VST3 inserts. The UI then initializes one VST3 insert per
frame and displays its name/count before each initialization. Workspace/audio-
engine finalization remains on the UI thread. A single slow plug-in or the final
playback-plan rebuild can still stall that frame; runtime profiling is needed to
quantify both.

Offline local-ASIO `cargo check`, the focused audio unit test, and the app test
binary compile pass. A workspace-wide `--no-run` is blocked by three standalone
VST probe examples using older editor-window API signatures. Runtime interaction
and visual review are pending. The user's instruction not to use computer
control remains active. Popup preference is outside-click dismissal only, with
no inactivity timeout. The background loader now reports stage progress;
finalization smoothness is still unresolved. The VST3 hidden-until-attached
mitigation has not been visually verified.

The current x64 local-ASIO package is `builds/x64/OpenTrack.exe`, SHA-256
`97F6FC253AF76FBFC9BF630C939F6D541C45F886AF41AC6ECB14A419472802DE`.
Popup investigation found that egui menu bars and popups use their default
click-close behavior. The shared outside-click handling above now covers the
common app-managed transient/configuration windows; the remaining specialized
windows are still outstanding. The handler skips dismissal while an egui popup
is open, so selecting from a nested dropdown does not close its parent window.
The focused close-behavior test passes, and the x64 local-ASIO release build
passes after this guard.

Follow-up: project audio and collapsed-group rows registered a full-row
click-and-drag interaction before the audio-event handles. That response
captured pointer drags before fade and clip-gain handles could start. Those
rows now use a hover-only response so the clip controls own their drags; the
track headers remain selectable. Audio-event interaction tests pass with a
simulated full-row hover response. Repackaging is pending because the build
script initially detected OpenTrack still running; after it was closed, the
x64 local-ASIO package rebuilt successfully.

The audio callback now reports an EMA render load and overrun count in the
status strip, while audio clip gesture edits defer plan rebuild until release.
These changes are tracked under T058; runtime performance under ASIO 256-frame
playback still needs measurement and listening review.

Project inspector follow-up: the selected track's M/S indicators now include
inherited parent-group state, and group rows/inspector indicators include solo
state from descendants. The inspector Monitor control now toggles the existing
track monitor setting for Audio tracks with an input route, and shows disabled
when the route is explicitly No Bus. Project track-header monitor painting uses
an ASCII icon key. Two unreferenced legacy RGBA logo assets were removed; the
active SVG branding and native app icon remain. `cargo check --offline -p
opentrack --features local-asio` passes. Desktop visual and live monitoring
review remain pending under the user's no-computer-use instruction.
