# Current assignment — T065

## T065 — dynamic shelf direction correction

User approved fixing the audio and animation together after the low/high shelf dynamic movement was found to amplify the opposite Q flank. Static shelf gain, Q and flavor are retained; dynamic movement now uses a separate monotonic shelf stage in the existing secondary filter slot. The graph shares this response model with audio. No new dependency, project schema or knob/layout changes in this correction.

Actual implementer/reviewer: GPT-6.1 Sol, baseline `901de54` plus the preserved T064 working tree. Full audio suite: 110 passed, 3 manual tests ignored. Channel Settings: 19 passed, including the actual envelope mesh direction check. Generated audio agrees with the plotted response; tests cover all shelf variants, positive/negative movement, gain limits, disable/bypass and Bell behavior. Existing OPR EQ/tonal-reference round trip passes. An offscreen render of both shelves was inspected; temporary renderer removed and image retained only at `/tmp/shelf-response-review.png`. Live listening/animation and Windows runtime remain pending. Linux local-ASIO release built and staged at `builds/x64/OpenTrack`, SHA-256 `8eea778ca70b347677c1d9bdbfd0d6fab52db0ecb6614f0c1ca5f39234d0d7f3`. See [T065](tasks/T065-dynamic-shelf-direction.md) for delivery evidence.

## T064 — tool-window pinning, save-prompt priority and luminous LEDs

User chose XWayland when available to retain separate native windows with supported raise/pin behavior. Linux main and splash windows now prefer the existing winit X11 backend when `DISPLAY` is advertised. `OPENTRACK_WINDOW_BACKEND=wayland` retains native Wayland for testing; pin/raise remains unsupported on that fallback. This follows the pinned winit 0.30.13 source and its [window-level documentation](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html#method.set_window_level). Focus for the app-scoped pin now includes every OpenTrack viewport, including Transport, and modal dialogs release Transport's pin. Windows Channel Settings retains ordinary owner-relative ordering.

Save/Discard/Cancel is now a true modal shown last in Project. On call, sibling app viewports are temporarily hidden and Project is focused once; Cancel/completion restores still-live viewports without changing saved pin preferences. Native Windows VST editor containers are hidden/restored without closing the editor or stopping processing; hidden HWNDs remain alive. This prevents tool/plugin windows from covering the save prompt without repeatedly stealing focus.

All shared knob lights now use a brighter emitted-light version of the track color with two halo layers. Purple keeps its hue and gray stays neutral; the original SVG and unlit dots are unchanged. Threshold's input-RMS source and -90…0 dB mapping are retained.

Actual implementer/reviewer: GPT-6.1 Sol; baseline `901de54` plus local work. Thirty-four focused checks pass: Mixer 8, Channel Settings 18, save-prompt UI/restore/focus 3, level policy 2, Linux backend policy 1, splash geometry/timing 2. Local-ASIO compile, formatting and diff checks pass. An actual egui offscreen raster confirms brighter purple/gray lights at 34/48 px; the temporary renderer was removed, PNG only under `/tmp/brighter-knob-leds-review.png`. Full-app headless initialization stalled and was stopped; the prompt's direct UI test passes. Windows cross-check is blocked by missing SDK `rc.exe`, not established as a Windows pass. Live compositor activation/stacking, desktop save-prompt priority and native VST editor restore still need runtime/user review. Normal-directory local-ASIO release build passed and is staged at `builds/x64/OpenTrack`, SHA-256 `3d32ee5512fc282066928aaf5641fa240b5c968f253afc406876c486580817ee`.

## T063 — shared strip height and input-gain control

User requested the extended strip match the proportions in `reference/good mixer panel.png`, fixing the centering visible in `reference/extended bad mixer panel.png`, in both Channel Settings and Mixer. The shared fader renderer now uses all available height with its existing fixed end padding; the 240 px travel cap is removed. The existing gain curve, handle and meter/footer positions are retained. Both views now use one shared input-gain/phase/value control, with 34 px artwork and hit area (previously 24 px), partly occupying the mono/stereo band while leaving its text clear. Numeric fields and the knob do not overlap. Per-frame knob dragging, Shift fine adjustment and double-click reset are shared as well. The user SVG itself is unchanged; no schema or DSP changes.

Actual implementer/reviewer: GPT-6.1 Sol, baseline `fbee87f` plus the existing dirty working tree. Seven Mixer checks and 18 Channel Settings checks pass; local-ASIO compile, format and diff checks pass. Reviewed an offscreen raster of the actual Channel Settings and Mixer strip renderers at 430 and 650 px heights. Temporary rendering code was removed; the PNG stays only in `/tmp/channel-strip-height-review.png`. User accepted this channel-strip refinement and requested GitHub publication on 2026-10-31 ("this is fantastic! push to github."). Agent live desktop/hardware checks were not performed. Normal-directory local-ASIO release build passed and is staged at `builds/x64/OpenTrack`, SHA-256 `57d7be7e9678873904bce4ff7ae60bb6546e88292ebf7cad0c4d04b3ff33d0b7`.

## EQ editor single-column arrangement and input LEDs — 2026-10-31

User accepted the compact adjacent-label/value arrangement, requested one vertical column, and explicitly requested track-colored LEDs and a live input-dB display for Threshold. The side panel is now one 168 px strip: Gain, Frequency, Q, Filter, then Dynamic EQ power, Threshold, Range, Attack and Release. The 48 px user knob SVG remains unchanged; row spacing remains 58 px. Content height is 514 px with independent scrolling in shorter windows. Default Channel Settings size is 1120×650, minimum 1010×420, preserving usable graph width.

All EQ parameter LED fills now use the selected track/bus color. Threshold lights show calibrated broadband pre-EQ linked-stereo RMS on the same -90…0 dB scale as its pointer setting; other LEDs follow their parameter values. The input reader reuses 20 ms of the existing bounded spectrum tap (maximum 2048 frames) off the callback, with no FFT or additional callback writes/allocations. Its reading is independent of output fader, Post/Pre display choice and Freeze, clears on source changes, and expires after 120 ms without fresh samples. The tooltip identifies pre-EQ input RMS. Audio processing and project schema are unchanged.

Actual implementer/reviewer: GPT-6.1 Sol. All 18 Channel Settings tests pass, including track-color/input-dB LED count and single-stack geometry/input isolation. New audio checks verify calibrated pre-EQ stereo RMS, silence, rate/reset/disable, and bus input gain independent of the output fader; the stale/owner/Freeze UI-state check passes. Local-ASIO compile, formatting and diff checks pass. An offscreen raster of the actual UI shows one clean column with track-colored lights; the temporary renderer was removed and PNG remains only in `/tmp`. Running-app visual/listening and user acceptance remain pending. Normal-directory local-ASIO release build passed and is staged at `builds/x64/OpenTrack`, SHA-256 `9dbfabf375670199777b9e57a03f25fe71031fb367666ffac007329087e580ff`.

## T062 EQ and tonal reference goal

The user clarified on 2026-10-30 that this feature expands the existing
Channel Settings EQ. The user resumed the goal on 2026-10-30 after
confirming the product direction. The first acceptance milestone is the
existing Channel Settings EQ workflow:
up to 32 user-created EQ bands, responsive short-spectrum analysis, dynamic EQ, and a separate
long-capture tonal-reference mode with capture/import and saved target
comparison. The broader goal continues through separately scoped advanced EQ
milestones from T062's Later enhancements list after these stages are accepted.
On resuming T062, the analyzer's stale 250 ms sequence gate was found to discard
live frames whenever audio progressed; it now consumes distinct frames
immediately and marks explicit capture incomplete on detected sample-stream
discontinuity. Focused analyzer and band-strip checks pass, including capture
invalidation, target tap summaries, efficient responsive wrapping, and band creation,
removal, and capacity. Pointer-level app tests cover add, remove, graph
double-click creation, and dragging the detail panel; they also exposed and
fixed the invalid zero-rate graph fallback (48 kHz is used until a rate is
known). The tonal toolbar displays target name/source tap and
signals when captured target tap differs from the live comparison tap. The EQ
strip has a bottom-right `+`; right-click removes a band in either the strip or
graph; double-clicking empty graph space creates one at the pointer. The detail editor opens
at the graph top and remains stationary until dragged by its title bar. Current
project/DSP limit is 32 bands. A current local x64 Linux release with
`local-asio` is at `builds/x64/OpenTrack`, SHA-256
`91ee5ca435336df490d0d44184ecbb94394ab04a58bb6fe74a71e87a44116ea9`; runtime
visual and capture review remains outstanding.
The detail popup is grouped into Static EQ and Dynamic EQ sections. Gain,
frequency, Q, threshold, range, attack and release have labeled knobs with
editable numeric values. Dynamic EQ has its own power button, independent of
band enable and the overall EQ bypass; focused interaction tests verify this.
Its graph representation has a soft shaded maximum-range envelope and a
brighter filled curve for live gain movement past Threshold. Each envelope
uses its own band's Q for width and response change for dynamic pull; its upper
edge follows the combined static EQ curve so it joins the graph continuously.
The selected band node has a bright center and
channel-color halo. The detail panel has static/dynamic sections in a compact
176 px layout, with band and DYN power controls beside their headings. Dynamic
range shading is a continuous shared-vertex mesh anchored to the combined
static EQ response at the upper edge; the lower dynamic boundary has no
horizontal end caps. The filter type is shown as a bordered dropdown and now
offers explicit 12/24 dB per octave low/high-pass choices. The 24 dB options
use two cascaded sections with a Butterworth-aligned Q pair; shelf transition
width is controlled by Q. Sixteen focused EQ interaction tests pass.
The OPR integration test now also round-trips captured tonal targets with both
Pre and Post tap metadata; direct Channel Settings save/reopen review remains
pending. Mixer and Channel Settings input-gain knobs now use the supplied
`knob_tight.svg`; their radial lights show live peak level in track color and
turn red above 0 dBFS. EQ parameter knobs use the same artwork without signal
lights, keeping parameter indication separate from audio level. Linux x64
local-ASIO release is at `builds/x64/OpenTrack`, SHA-256
`500e59a544ccb5750aa6148b48a4df72d26cbd53cffc274d1497ff391d76757b`; visual
and audio review remain pending. The EQ knob artwork was enlarged after the
provided screenshot exposed a 16 px clipping area: rows are 48 px, dials are
26 px, and the detail popup is 156 px tall.
Routing audit: Channel Settings edits `ChannelEqualizer`, consumed by track
`ChannelProcessor` and output `BusRuntime`. 32-band EQ and dynamic
processing are implemented in source, with per-band coefficient/enable ramps;
band-solo audition accepts all 32 bands on tracks/groups and Stereo Out.
Tonal view has an independent Pre/Post tap, relative-dB axis,
reference envelope and transport-aware capture state; save dirty-state wiring
is connected. Tonal graphs now label frequency-aligned Low, Low-Mid, High-Mid,
and High regions; the difference view shows a ±3 dB guidance band. Short-spectrum
Freeze and amber Peak display are implemented;
Peak falls at 12 dB/s; Freeze preserves the last spectrum through silence, and
transient holds clear when changing analyzer mode or tap. The display now has
-60/-90/-120 dB ranges and 0/3/6 dB-per-octave tilt that affects only the short
spectrum drawing. The band strip follows selection changes without fighting
manual scroll. The short spectrum can select 2K/4K/8K FFT resolution from the
existing 8192-sample bounded tap; long-window tonal analysis uses its 8192-point
window and 96 interpolated log-frequency samples. Tonal graph traces get
display-only Gaussian smoothing, and the tonal view hides the aggregate EQ
response curve, band nodes, DYN overlays, and the selected-band detail panel;
graph editing is disabled in that mode. Existing persisted Preferences > Metering controls temporal release
smoothing. EQ-band solo now uses a transient, smoothed band-pass audition for
tracks/groups and Stereo Out without dirtying EQ project state. Focused audio
tests and local-ASIO compilation pass. The EQ detail popup is 176 px high with
tighter section spacing and left-aligned band/DYN power buttons. Bell/parametric
dynamic envelopes follow each band's own response until both edges return to
within 0.08 dB of baseline, preventing a visibly clipped notch. WAV reference
import now analyzes mono or
stereo PCM/float files off the callback with cancellation, silence rejection,
and stale-owner/target-change rejection; target profile and provenance are
embedded in the project without a source-file dependency. The OPR round-trip
test now also verifies bypass, eighth-band filter parameters and dynamic EQ
settings together with the imported target. Generated-WAV and project migration
tests pass. Tonal mode now has a selectable 10/30/60/120-second weighted-average
time constant, Hold/Resume independent of explicit capture, valid-signal warm-up
status, and Reset; it samples only the selected owner while Tonal mode is active.
Tonal live/import analysis uses the same -90 dBFS RMS silence gate, measured
from unwindowed stereo samples independently of frequency. Focused app tests
cover weighting, Hold, silence gating, initialization, and
invalidation when sample rate, channel layout, or tonal tap changes; prior
captured data is marked incomplete and cannot be saved.
Dynamic EQ movement is now exposed as lock-free per-band meter telemetry for
its graph overlay; disabled/ineligible bands reset to the static curve through
the existing smoothing ramp. Generated audio covers movement telemetry and
disable reset. The full audio suite passes 102 tests (3 ignored), including
dynamic movement publication through the bus meter. Attack/release exponentials
are now cached on parameter changes; dynamic level-to-dB mapping and biquad
coefficient updates run every 32 samples, while envelope and gain smoothing
remain per-sample, with an explicit control-cadence check. A three-run release
probe of eight bands at 48 kHz/128 frames on an i7-1165G7 measured p99 blocks
of 18.7–22.1 µs (0.70–0.83% of the 2.67 ms budget); worst observed block was
120.7 µs. An updated 32-band synthetic detector/filter probe at 48 kHz measured
p99 72.2 µs for 64-frame blocks (5.41% of 1.33 ms) and 144.6 µs for 128-frame
blocks (5.42% of 2.67 ms). It excludes the rest of the callback, host scheduling,
and device/driver; physical low-buffer playback review remains open. The EQ strip now has a
fixed bottom-right add button; right-click removes a band from the strip or
graph, and double-click on empty graph space adds a band at the pointer. The detail editor
opens at the graph top, stays put as selection changes, and can be dragged by
its title. The current project/DSP supports up to 32 bands. The latest full
app suite reports 168 passed, 15 failed, and 9 ignored;
failures are in Channel Window lifecycle, Project ruler/drag, and Sample Editor
tests, while focused T062 analyzer and graph tests pass. Live import/control
behavior, dynamic overlay review, physical low-buffer cost, visual/audible review and
target-capture accuracy remain open. The actual OPR writer/reader round-trips
EQ bypass, eighth-band parameters, dynamic controls and a target profile without
the source WAV; live OpenTrack save/reopen remains pending. The persistent goal
remains active.
See
[T062](tasks/T062-eq-and-tonal-reference-plan.md).

## Active goal T059 - editing, playback stability, menus, and loading polish

Work through these areas in small, independently reviewable steps:

1. MIDI editing: add the upper-left-pointing pencil cursor and Draw/Select
   tools; right-click opens the tool picker only over the note grid; drag a
   marquee to select groups of notes and move selected notes together. Then
   review runtime behavior and add any needed delete/copy/group-edit actions.
2. Audio stability: reproduce and measure playback pops during playback alone,
   track dB/fader adjustments, and MIDI controller changes; identify whether
   the glitches share a real-time audio cause before changing the engine.
3. Popup behavior: close menus and popups when the pointer leaves their
   activation point and selectable area; keep them above overlapping windows.
   No inactivity timeout.
4. Loading: report real project-load phases and investigate stalls from audio
   source and plugin initialization without regressing the existing loader.
5. First-render flashes: trace the white-box flash separately for app-owned
   windows and plugin views, then fix initialization/render readiness.

Current checkpoint: MIDI Draw/Select/Range controls, grid-only context menu,
marquee selection, and group move are implemented in source. Project inspector
gain/pan now update live, avoiding full playback-plan rebuilds for those
controls. Equalizer revisions now advance only when EQ values change, avoiding
redundant filter reconfiguration during fader drags. Selected-note velocity
changes now update atomic per-note targets and are smoothed over 5 ms for
already sounding notes, avoiding a full playback-plan rebuild on each slider
drag. A focused audio unit test passes. MIDI note structure and controller
drag gestures now defer plan reconstruction until pointer release, avoiding a
full rebuild on every changed frame. Main app utility/configuration windows now
close on outside clicks, and a focused egui interaction test covers the shared
close behavior. The shared hover-leave handler covers root UI popups and the
Media Pool, Channel Settings, and Transport viewports. It keeps menus open while
the pointer is in their activation hotspot or a foreground popup area, then
closes the popup chain after the pointer leaves. Native window ordering is
adjusted so the viewport containing an open popup appears above overlapping
OpenTrack windows; saved pin preferences resume after dismissal. No inactivity
timeout is used. Playback impact has not been measured on the
user's audio setup. `cargo check --offline -p opentrack --features local-asio`,
the selected-velocity audio test, and the focused popup test pass. A
workspace-wide `--no-run` remains blocked by three standalone VST probe examples
using older window API signatures. The focused MIDI follow-up received runtime
review on 2026-10-29; audio-performance checks, other T059 runtime behavior,
and overall milestone acceptance remain open.

2026-10-30 Wayland stacking follow-up: winit does not implement compositor
window-level requests on Wayland, so Transport and Channel Settings now use
foreground egui windows inside OpenTrack in that session. Their stored
positions and pin preferences are retained; FX raises Channel Settings, and
Transport keeps draw priority over it. Native viewports remain in use on
Windows and non-Wayland sessions. Linux local-ASIO compilation passes; live
stacking, dragging, and popup interaction need user review. See T048.

2026-10-30 click-through correction: before Project processes its pointer input,
the Wayland path now masks Project input while the pointer is inside Transport
or Channel Settings. It retains capture through drags that begin inside those
panels. The native window paths are unchanged. Linux local-ASIO release rebuilt
at `builds/x64/OpenTrack`, SHA-256
`6f4d1e9c6c6894171cf94462a55d282c8083da9c380c3fd92d0cdf70a88145e8`. Formatting
and compilation pass; live pointer review remains pending.

2026-10-30 focus dimming follow-up: popup menus and nonmodal utility windows
retain normal brightness. The existing dim treatment remains for modal
confirmation/setup dialogs. Linux local-ASIO release rebuilt; visual review is
pending.

2026-10-31 Plasma window-move follow-up: Channel Settings and Transport now use
native child windows in Wayland sessions instead of Project-owned egui overlays.
This lets KWin receive window moves and apply desktop effects such as Wobbly
Windows. The Transport drag handle uses the native viewport drag request;
Channel Settings can move from its native title bar. Wayland still does not
honor the app's always-on-top level requests. Linux x64 local-ASIO release is
staged at `builds/x64/OpenTrack`, SHA-256
`e997effd1f9a77034cd99035fb21ea5e73c7324bfa95af945725ef93243d7609`.
Formatting, diff check, offline local-ASIO check, and release build pass. Live
KWin effect behavior remains for user review.

Wayland popup regression follow-up: the embedded Transport and Channel Settings
now run the shared hover-leave handler, so VST selection menus and other popup
chains close after the pointer leaves both the activation point and selectable
popup area. No inactivity timeout is used. Linux local-ASIO compilation passes;
live interaction remains pending.

Live spectrum resolution follow-up: the Channel Settings analyzer now uses a
4096-sample FFT (10.8 Hz bins at 44.1 kHz, 2x denser than before) and an 8192
sample ring. This addresses coarse low-frequency spectrum points; the existing
temporal release smoothing is unchanged. The editable EQ response curve was
already sampled at 257 logarithmic frequencies. Playback/UI latency and visual
review remain pending. See T021.

2026-10-30 z-order follow-up: an open popup keeps priority over overlapping
windows, then the saved Transport and Channel Settings pin behavior resumes
after dismissal. Channel Settings uses owner-relative ordering on Windows, and
restoring its owner now explicitly reinserts it above Project after
popup/dialog transitions. Formatting, Linux local-ASIO `cargo check`, and
focused popup tests pass. Windows cross-compilation stopped in the app build
script because Windows SDK `rc.exe` is unavailable; native Windows stacking and
user review remain pending. See T048's z-order rework note.

MIDI tool/delete follow-up (2026-10-29): the MIDI editor tool strip and its
note-grid right-click picker now use Mouse, Scissors, Pencil order. Mouse empty-
grid drags use the existing marquee/group-move path; Scissors split a note at
the snapped beat while retaining pitch, velocity, and channel; Pencil inserts
notes. Project tools now place Scissors and Pencil alongside Mouse, with the
existing MIDI-part Pencil gesture retained. Multi-note selection is transient
app state so Delete in the active MIDI editor can remove the selected notes
through project history and rebuild playback. `cargo fmt --all -- --check` and
offline local-ASIO `cargo check` passed before packaging. The x64 local-ASIO
build was visually reviewed in `Documents/EXAMPLE/EXAMPLE.opr`: toolbar order
and the three-icon picker were confirmed, Pencil insertion and a 1/16-snapped
Scissors split were exercised, and all test edits were undone without saving.
Delete on a selected MIDI note changed the count 11 to 10; Undo restored 11,
with the MIDI track intact. Playback/audio, remaining popup/loading/first-render
work, and overall T059 acceptance remain open.

Project loading reports project-read, source-discovery, per-source opening, and
plugin initialization stages; it initializes one VST3 insert per UI frame and
shows the plugin name/count. A slow individual plugin can still block its frame,
and final workspace/audio-plan setup remains on the UI thread. The dedicated
VST3 editor host stays hidden until its view is attached and saved bounds are
applied; first-paint behavior and app-owned window flashes need runtime review.

Project panel follow-up: inspector M/S buttons display inherited group state,
group headers reflect soloed descendants, and the inspector Monitor button now
toggles audio-track monitoring only when an input route is available. Two
unreferenced legacy RGBA logo assets were removed. Local-ASIO compilation
passes; runtime display/monitor review remains pending.

MIDI editor follow-up: the hover note preview is visible only with Pencil
selected. Mouse now resizes notes from either horizontal edge, with a horizontal
resize cursor at the edge and the opposite endpoint held in place. Project
inspector Pan uses a left/right split icon. Low-buffer GATEWAY/Ozone/CLA-76
stress-test stages are specified in T061; no hardware stress run has been made.
See `docs/workflow/tasks/T061-low-buffer-neural-amp-stress-plan.md`.

T060 visual changes were reverted after the user said the enlarged Project
layout felt regressive. The previous toolbar, track-header positions,
disclosure controls, and 42 px default/36 px minimum row heights are restored;
MIDI editing changes remain. Formatting and the x64 local-ASIO package build
pass. The restored build was reviewed in EXAMPLE.opr and the original layout
was confirmed; user visual acceptance remains open. See
`docs/workflow/tasks/T060-project-icon-legibility-plan.md`.

## Previous task T057 - recovery after accidental app.rs reset

## 2026-10-28 recovery after accidental app.rs reset

Actual model: GPT-6.1 Sol. Recovered Project/MIDI view functions from the
earlier session's recorded patches and shell edits, replayed in an isolated
recovery copy. Preserved the current project loader that the user confirmed
works. Restored Project sidebar/scrollbar placement, record-readout/time-zoom
lane, MIDI ruler/overview navigation, note previews, neutral gray controls,
and the controller divider. The current source and patch checkpoints are in
`recovery-20261028-204244/`; do not reset the working tree to recover a single
change.

The group waveform was suppressed by unrelated track solo or inherited mute.
Editing previews now retain child envelopes regardless of playback mute/solo;
the composite remains restricted to collapsed groups (children hidden).
The expression lane shares its actual resized height with the allocated
note/controller stack and reserves horizontal-scrollbar space. The recovered
note grid has its own click/drag response. Piano keys are painted in the fixed
gutter and note hit/draw regions exclude that gutter.

Focused tests pass: group envelope remains nonzero and unchanged with an
unrelated MIDI solo plus group mute; expression header is visibly inside the
viewport and divider size survives drag release. Existing test-only call
signatures were repaired to compile these checks. `cargo check --offline -p
opentrack --features local-asio` and whitespace validation pass. An ASIO-enabled
x64 package passed with `local_asio: true`, SHA-256
`8EAD799A049B2A305756CA5AE464525B5B155F8387DFC895F42DB1E90BE90245`.
The broader ruler suite reports 8 passed, 8 failed, 1 ignored, including stale
198 px header assumptions versus the current 264 px header and other fixed
pointer coordinates; it is not claimed passing. Desktop review remains pending under the user's
no-computer-use instruction; compilation/tests do not establish visual parity.

## T058 — Realtime performance and stability program (2026-10-29, v2)

Goal is artifact-free playback at **64 samples** on the Mackie ASIO interface
with GATEWAY modelers and CLA-76/Ozone on Stereo Out; the driver reports roughly
5 ms at that setting, and 128 samples is a fallback rather than the target.
Implemented so far: EMA renderer load and deadline-miss counters, playback-plan
rebuild deferral during event gestures, ASIO input-bus DSP bypass when idle, and
a release profile with fat LTO. No measured improvement or stable 64-frame
workload is established yet.

Decisions recorded 2026-10-29: the device **hard ceiling is retained** (R10
closed — no host soft clip, knee, fold-back or limiter), and 64-frame stability
is the acceptance target. Next approved work is Stage 1 diagnostics with three
controls that are cheaper than profiling: a cross-host control in another ASIO
host, an ASIO callback scheduling-class A/B (the WASAPI thread enters MMCSS "Pro
Audio"; the ASIO driver thread currently requests neither), and an internal
hosted-block-size comparison at a fixed period.

See `docs/workflow/tasks/T058-realtime-performance-and-stability-program.md` and
`docs/workflow/tasks/T061-low-buffer-neural-amp-stress-plan.md`. 32 frames is a
stretch evaluated only after 64 passes its soak; it is never acceptance and must
not delay the 64 result. Oversampling is deferred rather than dropped: the
constraints are recorded in
`docs/workflow/tasks/T062-optional-insert-oversampling.md`, which is blocked on a
proven 64-frame workload and on a stated answer for the hard-ceiling overshoot.

### 2026-10-28 cross-editor navigation and scrollbar pass

- Project plain wheel input over the track-list/header scrolls tracks
  vertically; plain wheel over the timeline zooms time, and the modifier paths
  continue to map to horizontal pan and track-height scroll/zoom.
- The Project scroll area now ends beside a dedicated right-edge track-height
  zoom rail, so it no longer covers the vertical track scrollbar. Its bottom
  horizontal scrollbar ends before the time-zoom slider, leaving both controls
  side by side on the same lane.
- MIDI and Audio Editor now apply the same solid scrollbar style as Project.
  MIDI native wheel scrolling is disabled so shared modifier-aware wheel
  actions do not also move the horizontal ScrollArea. Audio overview viewport
  can be dragged directly to pan, matching MIDI overview behavior.
- MIDI viewport rail and zoom control extend alongside the controller lane;
  velocity dark fill applies to the slider track itself. MIDI ruler zoom/pan
  and part viewport updates from earlier passes remain in the working tree.
- Verification: `cargo fmt --all`, `git diff --check`, and `cargo check
  --offline -p opentrack` passed. `build.ps1 -Architecture x64` passed with
  local ASIO enabled. Packaged executable SHA-256:
  `A9E8DBAABCE422DF74567832D60CB2D4EEC5C22D43076AF4FE9A2A1455BB7982`.
- Runtime screenshot review confirmed the two Project zoom/scroll lanes do not
  overlap. With track-height zoom temporarily raised to create overflow, plain
  wheel over the left track headers moved the visible track rows while ruler
  positions stayed fixed; plain wheel over the timeline changed horizontal
  scale while track heights stayed fixed. Both view adjustments were restored.
  Audio overview dragging shifted the visible ruler range. MIDI overview-frame
  dragging shifted the visible bar range, and MIDI wheel zoom changed bar
  spacing without changing pitch rows; Fit Part restored the view. The
  modifier-wheel paths were source reviewed but not directly injected in this
  runtime pass. The reviewed selection was saved; OpenTrack remained open at
  that review checkpoint. The later divider/rail package and its current
  closed-app state are recorded below. Full T057 visual parity and user
  acceptance remain pending.

### 2026-10-28 MIDI divider and navigation rail follow-up

- The MIDI expression lane now has a press-origin resize gesture over its top
  14 px strip. Dragging that divider downward reduces the expression pane;
  resizing is gated away from controller-point edits. The lane is clamped to
  96–480 px with a 126 px default.
- Slider selection fills and custom zoom/pitch navigation thumbs use neutral
  gray. Shared solid scrollbars are 16 px wide, and the Project right-side
  track-scroll/zoom lane is 20 px wide with a 94 px zoom control, matching the
  Audio/MIDI editor rail size and placement.
- Formatting, `git diff --check`, `cargo check --offline -p opentrack
  --features local-asio`, and `cargo build --offline --release -p opentrack
  --features local-asio` passed. The packaged x64 build also passed via
  `build.ps1 -Architecture x64`; metadata confirms `local_asio: true` and the
  executable SHA-256 is
  `30247E9AC3766CB67269409CC90920DDD6899B8E1DC4F1A1FFB503D66CED1BB9`.
- The user requested no further computer use, so the current build was not
  opened for runtime interaction or screenshot review. The earlier response-
  based divider drag did not resize; this press-origin rework and the revised
  slider/scrollbar colors and widths remain runtime-unverified. T057 remains in
  rework pending that check and the user's broader visual review.

## T057 — MIDI editor framing, note preview and arrangement parity (2026-10-27)

The user asked for the MIDI Editor to use the Audio Editor's overall UI
structure while retaining the piano-roll look, with shorter default notes and a
snapped ghost note on hover that is committed by click. Notes should use the
track highlight color. MIDI parts in Project should share audio-event selection
visuals and applicable behavior, without audio gain/fade controls. See
`docs/workflow/tasks/T057-midi-editor-and-arrangement-parity.md`.

### 2026-10-28 note-preview readability follow-up

Project MIDI note previews now preserve their scaled beat duration instead of
forcing short notes to overlap at arrangement zoom. Both mini previews scale
pitch positions to the notes' actual pitch range. Built x64 with local ASIO
enabled, SHA-256
`1C9199C91C892C9A2E3031652D7CC94A15843989EE56BBF7CA69CC721A15239F`.
Reopened the saved four-note `EXAMPLE.opr` in the rebuilt app. The Transport
Panel was closed for the review capture; four note blocks remain visible in
the arrangement clip and piano roll. T057 visual parity and user acceptance
remain pending. The app is left open for user review.

### 2026-10-28 overview navigator follow-up

The user identified that the MIDI visible-range box crossed the inspector's
track header and did not have a clear overview role. The MIDI overview now has
a dedicated labeled lane: its note map and interactive viewport frame start at
the piano-roll beat-grid edge, leaving the inspector and keyboard gutter
outside the navigator. `cargo fmt --all --check` and the x64 release build
passed with local ASIO enabled. Packaged executable SHA-256:
`7B2D90C27B3FBB66F81BCDC5787906909E16ABD6536F1E55C9D01C5C77AD9C8B`.
A fresh `EXAMPLE.opr` screenshot was captured at 1920×1033. The viewport frame
no longer crosses the inspector header; in this saved view it spans nearly the
full overview width because the piano roll is fit to the part. OpenTrack was
closed after capture. This targeted rework does not complete T057 parity or
user acceptance.

### 2026-10-28 MIDI map and ruler follow-up

The user clarified that the overview must show the whole MIDI part at a glance,
with a viewport frame for the currently visible piano-roll section, and a
separate ruler below it. The viewport now maps visible piano-roll beats to the
full part overview; clicking jumps and dragging the frame pans. The piano-roll
ruler uses the transport's musical position formatting, seeks on click, and
draws the shared transport playhead. Project arrangement note previews now use
part-local note positions offset from the part's project start and clip to the
part bounds. Formatting, `cargo check --offline -p opentrack`, and the x64
release build passed (local ASIO enabled); executable SHA-256:
`61C5D7F80A521B1BB0D4E223439383318C3FFDD502DD2A64A7EE9C5A274D0D9F`. Review
then exposed that empty-cell clicks only cleared selection; note creation was
drag-only. Empty-cell clicks now add a snapped note at the saved insert length,
while click-drag still sets its duration. Formatting, `cargo check --offline -p
opentrack`, and x64 release build passed again with local ASIO; current
executable SHA-256: `F5A6B4DBE4B28820141B5F828A835179975909029045BFDC0DB2ADC8344038FB`.
In the rebuilt app, click-created four notes and confirmed the Notes field
incremented to four; saved `EXAMPLE.opr` and closed OpenTrack. The four-note
MIDI Editor screenshot was captured, but Project-window note preview at this
zoom remains too small to confirm visually. T057 remains in rework pending
timeline comparison and user review.

### 2026-10-28 readability follow-up

The user identified weak semitone borders and a controller title colliding
with the piano keyboard. The shared piano renderer now draws pixel-aligned
one-physical-pixel separators after the key fills, across both the keyboard and
pitch rows. Key borders use stronger contrast. The controller title now sits
in a raised strip above its plot, to the right of the keyboard. Formatting and
the x64 build passed with local ASIO enabled; a fresh runtime screenshot was
captured. This targeted polish does not complete the full T057 parity review;
user acceptance remains pending. OpenTrack was closed after review.

The 2026-10-28 side-by-side screenshots reject the prior claim of visual parity.
The MIDI editor still differs substantially in toolbar and field presentation,
inspector/header styling, keyboard rendering and note naming, and pitch
navigation. The expression/controller lane is below and aligned with the note
grid. MIDI note drawing inside the MIDI Editor does not require
Pencil: primary left-click on an empty cell creates a note and primary drag can
create its duration. Other mouse buttons must not create or edit notes, and
middle-button drag pans only. Pencil remains required for drawing MIDI parts in
the Project arrangement. T057 is in rework, not awaiting acceptance. The only
intended structural differences from Audio Editor are no audio waveform and the
expression/controller lane below the MIDI note area. See the visual gap table
and input contract in
`docs/workflow/tasks/T057-midi-editor-and-arrangement-parity.md`.

## Prior assignment — T056

## T056 — Plug-in window memory and MIDI arrangement/editor workflow (2026-10-27)

The user confirmed Gateway meters look good and requested persistent VST Audio
Channel Settings and VST3 editor positions, VST3 editors above OpenTrack's
Project and lower-zone panels, pencil-drawn MIDI parts on MIDI/Instrument
tracks, and a MIDI Editor tab alongside Mixer and Audio Editor. Existing
Channel Settings bounds already flow through `.opr` workspace snapshots;
verify close/reopen behavior and preserve the feature. Existing MIDI parts,
note editing and a floating MIDI Editor exist, but Project pencil creation and
the docked MIDI Editor tab are missing. See
`docs/workflow/tasks/T056-plugin-window-memory-midi-arrangement-editor.md`.
Implementation uses snapped drag duration (click makes one grid unit); drawing
leaves the part unselected and the current lower-zone tab unchanged, while
double-click selects the part and opens the MIDI Editor. Native VST editor
bounds use project-workspace snapshots with owner-relative z-order. Runtime and
user acceptance remain pending.

## Prior assignment — T055

## T055 — Splash isolation, VST editor animation, and loop pencil cursor (2026-10-27)

The user confirmed that fader/EQ response is substantially improved and that
Ozone can load on Stereo Out. Follow-up concerns: startup splash must appear
alone before the main window, Ozone editor animation should continue without
pointer movement, and the Loop Draw pencil must point upper-left with its tip
at the cursor hotspot. See
`docs/workflow/tasks/T055-startup-splash-vst-editor-animation-and-draw-cursor.md`.
The user rejected the first splash approach because it opened the application
small instead of maximized, and reported that forced redraw did not fix Ozone.
Discard that approach. Rework startup to use a standalone 600×327 splash while
the maximized main window remains hidden, then reveal the main window after
initialization and a two-second minimum. Fix the VST editor message pump to
dispatch thread-level timer messages, a likely source of idle animation stalls.
The requested Ozone behavior is specifically live EQ and level-meter movement
during audio playback without pointer movement. Keep the cursor-only upper-left
pencil change. The user clarified that the README SVG is authoritative, the
background must remain grey but darker, and the app window must not load until
the splash is finished. Startup now shows the exact SVG artwork over a uniform
dark grey background, then creates the maximized main window after the splash
closes. Runtime startup appearance and Ozone playback animation remain
unverified.

The latest follow-up also addresses the remaining display/build concerns:
Gateway now receives processor-produced VST3 parameter changes on the host
control thread, editor windows request regular repaints, the executable has a
multi-size Windows icon resource, and the release script keeps build metadata
out of `builds/x64` while replacing stale package contents. Main and floating
windows are created hidden and shown after their first UI frame to reduce the
startup flash. `builds/x64` was rebuilt; the installed Gateway smoke test loads
the plug-in and processes audio successfully. The live Gateway meter behavior
and cold-start appearance still need a GUI check because the Sky computer-use
service is unavailable in this session.

Linux splash follow-up (2026-10-30): the splash was missing because the
non-Windows `startup_splash::show()` implementation returned `false`. Linux
now shows the same artwork in a separate borderless eframe viewport while
keeping the Project viewport hidden, then reveals the maximized app after at
least two seconds measured from the splash viewport's first frame. Linux
local-ASIO compilation passes; runtime display review is
pending. The Windows native startup splash is unchanged. See the T055 packet.

2026-10-30 follow-up: Linux runs the splash in a standalone helper process and
waits for it to exit after the first-frame-based two-second minimum before it
starts eframe or constructs OpenTrackApp. This prevents a hidden or empty main
window from appearing during the splash interval.
The Channel Settings pin on non-Windows systems follows focus in either the
Project or Channel Settings window; it is released only when focus leaves the
app or a dialog takes over. Clicking FX now requests focus and raises the
Channel Settings window on Windows. Linux focused-pin and splash-duration
tests pass. The rebuilt Linux x64 package is `builds/x64/OpenTrack`, SHA-256
`1cfb63ec89994dfced7838bf3e2de13cbbdf120b04d5f0f25793f34eef2557d1`. The
updated splash helper exited successfully after 2.206 seconds in a bounded
launch check. The splash builder explicitly disables maximized/fullscreen and
uses a distinct Linux app ID. Floating-window levels are reasserted for several
frames after transitions. Focus, level-reassertion, and compact-splash builder
tests pass; live visual size and stacking review remain pending.

## Sample Editor piano key labels and MIDI pencil (2026-10-27)

The Pitch & Time keyboard now names every key, including black keys, using
sharp names with octaves (`C#4`). The keyboard widened from 38 to 64 px so a
black key's name sits inside the black key; labels hide as a set when semitone
rows are under 12 px. Pitch-note corner handles now draw and hit-test inside
their note at every row height. The existing MIDI Editor window gains pencil
semantics: the Draw tool click- or drag-draws notes with a live preview, while
the Select tool no longer adds notes from empty cells. Source and automated
checks pass and the x64 build was rebuilt; live visual review in OpenTrack
remains pending. Whether the MIDI Editor should become a lower-zone tab beside
Audio Editor instead of a separate window is an open user decision.

## Prior assignment — T054

## T054 — Realtime control performance, Stereo Out VST3 and startup splash (2026-10-27)

The user accepted the T053 group editing tools and drag behavior as good to go.
Waveform edge cases are deferred to a separate user review and do not block
editing-tool acceptance. T054 addresses audio static/slowdown during fader and
EQ manipulation, VST3 inserts on Stereo Out, and the supplied startup splash
reference (approximately 600×327 px, centered, with a two-second minimum
display while initialization proceeds). See
`docs/workflow/tasks/T054-realtime-controls-output-vst-and-splash.md`.
The user reports substantially improved fader/EQ response and successfully
loaded Ozone on Stereo Out. Live plug-in processing and persistence remain to
be checked.
Investigation found that Channel Settings changes rebuild the complete playback
plan; each plan reconstructs per-track channel processors, including EQ filter
state, and may crossfade/render both plans. VST scanning, instance mapping and
audio hosted inserts currently enumerate project tracks only, while Stereo Out
is a separate bus. Current work should avoid rebuilding the full playback path
for every continuous control drag and must extend hosted processing to the
existing output-bus identity without disturbing normal track/group inserts.

## Accepted prior assignment — T053

Group editing tools, including live ghost/candidate drag preview and stable
cursor, are user-accepted. Group waveform edge-case study is deferred to a
future user review. The rest of T053 history is retained below.

---

## T053 implementation and build history

## T053 — Group audio tracks (2026-10-27)

Implementation and build are complete; runtime/user review is pending in
`docs/workflow/tasks/T053-group-audio-tracks.md`.
The latest drag-preview follow-up preserves the existing composite styling,
shows the original group segment as a translucent ghost, renders the candidate
segment live from its dragged child clips, and keeps the standard arrow cursor
while hovering/dragging group segments. The updated x64 local-ASIO build is
`builds/x64/OpenTrack.exe`, SHA-256
`9DB7002CCEC6C07C8B75EBDEA46A9C9AC4EEE5D5ACF207E58C66A6E71A9797FB`.
Live drag and cursor behavior still need user review.
The user confirmed that drag-and-drop grouping nests the selected track. The
existing lane-button slot uses the supplied stack-and-G SVG to collapse/expand
group children; descendant M/S indicators reflect inherited group state. Group
creation offers mono/stereo configuration; moving a group carries descendants,
and placing a track between group members assigns it to that group. The Project
track-header area is one third wider.

Collapsed groups render a normalized mono or stereo composite. Group-row
editing now supports synchronized split/cut/delete/copy/paste/move/trim and the
shared fade/gain gestures, retaining child tracks and applying common timeline
offsets. Group tracks are now actual playback buses: children with automatic
routing feed their group, nested buses process before their parents, and the
group output follows its own channel processing to Stereo Out. Automatic
routing returns children to Stereo Out when removed; explicit non-default
routes survive group membership changes. Route displays resolve group names.
The lower-zone Sample Editor tab is renamed Audio Editor. The latest user
review clarified that a collapsed group is one continuous virtual event from
the earliest child start to latest child end. Only explicit group cuts and
boundaries shared by every populated child track divide it into selectable
pieces. Unshared child clip edges remain hidden; selecting/moving a piece
operates on child clips in that range. Collapsing no longer highlights child
recordings. Project meters use the track's mono/stereo channel count. VST
Channel Settings resolves group routes to names and offers valid group outputs.

`cargo fmt --all` and `cargo check -p opentrack` pass. The x64 local-ASIO
release build passes with existing dead-code warnings. Executable:
`builds/x64/OpenTrack.exe`, SHA-256
`899B47F3C873CDFA1A78DBBBB2E7DFF8B696AA96C3AEE0368E1EDC6E2BED1B59`;
`build-info.json` confirms `local_asio: true`. Runtime review of grouped edit
gestures, routing/audio, and the updated labels is pending user acceptance.

## Prior assignment — T052

Updated: 2026-10-27. Implementer: GPT-6 Luna. Implementation incomplete;
runtime and user review pending.

## T052 - Mixer meters, fader rail and numeric gain

Mono Mixer and Channel Settings tracks now draw one full-width meter lane;
stereo tracks retain independent L/R lanes and their separator. The shared
fader has a dark inset slot with shorter travel capped at 240 logical pixels.
Its thumb and gain mapping retain the existing range and fine-drag behavior.
Double-clicking the current dB readout opens an inline editor: Enter/focus loss
commits, Escape cancels, invalid input leaves gain unchanged, and -inf mutes.
Gain commits use the existing Track::gain and project history/persistence path.

Packet: `docs/workflow/tasks/T052-mixer-meter-fader-editing.md`. Ten targeted
checks, formatting and ledger validation pass. The x64 local-ASIO build is
staged at `builds/x64/OpenTrack.exe`, SHA-256
`7E7D3BF349F418ECD9528734CAD0FC58714655FF66B7570AD8923EFD498983D9`.
Live visual/audio review is pending.

## Prior assignment - T051

Updated: 2026-10-27. Implementer: GPT-6.1 Sol. User visual review pending.

## T051 - Button shapes and conventional letter controls

The latest README screenshots are the reference for the user's requested
button-shape review. Toolbar, track/Inspector letters, Mixer, Channel Settings,
Transport, panel close controls and standard egui buttons now share compact
rounded corners and restrained edges. The old Mixer bevel stripe is removed;
letter labels use fitted Segoe UI Semibold and the circular e label is centered.
Hover, pressed, disabled and keyboard focus treatments are shared. Existing hit
rectangles, control order, conventional letters and command behavior are retained.

Packet: `docs/workflow/tasks/T051-button-shape-polish.md`. Compile, formatting,
and 11 focused existing checks pass. The x64 local-ASIO build is staged at
`builds/x64/OpenTrack.exe`, SHA-256
`A19AA8B5A92AF419B9369F7D1FBAD2E5FBB6539953702329A1AF9412DF33A506`.
Live visual acceptance remains pending.

## Prior assignment - T050

Updated: 2026-10-26. Actual implementer: Codex. Independent review: pending.

## T050 - Tabler icon system and OpenTrack branding

The user approved continuing the toolbar prototype across the rest of the UI,
asked for slightly stronger icon contrast, and requested removal of the logo
beside File. The user supplied the new OpenTrack circle-and-dots mark, a
transparent GitHub banner, and dark-background splash artwork. The app icon and
README use the new SVG mark; the menu row has no extra logo. Startup displays
the dark splash artwork briefly and allows click-to-dismiss. The GitHub banner
is in `docs/images/opentrack-github-banner.svg` for the repository social image.
The native window/taskbar icon is rasterized at 256px for cleaner Windows
downsampling and uses a tighter SVG viewBox to make the mark fill more of the
icon. The transparent GitHub wordmark is displayed at 440px in a dark README
band. The README screenshots now show the supplied Project/VST and Sample
Editor/Pitch & Time views.

Tabler v3.49.0 (commit bbed884) is pinned and only required verified SVGs are
vendored, with MIT notice. The central renderer and resvg cache cover toolbar,
track controls, Mixer, Channel Settings, Project inspector, Sample Editor,
Transport, and split/draw/zoom cursor overlays. Existing layout, labels, DAW
letter controls, shortcuts, actions, colors and states are retained. Enabled
toolbar icons use near-black ink against light button faces; outline width is 1.75.

Current report: `docs/reviews/tabler-application-icon-inventory.md`, with the
full source inventory, mapping and screenshot limitation. The enlarged state
specimen is `docs/reviews/tabler-toolbar-specimen.svg`. Source validation and
the staged x64 local-ASIO build pass. Current executable: `builds/x64/OpenTrack.exe`,
x64 with local ASIO, SHA-256
`CA89FF580C3B9C9F7B2E04DE0AA0A699D02C2240C5136738F72D42CBF57B793B`. Live
desktop screenshot and cursor hotspot review await the user's screenshot
because this session cannot capture the Windows desktop. Audio and editing
behavior are unchanged.

## Prior assignment - T049

Updated: 2026-10-26. Actual implementer: OpenCode session (DeepSeek V4.1 Flash); no independent review.

## T044 continuation - native VST editor thread and live processing display

The installed Waves editor was reproduced as a white/stalled window on the main
UI thread, with controls that did not move. The cause was the native view sharing
the application's eframe/winit message and paint cadence. Native VST editors now
run on a dedicated STA thread with their own message pump, focus handling and
repaint requests. The view is created from the live audio effect's controller
(`DetachedEditor`) rather than a second plug-in instance, so the CLA-76 VU/GR
meters and processing displays follow the actual audio callback. Parameter edits
still use the effect's lock-free host queue, and track-wide insert bypass still
applies.

A standalone CLA-76 probe with a continuous test tone showed the meter moving and
the controls responding. The installed mono/stereo audio checks pass, as do the
hosted-audio and dedicated-editor regressions. The user reported that the
controls work in the application; final in-app meter and save/reopen review
remains pending.

Logo follow-up: the user-supplied reel mark is now stored as
`docs/images/opentrack-logo-source.png`. White and black variants are used in the
application header and README, and a matching taskbar icon is generated from the
same artwork.

Final candidate: x64 local-ASIO `84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`.
The full app suite has 133 passes, 8 ignored opt-in tests and six pre-existing
pitch-editing failures owned by the separate pitch track. VST3, scanner,
plug-in-catalog, hosted-audio and installed CLA-76 checks pass. The app was not
launched by this session after the final build.

## T049 - Project scissors, Sample Editor guide, Mixer selection, and VST review

The Project's lower-waveform cut zone was removed; users cut only after choosing
the Scissors tool. The Sample Editor keeps the scissor cursor at the real pointer
and draws its snapped guide through the ruler and waveform or Pitch & Time grid.
Clicking anywhere on an ordinary Mixer track strip selects the shared Project
track. Native VST editor windows now run on a dedicated STA thread, use the live
audio instance's controller, and report a matching processing failure while
passing dry audio through.

Source inspection confirms the hosted VST insert path is included in each track's
playback plan and reaches the configured output route. Installed CLA-76 mono and
stereo processing checks produce finite output with the track path intact. The
reported Waves blank editor, stalled input and dead meter were addressed by the
dedicated editor thread and live-controller view; a continuous-tone probe showed
the meter moving and controls responding. Final in-app meter and save/reopen
review remain pending. The supplied reel mark is now the white/black application
and repository logo.

The x64 local-ASIO release build completed successfully (SHA-256:
`84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`). The app
was not launched by this session after that build. Formatting and whitespace
checks pass; VST3, scanner, catalog and hosted-audio checks pass. Six pitch tests
remain failing in the separate in-progress pitch track. Runtime Project, Mixer,
final Waves meter display, z-order, and audible routing acceptance remain
pending.

## Product direction — feature-led open-source DAW and project interchange

OpenTrack development is organized around user workflows and concrete DAW features, with the running application as the primary design surface. User-provided screenshots are optional inputs for specific tasks and are not archived by default. Project interoperability remains a product capability: implement format- and version-specific adapters, map supported features, preserve unknown project data as unchanged opaque stubs where possible, and report unsupported content.

## Product direction — effects and plug-ins (2026-10-26)

The user clarified that new effects should be VST plug-ins rather than built-in OpenTrack processors. The native OpenTrack Compressor was removed from insert pickers and audio processing. Existing projects preserve its serialized insert as an inactive legacy slot labeled unavailable; users can replace it with an external VST3 effect or remove it. External VST3 compressors are unaffected. Future first-party effects and instruments should be designed with the user and built as separately loadable VST3 plug-ins. This direction does not change the active Project/transport review gate.

## T048 - Transport viewport focus and pinning

The user requested an OpenTrack icon on VST Audio Channel Settings, transport shortcuts in focused child windows, and a right-click Always on Top toggle for the compact Transport that keeps it above Channel Settings. T048 moves Transport into a separate borderless native viewport, routes child-window keys through egui, sets the app icon on native windows, and stores the pin and bounds in app preferences. The x64 local-ASIO release build passed (SHA-256 `A91EA6C01C8652A0C9AA1987C9109CE8D1CA1AC4D55957DF6397890989327461`). Visual, focus, stacking, and restart review remain pending; see [T048](tasks/T048-transport-window-branding-focus-and-pinning.md).

T047's pitch edge handles and Project take lanes remain pending the separate user review documented in [T047](tasks/T047-pitch-edge-handles-and-take-lanes.md). T042 likewise remains awaiting the user's listening review as documented below. Neither review is resolved by T048.

## Current review - T042 Batch 2

Batch 1 T041 was accepted by the user. After listening to the first T042 iteration, the user asked for pitch targets to blend toward adjacent notes inside the notes themselves, with a maximum reach of one sixteenth note, and reported bubbly/choppy sound when shortening notes.

The renderer now shares an event-wide source-pulse map and continuous output clock across note cuts. It reacquires real waveform peaks at voiced onsets, adapts overlap windows to source harmonic content and time expansion/compression, and uses the same tempo-aware pitch contour in playback and the Sample Editor. Edited and adjacent unedited notes ease toward a shared boundary pitch for up to one sixteenth-note on each side.

The 2026-10-25 audit found a hard wet-to-dry switch at the inner endpoint of an unedited neighbor's rendered edge. The fix adds a 5 ms endpoint crossfade, shortens the audio handoff to 5 ms while keeping the long pitch ramps, and makes one-unchanged-note pairs resolve to the unchanged edge pitch. Sample-continuity tests cover both orientations. Headless synthetic and read-only Vocal 01 measurements now pass; user listening is still pending.

This checkpoint also adds semitone-row note colors, track-color cohesion for meters/EQ/spectrum with blue default Stereo In/Out, and a dedicated note-drag audition voice that works while the transport is stopped without moving the playhead.

## Build and checks

- x64 release build with local ASIO succeeded; WASAPI remains available. Batch 3, VST3 and the native-editor thread work are included.
- Executable SHA-256: `84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`.
- App suite: 133 passed, 8 ignored opt-in tests, six pre-existing pitch-editing failures. VST3 suite, 6 hosted-audio checks, scanner/catalog tests and installed CLA-76 mono/stereo audio checks pass.
- Native CLA-76 editor test passes on the dedicated STA thread. A continuous-tone probe showed the meter moving and controls responding.
- Headless production-renderer suite: 6 passed; 1 explicit-source test ignored in the ordinary run. Read-only Vocal 01 matrix passed separately. Details and artifacts are in T042.
- OpenTrack was checked closed before the final build and was not launched afterward. Final desktop, device and user-listening review remain pending.

## User review checkpoint - pending

The user will review pitch quality with their own audio. The current build also includes note colors, track-color cohesion (including blue default Stereo In/Out and the Mixer color picker), note-drag audition, and Batch 3.

Batch 3 implementation is complete in source: toolbar Scissors cuts, snap-aware guides, Ctrl+wheel horizontal pan, rectangular Mixer fader caps, and shared Mixer/Project track selection. T049 removes the selected-track cut band, extends the Sample Editor cut guide through its edit area, and makes ordinary Mixer track-strip clicks select the same Project track. The x64 build succeeded; live interaction, Waves graphics, audio routing, native pin behavior, and visual review remain pending.

## VST3 lane — native editor and effect compatibility corrections

2026-10-26 user follow-up rejects generic sliders as the default plugin UI and
reports Waves insert crashes. Insert add/click now opens the plugin's native
editor. Fixed Waves mono bus coercion, factory release after module unload on
initialization failure, native editor close/replacement lifetime, frame sizing,
and iZotope's no-op processing notification. Native editor edits mark projects
modified. Fresh installed-folder sweep: 25/25 effects pass native editor lifecycle,
overlapping instances, processing and state restore, including 19/19 Waves
classes. Retrologue, Superior Drummer 3 and sforzando remain unsupported instruments;
this is not universal hosting or production sandboxing.

The later dedicated-thread editor work uses the live audio effect's controller so
controls and VU/GR meters follow processing. App suite: 133 passed, 8 opt-in
probes ignored, six pre-existing pitch failures. VST3/catalog/scanner and six
hosted-audio regressions pass. x64 local-ASIO release SHA-256:
`84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`. Final in-app
meter and release end-to-end acceptance remain pending. See
[T044](tasks/T044-vst3-native-editors-and-compatibility.md) for reproduction,
per-class sweep command, evidence and remaining limits. T043 remains historical.

## Cross-platform VST3 foundation — T045/T046

The scanner and insert picker share native VST3 bundle resolution for Windows,
macOS and Linux layouts. Scanner defaults include standard macOS/Linux user and
system folders, and the architecture precheck recognizes PE, ELF and Mach-O
images, including universal Mach-O slices. T046 adds required Linux
`ModuleEntry`/`ModuleExit` and macOS `bundleEntry`/`bundleExit` loading, explicit
Linux X11/Wayland and portal dependencies, and a hosted compile matrix for
Windows x64, Linux x64/ARM64 and macOS x64/ARM64. The matrix is prepared but has
not run. Locally, Windows x64 workspace all-target compilation passes, and the
scanner/catalog/VST3 test code compiles for Windows x86; no test binaries were
run. This workspace has no macOS/Linux Rust targets installed, so their code
still needs the hosted compile check. Native editor parent surfaces and event
loops, audio/MIDI device paths, and plug-in runtime validation remain open. See
[T045](tasks/T045-cross-platform-vst3-discovery.md) and
[T046](tasks/T046-cross-platform-vst3-runtime-foundation.md).

## Preserved direction

Keep pitch analysis/editing/processing in Rust, preserve original audio and the accepted Mixer, and maintain the gated sequence: Batch 1 accepted; Batch 2 listening review now; Batch 3 Project scissors/wheel behavior and Mixer fader/selection only after Batch 2 acceptance. Scope remains `transport-v16-pitch-time-editing-workflows`; implementation, feature verification, and user acceptance remain separate statuses. See [the historical feature report](metrics.md) and [T042](tasks/T042-vocal-quality-and-note-transitions.md).

2026-10-31 T062 filter-flavor and solo-range implementation: the EQ selector labels
Parametric I as Bell · Constant Q and Parametric II as Bell · Proportional Q;
Parametric II now tightens effective Q as boost/cut increases, with the same
coefficient model used by DSP and graph response. Band solo now has a subtle
full-height, Q-derived frequency window behind the spectrum and EQ curves.
`cargo fmt --all -- --check` and `cargo check --offline -p opentrack --features
local-asio` pass. No tests were run for this change. Runtime visual/audio review,
save/reopen, Windows behavior, and full playback-cost measurement remain open.

2026-10-31 T062 shelf-shape and selector-width follow-up: the EQ filter menu is
218 px wide and its labels fit on one line. Low and high shelves each expose
Classic, Smooth, Steep, and Resonant choices; the three added flavors have
separate Q-scaled responses, while existing LowShelfI-IV project values retain
their former response. Dynamic EQ supports all shelf choices. Formatting and
ASIO-enabled `cargo check` pass. Release build staged at `builds/x64/OpenTrack`
with local ASIO, SHA-256
`a20b1044f04a3aaf49602d253179ec1e3e3c15973de3c9741a8e7c7c982451bd`. User
visual/audio acceptance remains pending.

2026-10-31 Transport drag regression fix: the Wayland overlay now applies
pointer drag deltas to persisted Transport bounds and follows them with
`Window::current_pos`; a five-pixel top-edge grip gives the overlay a reliable
hit target. Native viewport dragging still uses `StartDrag`. The pure position
regression test passes. A UI drag harness did not complete in this environment,
so live interaction remains unverified. T062 focused checks pass: Transport drag
(1), EQ interaction (16), audio suite (104 passed, 3 ignored), and OPR
EQ/tonal-target round-trip (1). Fresh-target x64 Linux release with local ASIO
is staged at `builds/x64/OpenTrack`, SHA-256
`38ab551690f396ae0cc0c227e790e5d5d6c2ac95db18953148fca7c13ec792ac`. Live visual,
audio, save/reopen, Windows, and device-cost acceptance remains pending.

T062 acceptance refresh (2026-10-31): the OPR EQ/tonal-target round-trip now
includes the new smooth low-shelf and resonant high-shelf types. Analyzer suite
17/17, EQ interaction suite 16/16, audio suite 104 passed/3 ignored, and the
Transport drag-position regression 1/1. The release artifact remains
`builds/x64/OpenTrack` (local ASIO), SHA-256
`38ab551690f396ae0cc0c227e790e5d5d6c2ac95db18953148fca7c13ec792ac`. Live pointer,
visual, listening, in-app save/reopen and device-cost review are still pending;
the current desktop surface cannot expose OpenTrack for that review.

2026-10-31 EQ detail panel compacting: popup height reduced 176→144 px and
knob rows to 38 px with 16 px dials. Filter selector keeps its 18 px hit region;
power controls keep 16 px. The focused EQ interaction suite remains green at
16/16. ASIO-enabled release staged at `builds/x64/OpenTrack`, SHA-256
`bf67454f358f720cfada241ed8e729414f2fb35bc8eadec97140f3c4c05f1128`.

Plasma appearance diagnosis: `theme::install` follows only egui's System
light/dark preference, then applies OpenTrack/egui palettes. Channel Settings
and Transport popup menus use custom egui styles and do not read KDE color
scheme or accent settings. Wayland renders these utility panels as egui
in-app overlays, not Qt widgets. There is no KDE color-scheme/portal bridge or
live Plasma theme subscription. A Plasma appearance bridge remains separate
work; window decorations/compositor effects are distinct from widget styling.

EQ layout correction (2026-10-31, GPT-6.1 Sol): user rejected the prior sizing. Source now uses 48 px dials, 88 px control rows with 10 px labels, separated filter and dynamic section positions, and a 384 by 280 px popup. Artwork uses foreground ink for legibility. Local-ASIO cargo check, formatting and diff check pass. Release package still contains the previous layout; new release and live visual review pending. User acceptance remains open.

- EQ selected-band editor layout (2026-10-31, GPT-6.1 Luna): inspected `reference/Screenshot_20261031_110807.png` for its vertical grouping, compact hardware labels and status-light style. Channel Settings now reserves a fixed 330 px right-hand editor column, with EQ graph interaction and rendering confined to the left area; the viewport opens at 1230 px (1120 px minimum) to retain graph width. The selected band editor remains fixed while selection changes, preserves existing controls/knob SVG/accent lighting, and disabled bands can be re-enabled. Tonal view keeps its full-width analysis graph without an editor. Focused sidebar, graph hit-testing, and band strip tests pass; formatting/diff checks and local-ASIO check pass. Linux x64 local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `4758025cade1f03f5dd7fa2ef42e6ebdd39d6f2815a1c312a134127773207266`. Runtime visual/audio review remains pending.

- EQ sidebar visual correction from user review (2026-10-31, GPT-6.1 Luna): changed the selected-band editor to a vertically stacked control list beside the graph, with a taller Channel Settings viewport; retained the existing knob SVG, numeric entry, filter menu, power and solo controls. Parameter LEDs now fill all 11 ring dots across the full knob travel; Threshold dots show the selected channel live peak. Removed the SVG center pointer that caused the ghost line; double-clicking a dial resets it to the parameter default (Gain/Range 0, Frequency 1 kHz, Q 0.707, Threshold -24 dB, Attack 10 ms, Release 100 ms). Focused EQ tests (7) pass, as do local-ASIO check/release build, formatting and diff checks. Linux x64 release is staged at `builds/x64/OpenTrack`, SHA-256 `22a1b84d4e0f496c21d291966ae55b84181da0f9ea6e9b12eb305a89adf30b41`. Live visual/audio review remains pending.

- User visual follow-up (2026-10-31, GPT-6.1 Luna): retained the right-side EQ panel but made its controls economical: three Static EQ knobs share one compact row, the filter selector sits immediately below, and four Dynamic EQ knobs share a second row; the viewport returns to 1230×430 (minimum 1120×420). Retains 48 px user knob artwork, full 11-light sweep, threshold live meter, no fixed center marker, and double-click parameter reset. Seven focused EQ tests, local-ASIO cargo check and release build, format and diff checks pass. Linux x64 release staged at `builds/x64/OpenTrack`, SHA-256 `4d4238895763b436d5880c9a93c2b55a01f706f3b92b47ef244cfc9f52a57fed`. Visual acceptance in the running app remains pending.

- Input gain and Dynamic EQ threshold LEDs (2026-10-31, DeepSeek V4.1 Flash): track and group channels now apply `input_gain_db` and `input_phase_inverted` ahead of the built-in EQ through a smoothed engine gain stage, with a live `AudioEngine::update_track_input_gain` path from Channel Settings and Mixer. The trim previously reached device buses only, so both knobs and their peak lights were inert for tracks. `StereoMeter` now publishes a pre-fader input-stage peak, and the input-gain ring reads that instead of the post-fader meter, so it reports the level at the trim and still moves with the fader down. The Dynamic EQ THRESH ring now maps pre-EQ RMS over the channel meters' 60 dB window instead of the knob's -90..0 dB threshold range, which had pinned it at 9-11 of 11 dots for ordinary material. `cargo test -p opentrack-audio` passes 114 with 3 ignored; `cargo test -p opentrack --bin opentrack` passes 188 with the same 14 pre-existing `ruler_interaction_tests`/`sample_editor` failures reproduced without these edits. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `211aa78b7cd185b600fa3a4232e5a3a9cbc8cb61248a1e6b5244a88cecfdcb29`. Live listening, LED observation and desktop review remain pending.

- Channel Settings density and popup dismissal (2026-10-31, DeepSeek V4.1 Flash): the EQ selected-band column narrows from 168 to 150 px, so each numeric value field loses 18 px and the graph regains it; Inserts and Sends narrow from 154 to 146 px. Insert names and send labels now share the selector chevron's centre line, with the chevron placed at the slot body's centre instead of a fixed 25 px offset. Menu and context-popup hover dismissal now anchors on the whole control that opened the popup instead of a 24 px square at the click point; leaving both that control and the popup still closes them, and clicks inside a nested popup keep the original anchor. `cargo test -p opentrack --bin opentrack` passes 190 with the same 14 pre-existing ruler/sample-editor failures, including 4 popup checks and new layout checks. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `c4758b9a32052028c675ecaa48db063fd3cb6026b0e2ece084c662f2192ebedb`. Visual acceptance in the running app remains pending.

- Channel Settings rack and plug-in editor access (2026-11-01, DeepSeek V4.1 Flash): the window retargets in place when the FX button of another channel is used; a new native window is only created when none is open. Insert and send slots are single-line 23 px rows on a 25 px pitch, with power, optional view and list chevron sharing the text centre line and the slot number at the row's right edge. The plug-in list opens from both the name area and the chevron, with hover feedback; a dedicated eye button beside the power button opens, closes and reopens that insert's VST3 editor, is lit while the editor is up, and appears only when the plug-in reports an editor. `cargo test -p opentrack --bin opentrack` passes 192 with the same 14 pre-existing ruler/sample-editor failures, including new retarget, rack geometry and eye-toggle checks. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `e75b66a39c4180e0bb77843cc6608dc3eb4634a68e8ccd0dbbc27ade84396650`. Visual acceptance in the running app remains pending.

- Rack rows and ruler playback toggle (2026-11-01, DeepSeek V4.1 Flash): the chevron and the plug-in name are now one button, plug-in names are right-anchored, and each rack starts with one row plus a "+ Add Insert"/"+ Add Send" row that opens more rows up to 24. Double-clicking the timeline ruler in the Project window, MIDI editor or Sample Editor toggles play/stop through the existing transport path; only the ruler block counts. Double-clicking a plug-in name/chevron no longer opens the editor; the eye does. `cargo test -p opentrack --bin opentrack` passes 194 with the same 14 pre-existing ruler/sample-editor failures. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `58391faefad85ca618267fa022a27e05cb3e3472d2b2bffb4cd50f2024db9f8e`. Visual and transport behaviour review in the running app remains pending.

- Insert-row power, bypass and removal (2026-11-01, DeepSeek V4.1 Flash): slot numbers now lead each insert/send row, followed by the power, bypass and (for VST3) eye buttons; empty slots hide the power, bypass and eye controls, while a loaded VST3 row always shows the eye (lit when its editor is open, slashed when closed). Right-clicking a row removes it; for inserts the live plug-in state is captured first, the editor closes and later slots shift down. `InsertEffect` gained a persisted `bypassed` flag (serde default, so older projects load unchanged): bypass keeps a plug-in loaded but out of the chain and out of latency compensation, while powering off now unloads the VST3 instance after capturing its state, so a powered-off plug-in costs no CPU. `cargo test -p opentrack --bin opentrack` passes 195 with the same 14 pre-existing ruler/sample-editor failures; `cargo test -p opentrack-audio` passes 115 with 3 ignored. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `a8b13d14a7afdbf3cb8b6ba85b6d2eec066a123fbf4a683f446fda528481a2c4`. Visual and CPU/runtime behaviour review in the running app remains pending.

- VST3 load stalls and tonal comparison (2026-11-01, DeepSeek V4.1 Flash): user reported freezes when loading TDR Nova and Tonal Balance Control 2 settings. Instantiating a VST3 no longer creates an editor view just to test for one (that eager `createView` is the likely stall for these plug-ins): the editor-support flag is now lazy and set when an editor actually opens. `Runtime::sync` and `load_insert` emit stage timings (module, instantiation, state-restore size/time, total sync) when `OPENTRACK_VST3_TRACE=1`, so any remaining stall can be attributed to module load, component creation or `setState`. An ignored smoke-test helper can still probe editor support explicitly. The user's separate observation that Tonal Balance Control 2's frequency display differs from OpenTrack's tonal view is captured in T068 for a measured comparison before changing our tonal representation. See T069 for the trace procedure and T068 for the comparison questions.

