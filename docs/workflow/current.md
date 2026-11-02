# Current assignment — roadmap activation (2026-11-01)

## Paused checkpoint — 2026-11-01

The active 32-step roadmap goal is paused at user request. Current `main` is `2271946` (`Reveal stored automation from arrange menus`), 17 commits ahead of the cached `origin/main` ref. The last push attempt failed because this machine could not connect to `github.com:443`; no remote update was confirmed. The Kubuntu CPU/memory-meter update has not appeared in this checkout. Preserve the untracked `recovery-20261028-204244/` and `stress-bench/` folders.

Since the earlier checkpoint, local commits added atomic Split at Cursor, unified Crossfade, Move to Cursor with undo, disabled and documented unavailable automation Write controls, constrained Read controls to supported Audio/Instrument tracks, and enabled arrange context-menu discovery for stored volume curves. Focused tests and `cargo check -p opentrack --offline` pass for these latest slices; `git diff --check` passes. Runtime/audio review remains distinct and unaccepted.

### Resume sequence

1. Recheck Git/remote state and preserve any newly arrived Kubuntu meter work; push the existing local commits when GitHub is reachable.
2. Resume T078 with cancellation semantics for automation point drags. The contract requires cancel to restore the original curve and keep undo atomic. The starting code is `crates/opentrack-app/src/automation.rs::draw_lane`; inspect its temporary drag index and frame-history integration before changing it. Then finish the open T078 gates: fader-follow/manual override, write capture through a bounded nonblocking handoff, loop-boundary handling, and desktop/audio review. Do not re-enable Write controls until capture exists.
3. Verify read-all/context-menu visibility, track-volume playback and undo/save/reopen in the application when desktop review is available; separately investigate the known baseline arrange snap failure (beat 3.25 expected vs 3.5 actual).
4. Continue roadmap steps 21–32: mixer integration, physical routing, recovery, VST3 reliability, remaining editing operations, selection consistency, a real preference consumer, CPR fixture-based interchange, and final architecture/inventory reconciliation. The full roadmap is not complete.

## T078 - Automation contract

Track-volume automation points have a bounded persistent model and sample-accurate read playback in both engine paths and offline bounce. The arrange lane draws curves, inserts snapped points, supports left-drag movement and right-click removal using validated core commands, and clamps movement between neighboring points. Project history wraps gestures; command-level tests and the app check pass. Write capture, gesture cancellation, fader-follow/manual override and desktop playback review remain open. The existing expanded-lane snap test fails at beat 3.25 vs 3.5, reproduced at baseline `8b90b8a`. See [T078](tasks/T078-volume-automation.md).

Write controls in Mixer, Channel Settings, Project Inspector, the arrange lane and global Mixer are now disabled and say automation writing is unavailable. This preserves the stored flag for compatibility while preventing the UI from suggesting that fader recording is active. Read remains available only for supported non-group track-volume playback; group and device-bus automation is unsupported.

Arrange background and per-track context menus now reveal stored supported volume curves. Empty or invalid lanes and Group/MIDI tracks are excluded. The matching “Hide All Automation” action remains available when any lane is visible.

## Shared Split at Cursor command

Edit > Split at Cursor and the existing `edit.split_at_cursor` shortcut now share one operation for selected ordinary audio clips. The selection is atomic: if any selected clip cannot split at the cursor, no clip changes. Project undo/redo applies. Group-segment and loop splitting remain unavailable. Three focused app helper tests cover multi-clip split, cursor miss, and rollback; runtime menu/shortcut review remains pending.

## Shared Crossfade command

Edit > Crossfade, `audio.crossfade` (X), and the arrange X gesture now share one operation. It requires two overlapping audio clips on the same track and leaves the project unchanged otherwise. Two focused app tests cover success and rejection; runtime listening remains pending.

## Move to Cursor

Edit > Move to Cursor and `edit.move_to_cursor` (Ctrl+L) align the earliest selected ordinary audio clip to the snapped transport cursor while preserving the selection's relative offsets. The move is applied atomically, recorded in project undo history, and rebuilds the playback plan. Two focused helper tests cover offset preservation and stale-selection rejection; runtime interaction review remains pending.

## Routing boundary

The application routing module now owns send-to-playback resolution, group-cycle checks used by routing menus, and the shared active-send predicate. Core project validation and the audio engine retain their existing responsibilities; routing UI and broader typed-bus commands remain for later work. Local-ASIO check and focused Channel Settings (28), mixer (11), and preset/history (4) checks passed. The running EQ review build remains untouched.

## T076 - Send model foundation

Track and group sends now route in both engine paths and offline bounce, with pre/post tap, gain, pan, power, bypass, group selection, bounded compensation and cycle validation. Presets, mixer paste and slot removal preserve settings. Core, audio, focused UI and synthetic latency tests pass. A full app run has 14 timeline/pitch-editor failures that reproduce identically at baseline cae80fc. Device-bus sends, live gestures, hardware listening, VST compatibility and real-time performance remain open. See [T076](tasks/T076-send-routing-model.md).

## EQ strip layout update

Follow-up from the user build review: the dynamic section power switch occupies the empty left column beside Threshold, centered above Range. The editor is 130 px wide with equal outer knob margins, increased vertical stagger spacing, and a visible stepped divider. GPT-6.1 Sol inspected an offscreen raster from the actual egui shapes and textures; 25 focused Channel Settings checks and the temporary render harness passed. Temporary capture code was removed. The existing release executable predates these corrections; live gesture/visual acceptance remains pending.

Further polish: steepened the curved section step to follow the SSL reference more closely, clarified 9 px section headings and 8.5 px knob labels, and reserved filter text clearance before its arrow. Actual egui render inspected; 25 focused checks pass. The actual UI was rasterized and inspected again; all 25 focused checks pass. Release build remains pending.

The divider now rises steeply in the clear space between the static controls, and the dynamic controls sit higher with the power switch centered above Range. Actual egui shapes and textures were rendered offscreen for inspection; 25 Channel Settings checks passed. Release build and live gesture review remain pending.

The matching 16 px power switches are centered below their section headings; the static switch is centered above FREQ and the dynamic switch above RANGE. The divider right end sits slightly below the midpoint between FREQ and THRESH, with its left end unchanged. Both power buttons are centered vertically between their headings and knob columns. The 25 focused Channel Settings checks passed; live review remains pending.

The selected-band editor uses a narrow staggered strip with the filter at the top, labels under the existing knob artwork, a stepped section divider, and a compact value editor anchored beside the hovered knob. Static and dynamic DSP and project data are unchanged. Implementer/reviewer: GPT-6.1 Terra. The 25 focused Channel Settings tests pass; interactive hover editing and visual review in the application remain pending. The user requested a standalone design preview and no application launch during this work.

User confirmed T071 CPU/memory meters complete and authorized execution toward the 32-step feature and architecture roadmap in `docs/workflow/roadmap-32-steps.md`. T071 is recorded user-accepted; no independent runtime observation is claimed. Source audit registers now cover main/context menus, all 152 keyboard command IDs, windows/dialogs, 30 combo boxes and custom selector families; evidence and coverage limits are linked from `docs/reviews/window-selector-coverage.md`. T072 wires MIDI editor Snap to a persisted app preference and positional quantization. MIDI preferences and quantization policy now live in `midi_editor_settings.rs`; three unit tests and the existing editor recovery render test pass. Runtime gesture/persistence review remains pending because native desktop control is unavailable in this session. The user verified Kubuntu Linux support; Windows x64 remains the primary target. The README reflects these strengths, the known send limitation, and the highest-value feature gaps. OpenTrack's original workspace code is licensed Apache-2.0; third-party licenses remain separate. Preserve existing working-tree work and record automated/runtime results separately.

Next roadmap workflows are tracked in `docs/workflow/roadmap-32-steps.md`. T074 audio mixdown/export has an end-to-end source implementation and awaits runtime acceptance; its contract and evidence are in `docs/workflow/tasks/T074-audio-mixdown-export.md`.

T074 now has an end-to-end source implementation: the export menu opens a beat-range/rate/encoding dialog, and a worker renders through an isolated graph and VST3 runtime to an atomically replaced WAV. Export uses the latest prepared immutable graph snapshot without rebuilding the live device plan. Targeted offline-render (3) and WAV (3) tests pass, and native plus Windows x64 local-ASIO compile checks pass. Runtime interaction, active-playback parity and listening remain pending.

T075 adds bounded format-0/1 PPQ MIDI import and format-1 export in `opentrack-core`, with file actions and background I/O in `midi_file.rs`. Eight focused tests, including format-1 import and an independent export decoder, pass; the complete core suite passes (31/31). The Windows x64 local-ASIO release build succeeds. Independent DAW files, undo/redo and in-app validation remain open. Kubuntu support was part of development and was user-verified; Windows x64 remains primary.

The compact two-column built-in EQ layout is pushed to `main` at `1f10fb1`. A Windows x64 local-ASIO release package built from the current worktree is available at `builds/x64/OpenTrack.exe` (SHA-256 `51B23CE5495A7496E599DFEDAF94F604149638FF80FB67ECBA4AC348D8A3F22E`). The build includes current in-progress T074/T075 source; their runtime acceptance is still open.

# Historical assignment — T065

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

- Tonal weighting, target bands and smoothing (2026-11-01, DeepSeek V4.1 Flash): after comparing with Tonal Balance Control 2, the tonal view draws one smooth target envelope of +/-3 dB that follows the target curve, with Low/Low-Mid/High-Mid/High separators across it, smooths tonal curves over roughly a third of an octave instead of a quarter, and applies the existing 0/3/6 dB-per-octave display tilt to the tonal curves, pivoted at 1 kHz. The user kept OpenTrack's frequency span, line/target style and sizing; analysis resolution and persisted targets are unchanged, and the measured-vs-target difference view is unaffected because the same weighting cancels on both sides. New checks cover the 1 kHz pivot, tilt scaling and band mapping, and a tonal `Ref` selector now chooses what 0 dB means (whole-spectrum mean, 200 Hz-2 kHz average, or the 500 Hz band). `cargo test -p opentrack --bin opentrack` passes 199 with the same 14 pre-existing ruler/sample-editor failures. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `eb640670137eba2078d84b2901f3f9aae62457eb90d7c2ffc38c2cfed5534561`. Running-app visual review of the bands, smoothing and weighting remains pending. See T068.

- TBC2 options crash diagnostics (2026-11-01, DeepSeek V4.1 Flash): user reported that opening Options in Tonal Balance Control 2 crashes OpenTrack. `IPlugFrame::resizeView` now resizes the host editor window only when the requesting view is the main editor view; popup and sub-views still receive their own `onSize` without touching the host window. Native faults that never reach the Rust panic hook are now written to `%LOCALAPPDATA%\OpenTrack-\logs\exception.log` (exception code, faulting module, raw frames) through a Windows unhandled-exception filter; panics keep their existing `panic.log`. That log is the evidence needed to confirm whether the fault is in the plug-in DLL or in OpenTrack. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `296caee0d0225e13257fef3f0fa6907516d6121429431f0f3ac5e3644912b7bc`. See T070.

- TBC2 options hang follow-up (2026-11-01, DeepSeek V4.1 Flash): Windows recorded the reported failure as an Application Hang (event 1002) with no exception or panic log, so it is a stall rather than an access violation. OpenTrack now writes a per-launch `%LOCALAPPDATA%\OpenTrack-\logs\session.log` with a UI heartbeat (`ui alive` every five seconds, `ui stalled` every second once frames stop) and VST3 editor-thread events (start, open, each command, close, and an `alive` tick about every two seconds), which identifies which thread stopped on the next hang. Editor close additionally waits only two seconds for the editor thread and then detaches it, so a plug-in stuck in its own modal loop cannot freeze the application. See T070 for the capture procedure. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `d03ddae568b149012c0fef9ebc8c2248d5f5364a25b4d864dfe9d7f1cd569114`.

- TBC2 options deadlock fix (2026-11-01, DeepSeek V4.1 Flash): the session trace captured the hang at heartbeat 377 with the editor thread silent - the editor was inside Tonal Balance Control 2's modal Options loop while the main UI thread was calling `IEditController::setParamNormalized` every ~16 ms through `sync_output_parameters`. The plug-in holds its controller lock during the modal loop, so the main thread blocked permanently. Processor parameter changes now sync on the editor thread that owns the view; the main thread no longer touches the controller while an editor is open, and `capture_states` skips inserts with an attached editor (their state is captured when the editor closes). Editor close waits at most two seconds and detaches a stuck thread. Heartbeat and editor-event tracing remain in place. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `20cf6ceefa60050abeb684b978c5f29e1bf85bcd6d38e3dc8a9e722c3c3b80f4`. See T070.

- TBC2 hang stage tracing (2026-11-01, DeepSeek V4.1 Flash): the controller-thread fix did not stop the freeze; a second session shows the UI stalling about four seconds after the editor opens while the editor thread's heartbeat also stops. The hang logger now records the UI section when frames stop (`workspace`, `dialogs`, `vst-windows`, `vst-timers`, `vst-bounds`, `vst-sync`), so the next reproduction identifies the blocking call instead of only the thread. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `52a8337ac3ead379d6277053aabde6fbd198287bc05bfa4b5c6d50b079831613`. See T070.

- TBC2 hang root cause fixed (2026-11-01, DeepSeek V4.1 Flash): the stage trace named `vst-timers`. The main thread was dispatching plug-in thread timers (`pump_ui_thread_timers`) while the editor thread was inside Tonal Balance Control 2's modal Options loop; dispatching a timer created during component initialization re-entered the plug-in on the UI thread and deadlocked against the modal loop. OpenTrack no longer dispatches plug-in thread timers from the UI thread - the editor thread pumps its own messages - and `pump_ui_thread_timers` was removed. iPlug2-style processors that push meter data through an idle timer may no longer update their editor from the host thread; that is the safe trade and can be revisited with a non-reentrant mechanism.

- Insert/send slot layout (2026-11-01, DeepSeek V4.1 Flash): slots are now twice as tall (46 px, 48 px pitch) with the slot number and power/bypass/eye buttons on a top control row and the chevron plus plug-in name on a full-width lower row, so names such as Tonal Balance Control 2 fit. The rack still starts with one row, grows from the `+` row to 24 or the panel capacity, and right-click removes a row. Off-state power/bypass icons now use the palette's muted colour instead of a fill tone, so a switched-off button remains visible while reading as disabled. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `a34efa93c839eaa587d8f9917f44397ec7ce0511a200e520f1ce09ad189c7944`.

- Transport CPU and memory meters (2026-11-01, DeepSeek V4.1 Flash): the transport panel reserves a 132 px left block and shows two segmented gauges with visible "CPU" and "RAM" labels to the left of the input meters. CPU is system load (`GetSystemTimes`) with OpenTrack's process share in the tooltip; RAM is system memory load with the used amount beside it and the OpenTrack working set in the tooltip. Sampling runs every 500 ms on the control thread; non-Windows builds compile with no samples, and workspaces saved at the old transport width are widened on restore. Local-ASIO release staged at `builds/x64/OpenTrack`, SHA-256 `6295bf07cf60341e30a6ee9e624b6852fc82eec2d3e544cf8887f73a3845bf50`. See T071.

- Automation drag cancellation (2026-11-01, DeepSeek V4.1 Flash): Escape during an automation point drag restores the curve captured when the gesture began and creates no undo entry. `ProjectHistory::begin` now stores the dirty flag with its snapshot and `cancel_pending` restores both; the lane returns `LaneAction::CancelDrag` and clears its drag index. `cargo test -p opentrack` passes 240 with 15 failures; the same 15 reproduce at the pre-change tree, including the newly failing `shift_selection_and_x_apply_real_overlap_fades` from the crossfade unification. Linux x64 release staged at `builds/x64/OpenTrack`, SHA-256 `64d8645ae6cf8800e37929d94b428e91ddd5510bac8d820fec364e1098b7bb4a`. Live gesture review remains pending.

- Automation fader follow (2026-11-01, DeepSeek V4.1 Flash): with Read enabled and the transport playing, Mixer strips and the Channel Settings fader show the curve value at the current transport position; grabbing a fader switches to the stored base gain for the gesture and automation resumes on release. The follow value is read at control rate from the same lane the engine evaluates, and following never writes the base gain. `cargo test -p opentrack` passes 241 with the same 15 baseline failures. Linux x64 release staged at `builds/x64/OpenTrack`, SHA-256 `14011be5a46ed5993a7dffde8618cf9634ce1cc41596af3f96583dfa3d7493ff`. Live fader review remains pending.

- User hardware verification (2026-11-01, reported by the user): track creation, recording and VST insert/load work on real hardware, tested with a Mackie interface and the laptop's built-in devices on Kubuntu. Recorded as user-reported runtime evidence for the ordinary device paths in roadmap steps 22-24; it does not cover incomplete-take recovery, general physical channel maps, VST3 failure isolation or plug-in editor limits.

- Automation write capture (2026-11-01, DeepSeek V4.1 Flash): Write is enabled for Audio/Instrument tracks across the Mixer strip and sidebar, Channel Settings, Project Inspector and the arrange lane. With Write on and the transport playing, fader movement is captured on the control thread into a bounded 4096-sample gesture; a loop crossing starts a separate interval. On release the new `replace_volume_points_in_range` command replaces only each interval's interior and adds edge points, keeping material outside untouched, as one undo entry. Over-budget or rejected writes restore the pre-gesture project and report. `cargo test -p opentrack` passes 244 with the same 15 baseline failures; `opentrack-core` 46 and `opentrack-audio` 141 pass. Linux x64 release staged at `builds/x64/OpenTrack`, SHA-256 `9159c460d283010ad4edfbb38d180c428208a3a046b487e9d156f62a5c72aaf3`. Live write/fader review remains pending.

- Incomplete-take recovery (2026-11-01, DeepSeek V4.1 Flash): `AudioRecorder` writes samples straight to disk but only finalizes the WAV header at the end, so a kill or power loss leaves audio that reads as empty. Recording now writes `Metadata/Recording/in-progress.json` before the recorder creates any file and settles each entry only once its take is in the project; a take that fails to place stays journaled. On the next open of that project a "Recover Interrupted Recording" prompt lists the unfinished takes with their track, beat and size. Restore calls `opentrack_audio::repair_take`, which rebuilds the RIFF and `data` lengths from the real file length, drops a partial trailing frame, and is a no-op on a finalized file; the take is then placed at its recorded beat on its original track (or the first audio track, or a new "Recovered Take" track) as one undo step. Discard moves the file to `Metadata/Recording/Discarded` with collision-safe names and keeps it. Tests: 5 journal tests in `recording_recovery`, 4 header-repair tests in `opentrack-audio`, and 5 app tests that build a real crashed WAV, prove it is unreadable, restore it, and render the prompt. `cargo test -p opentrack` passes 254 with the same 15 baseline failures; `opentrack-core` 46 and `opentrack-audio` 145 pass.

- Project recovery autosave (2026-11-01, DeepSeek V4.1 Flash): while a project has unsaved changes, OpenTrack writes a validated copy to `Metadata/Recovery/autosave.opr` at most every 20 seconds (`AUTOSAVE_INTERVAL`), using the same atomic, validated save path as a normal save. It never runs during recording, while a project load is in progress, or for an unmanaged/legacy project without a folder. A completed save, an explicit discard of unsaved changes, and choosing to discard the recovery all remove it. When that project is opened again, "Recover Unsaved Changes" shows when the copy was written and either replaces the opened project with it (one undo step, kept until the next save) or removes it. This is also the answer to the window close eframe will not let us prompt on. New module `project_recovery.rs` (paths, atomic staging, age description) with 3 tests, plus 6 app tests including a full open-project path that detects the copy and recovers the missing track. `cargo test -p opentrack` passes 263 with the same 15 baseline failures; `opentrack-core` 46 and `opentrack-audio` 145 pass.

- Edit-family completion (2026-11-01, DeepSeek V4.1 Flash): audio events gained a persisted `muted` flag (`AudioClip.muted`, serde default so older projects load unmuted). A muted event keeps its position, trim, fades and gain but is filtered out of the playback plan by `audio_events::clip_is_audible` and draws dimmed with an `M` marker. New core commands `duplicate_audio_clips` (copies after the selection span, preserving source/trim/fades/gain/mute) and `set_audio_clips_muted`. New Edit commands, each one operation shared by the menu and the reference keymap: Duplicate (Ctrl+D), Mute Events (Shift+M), Unmute Events (Shift+U), Mute/Unmute Objects (Alt+M), Select All Events (Ctrl+A, suppressed while a text field has focus), Select None (Ctrl+Shift+A), Range > Select in Loop, and Range > Set Left/Right Locator from the selection. Split at Cursor now splits a selected group segment through `split_group_audio_clips` instead of refusing. Tests: 5 new core tests (duplicate single/selection/no-op, mute reversibility, muted round trip and legacy default) and 4 new app tests (audible predicate, duplicate/mute/select actions, range selection and locators, group-segment split). `cargo test -p opentrack` passes 267 with the same 15 baseline failures; `opentrack-core` 51 and `opentrack-audio` 145 pass.

- Test harness debt (2026-11-01, DeepSeek V4.1 Flash): the 15 baseline failures came from tests that hardcoded the Windows pixel scale of the arrange view (`(1200-198-34)/49` px per beat, fixed click coordinates, fixed control offsets). The ruler fixture now measures the scale from the bar numbers the ruler itself draws (`pixels_per_beat`, `x_for_beat`, `beat_at`, `y_for_track`, `click_row_control`) and the affected tests use those helpers. Fixed: the automation-lane drop rejection, the full-project drag, selected grid controls, the corner-handle loop/skip resize, the hand-drag rigid move, the group-drop collapse control, and both ruler seek/zoom anchor tests. The crossfade test now asserts the crossfade *request* and applies it through `crossfade_selection`, which is the operation the app runs, instead of depending on the frame's dirty flag. `cargo test -p opentrack` is now 277 passed with 6 failures, down from 244 passed with 15 failures at the start of the session; the 6 remaining are note-editor (Sample Editor pitch editing) tests that still assume a fixed note height and drag scale and are the next harness target.

- Add Track instrument selection (2026-11-01, DeepSeek V4.1 Flash): Add Instrument Track no longer says "Instrument unavailable". The dialog lists the scanned VST3 instruments (category Instrument) with "No Instrument" first, and `load_dialog_instrument` inserts the chosen plug-in as the new track's first insert, so it persists and reloads with the project like any other insert. An empty catalog tells the user to scan plug-ins. Three new tests cover the insert it creates, the no-selection/out-of-range cases, and that an audio track never receives it. Preset browsing and speaker layouts remain labeled unavailable. `cargo test -p opentrack` passes 280 with the 6 remaining note-editor harness failures; `opentrack-core` 51 and `opentrack-audio` 145 pass.

- Routing model (2026-11-01, DeepSeek V4.1 Flash): new `opentrack_core::routing` is the single vocabulary for channel routing. `InputRoute` (No Bus, Stereo In, mono input channel N) and `OutputRoute` (Stereo Out, Left only, Right only, No Bus, Group) parse the persisted strings without migrating them (`Left - Stereo In` is channel 1, `Right - Stereo In` is channel 2), expose labels that name the physical channel, and drive `output_destinations`/`input_destinations`, so the Channel Settings routing menus and the Add Track input picker can only offer routes the engine can play. `MAX_INPUT_CHANNELS` is 2 because the capture bus is stereo; the model can express wider channels and validation rejects them explicitly. The app's send rack, group-cycle check, live-monitor tap and playback output route now resolve through the model, and an unresolvable route becomes `Disconnected` instead of silently joining the stereo master. `Project::validate_routing` reports unknown routes, dangling/self group targets, feedback loops and over-wide mono inputs; `finish_project_load` appends the problem to the status message, and `ProjectFile::write` refuses to save one (reading stays permissive so an affected project still opens and can be re-routed). Tests: 6 in `routing` (persisted round trip, unknown routes, channel-width validation, speaker assignment, destination lists, labels), 4 in `project` (accepted routes, missing group, unknown route and narrow input, self route and cycle), and 3 in the app (plan never falls back to the master, monitor tap per channel, broken route reported on open). `cargo test -p opentrack` passes 283 with the 6 remaining note-editor harness failures; `opentrack-core` 61 and `opentrack-audio` 145 pass.

- Sample Editor note interaction fix and a green suite (2026-11-01, DeepSeek V4.1 Flash): the pitch note's anchor handle was a fixed 7 px strip on the note's top edge, but a note row is only ~12 px tall, so the strip covered the note's vertical centre and every body click became an anchor move. That single bug caused five of the six remaining test failures: body drags, double-click resets, multi-select drags and shift-during-drag all landed on the anchor. The strip is now `min(7 px, 40% of the note height)`, leaving the middle free for the body while the top edge still grabs the anchor; `the_pitch_anchor_strip_never_covers_a_short_notes_middle` pins it at 12/16/24/60 px. The last failure assumed the editor's pixel layout: that test now reads the drawn note rectangle from the rendered shapes (as the other Sample Editor tests already did) instead of hardcoding `(301,123)-(1180,478)`, so the click and the semitone drag follow the real header height. `cargo test -p opentrack` is now 289 passed, 0 failed, 9 ignored (from 244 passed / 15 failed at the start of the session); `opentrack-core` 61 and `opentrack-audio` 145 pass.

- Step 32 consolidation (2026-11-01, DeepSeek V4.1 Flash): `docs/workflow/consolidation-2026-11-01.md` reconciles all 32 roadmap steps with their evidence, measures the UI inventory (68 wired menu commands, 176 declared-but-disabled entries, 52 of 152 keymap ids dispatched) and the coupling change over the 7 commits of this revision (3,750 added / 240 removed lines; `app.rs` 22,111 lines of which 3,093 are test modules). Steps 25, 27 and 28 were stale and are rewritten to match delivered behavior. The architecture agreement gained a routing row, a recovery row, and routing/recovery invariants. Task packets T079–T084 record the six slices of this revision with their checks, commit ids, known limits and pending review. The report's conclusion: source is ahead of evidence, so the next revision leads with runtime acceptance for T074/T075/T076/T078 and both recovery prompts, then Repeat/Crop/Invert, then Linux VST3 editor embedding; CPR stays blocked on real Cubase fixtures. Suites: `opentrack` 289 passed / 0 failed, `opentrack-core` 61, `opentrack-audio` 145.

- Step 30, recovery autosave as a real preference (2026-11-01, DeepSeek V4.1 Flash): Preferences > General now has "Keep a recovery copy of unsaved changes" and a 5–300 s interval, replacing the fixed 20 s constant. Both live in `metering::Preferences` (persisted through the existing `metering-preferences-v1` key), sanitize into range, and gate `maybe_autosave`, so off writes nothing and a shorter interval writes sooner. Legacy preference files without the fields default to on with 20 s. Tests: the preference round-trips and clamps (including the legacy JSON), and an app test proves off/too-early/at-interval behavior. `cargo test -p opentrack` 291 passed / 0 failed; `opentrack-core` 61 and `opentrack-audio` 145 pass.

- Edit family completed: Repeat, Fill Loop, Crop, Invert (2026-11-01, DeepSeek V4.1 Flash): three bounded core commands — `repeat_audio_clips` (N copies one selection span later, capped by `MAX_REPEAT_COUNT` = 64 and by the timeline end), `fill_loop_with_audio_clips` (repeat count derived from the loop range) and `crop_audio_clips_to_range` (trims crossing events through the ordinary split path so trims and fades keep their lineage, keeps the event id on the surviving part, and leaves events entirely outside the range untouched). App commands `repeat_selected_audio_events`, `fill_loop_with_selected_audio_events`, `crop_selected_audio_events_to_loop` and `invert_audio_event_selection` each run as one undo step; Repeat opens a small count dialog (Ctrl+K), Fill Loop and Crop live in the Edit and Range menus, Invert in Select. The inert-menu count drops from 176 to 172 entries while wired menu commands rise from 68 to 72. Tests: 6 new core tests (spacing, multi-event span, bounds and no-op cases, fill loop, crop edges and identity) and 2 app tests (one undo-step flow; refusals that leave the project unchanged). `cargo test -p opentrack` 293 passed / 0 failed; `opentrack-core` 67; `opentrack-audio` 145.

- Step 24, VST3 failure isolation (2026-11-01, DeepSeek V4.1 Flash): failures were recorded in the runtime but only visible in the VST3 parameter window. `Runtime::failures()`/`failure_count()` now expose them, the Channel Settings insert row draws a red `!` at the failed plug-in's name with the recorded reason on hover and requests a retry on click (`retry_insert` → `Runtime::retry` → plan rebuild), and opening a project with failures appends "N VST3 insert(s) could not be loaded (marked ! in Channel Settings)" to the status line while the rest of the project plays. `vst3-status.md` now documents the isolation chain and its limits: per-insert quarantine, no process sandboxing, no per-module scan timeout, Windows-only native editor. Test: `a_failed_vst3_insert_is_marked_in_the_rack_and_can_be_retried` renders the rack with a recorded failure and clicks the marker. `cargo test -p opentrack` 294 passed / 0 failed; `opentrack-core` 67; `opentrack-audio` 145.

- Select under cursor and move to locator (2026-11-01, DeepSeek V4.1 Flash): `select_audio_events_under_cursor` selects the events overlapping the transport position (half-open, so an event's own end is not under the cursor) and `move_selected_audio_events_to_locator` moves the selection through the existing `move_selected_clips_to_cursor` path so the first event lands on the chosen locator as one undo step. Both replace inert Select and Move to menu entries; the inert-menu count is now 169. Test: `select_under_cursor_and_move_to_locator_work_on_the_selection`. `cargo test -p opentrack` 295 passed / 0 failed; `opentrack-core` 67; `opentrack-audio` 145.

- Step 24, per-module scan isolation (2026-11-01, DeepSeek V4.1 Flash): `opentrack-plugin-scan` enumerates each module in its own child process of the scanner (`--enumerate-module PATH --output FILE`) under a bounded budget (`--module-timeout`, default 30 s, range 1-600). A module that exceeds it is killed and reported `timed_out`; a module that kills its worker is reported `crashed` with exit code, Unix signal and the worker's stderr; a module that loads but refuses is `load_failed` with its reason, because the worker writes an explicit `Failed` result and exits successfully so "the module said no" is never confused with "the worker died". The scan continues through the rest. `scan_folders_isolated` is what the app and the CLI now use; `scan_folders_enumerating` keeps the in-process path for tests and says so in the report notes. Plug-in Information already listed rejected modules and now receives `timed_out`/`crashed` with reasons, covered by a UI test. The worker ships beside the app (`build.sh` and `build.ps1` now stage
`opentrack-plugin-scan` next to `OpenTrack`), and the app resolves it from its own directory via `ChildProcessEnumerator::for_current_executable`; when it is
missing the scan falls back in-process and both the report note and the status
say so instead of implying isolation. Tests: 9 isolation unit tests (timeout clamp, payload round trip, missing worker, hanging worker killed, crash attribution with stderr, clean exit without result, read-back), 4 integration tests driving the real binary (worker protocol, refused module, full isolated scan, discovery-only scan), and 1 app UI test. App 296 passed / 0 failed; scanner 26 + 4; core 67; audio 145. Shipped release: `builds/x64/OpenTrack` plus `builds/x64/opentrack-plugin-scan`, and the shipped scanner's own `--enumerate` run attributes a rejected module with its reason.

- Persistent scan quarantine (2026-11-01, DeepSeek V4.1 Flash): a module that times out or crashes is now recorded in `vst3-quarantine-v1` (persisted beside the plug-in catalog) as `opentrack_plugins::QuarantineEntry` — module path, reason, the scanner's diagnostic and a timestamp, deduplicated by path with the newest record winning, bounded to 256 entries. The next scan passes those paths to `scan_folders_isolated_skipping`, which reports them as `quarantined` and never spawns a worker for them; plain load failures are not quarantined because a retry costs nothing. Plug-in Information lists the records with per-module Retry and "Retry all quarantined modules"; retrying releases the record so the next scan loads the module again, and a module that enumerates after an update releases itself. The scanner CLI gained `--skip-module PATH` for the same path. Tests: 6 in `opentrack-plugins::quarantine` (status mapping, dedupe keeping the newest, case-insensitive path matching, release, reason bounding, skip text), 3 in the scanner (quarantined module never loaded — proved with a counting enumerator — case-insensitive matching, empty quarantine no-op), and 2 in the app (crash recorded, persisted through eframe storage, skipped with its reason, retry releases; an enumerated module releases itself). Verified end to end with the built scanner: the same module reports `load_failed`, then `quarantined` with `--skip-module`. App 298 passed / 0 failed; scanner 29 + 4; plugins 18; core 67.

- Event Lock and Group (2026-11-01, DeepSeek V4.1 Flash): `AudioClip` gained `locked` and `group` (both serde-defaulted, so older projects load unlocked and ungrouped, and an ungrouped clip stores no field). A locked event keeps its audio and place but refuses move, trim, split and crop — enforced in `move_audio_clips`, `edit_audio_clip`, `split_audio_clip` and `crop_audio_clips_to_range` — and the arrange press on a locked event only selects it with a status message instead of starting a gesture. Grouped events move together because `move_audio_clips` expands a grouped edit by the same delta to every unlocked member, so the guarantee holds regardless of which surface started the move; a locked member stays put. `expand_selection_to_groups` makes clicking any member select the whole group, and locked/grouped events draw an `L`/`G` mark (locked also gets a dashed inset outline). New core commands `set_audio_clips_locked`, `group_audio_clips`, `ungroup_audio_clips`, `expand_selection_to_groups`; new app commands share the Edit menu and the keymap: Lock (Ctrl+Shift+L), Unlock (Ctrl+Shift+U), Group (Ctrl+G), Ungroup (Ctrl+U). The disabled-menu count falls from 169 to 165 while wired commands rise from 76 to 79. Tests: 4 new core tests (locked event refuses all four operations and unlocks cleanly; grouped events move together and ungroup releases all; a locked member stays put inside a group move; lock/group round trip with legacy defaults) and 2 new app tests. App 300 passed / 0 failed; core 71; audio 145; scanner 29 + 4; plugins 18.

- Convert to Real Copy, Auto Select under Cursor, Enlarge Selected Track (2026-11-01, DeepSeek V4.1 Flash): `convert_selected_events_to_real_copy` copies each selected event's source into `Audio/Imports` with the existing collision-safe naming, repoints the event, opens the copy into `data.audio_sources` and commits one undo step; a source already inside the project folder is reported as unnecessary, and an event carrying Pitch & Time edits is left unchanged because the analysis is keyed to the source bytes and modification time, which a copy would change. `auto_select_events_under_cursor` is a persisted app preference (`auto-select-events-v1`) applied from `advance_transport` only when the transport was not playing, so it follows user seeks instead of fighting playback; the menu entry is a checked toggle. Enlarge Selected Track toggles the existing exclusive track enlargement with a check mark. Disabled menu entries fall from 165 to 162; wired entries rise from 79 to 81 (two are checked toggles). Tests: 3 new app tests (copy moves the event onto project media and converting again is a no-op; a Pitch & Time event is left alone with its reason; auto-select follows the preference, the cursor and empty gaps). App 303 passed / 0 failed; core 71; audio 145; scanner 29 + 4; plugins 18.

- Automation follows Events (2026-11-01, DeepSeek V4.1 Flash): `Project::move_audio_clips_with_automation(edits, follow)` shifts the track-volume points strictly inside each moved event's original span by the same beat delta; `move_audio_clips` delegates with `false`, so nothing changes unless the toggle is on. Points exactly on the boundary belong to the neighbouring region and stay, points on the track the event came from travel even when the event moves to another track, and a lane whose shifted curve would collide with a fixed point or leave the timeline keeps its original points instead of becoming invalid. The flag is threaded through the arrange drag (`show_with_tool`/`show_project`) and both command paths, so drag, Move to Cursor and Move to Locator all obey it, and it persists as `automation-follows-events-v1` with a checked Edit-menu item. Disabled menu entries fall from 162 to 161. Tests: 4 new core tests (off versus on, points under the event travel with their values, collision keeps the lane, the source track keeps the curve when the event changes track) and 1 app test (the toggle decides whether the curve travels through Move to Cursor). App 304 passed / 0 failed; core 75; audio 145; scanner 29 + 4; plugins 18.

- Project Logical Editor (2026-11-02, DeepSeek V4.1 Flash): new `opentrack_core::event_query` is a bounded query over audio events — event name contains, track name contains, state (any/muted/unmuted/locked/unlocked) and a half-open start-beat range — plus four safe transforms (mute, unmute, lock, unlock). `Project::matching_event_ids` searches audio tracks only, so a selection count cannot silently include MIDI parts. The app persists the query and transform as `project-logical-editor-v1`; "Project Logical Editor..." opens a dialog with a live match count, a warning when the query matches everything, and Select Matching; "Process Project Logical Editor" applies the saved transform to every match as one undo step and reports matches that already had that state. Disabled menu entries fall from 161 to 159, leaving no roadmap-named inert command. Tests: 6 new core tests (empty query matches all audio events only, name/track/range filters, state filters, transforms only touch matches, clamping, round trip with legacy default) and 3 new app tests (select-and-process flow with its refusals, persisted query, dialog renders its query and live count). App 307 passed / 0 failed; core 81; audio 145; scanner 29 + 4; plugins 18.

- Revision 2 plan started (2026-11-02, DeepSeek V4.1 Flash): `docs/workflow/roadmap-32-steps-v2.md` is the new 32-task plan, ordered so only three items depend on the user (CPR fixtures and the runtime listening passes); everything else is implementable and testable here. First two tasks delivered. **Zoom commands:** Zoom In (H), Zoom Out (G), Zoom In/Out Vertically, Zoom to Selection (Shift+E) and Zoom to Project (Alt+Down) now operate on `ArrangeViewState`; horizontal zoom anchors on the playhead when visible and the view centre otherwise, clamps to the supported range, and Zoom to Selection falls back to the loop range and refuses when neither exists. The arrange records its drawn width in `ArrangeViewState::last_view_width` (not persisted, because a saved width from another window size would misplace the anchor). **Markers:** `Project::markers` with bounded add/rename/move/remove and strict next/previous navigation with wrap; Insert Marker (Insert), Locate Next (Shift+N), Locate Previous (Shift+B), and a Markers window (Ctrl+M) with locate/rename/remove; markers draw a line plus their name in the ruler. Older projects load with no markers and no stored empty list. Tests: 4 zoom app tests, 2 marker core tests, 5 marker app tests. Disabled menu entries fall from 159 to 152. App 316 passed / 0 failed; core 83; audio 145.

- Revision 2, tasks 5 and 6 (2026-11-02, DeepSeek V4.1 Flash): **Split Loop** (`Project::split_audio_clips_at_loop`) cuts every audio event crossing either locator — an event spanning the whole loop becomes three — skips locked events, keeps source identity, and runs as one undo step from Edit > Split Loop. **Nudge** (`nudge.left`/`nudge.right`, Ctrl+Left/Right) moves the selection by one grid step (a quarter bar when snap is off) through the same core move, so locks, groups and Automation follows Events apply; it refuses before the project start, past the timeline end, or when a locked event blocks the move. Disabled menu entries fall from 152 to 151. Tests: 2 core (split loop at both locators, locked/nothing-crosses) and 3 app (split loop undo step, nudge by grid step, nudge refusals). App 319 passed / 0 failed; core 85; audio 145.

- Revision 2, task 8 (2026-11-02, DeepSeek V4.1 Flash): `Project::delete_range_from_audio_clips` removes `[start, end)` from the selected events — an event spanning the range keeps both outer parts, one that starts inside keeps only its tail under the same id, one entirely inside is removed, and locked events are skipped. It leaves a gap rather than closing it, so this is not a ripple edit. Edit > Range > Delete Range runs it as one undo step and reports when nothing crosses. Tests: 2 core (gap and outer parts; inside/outside/locked plus the head and span cases) and 1 app (undo step and the refusal). App 320 passed / 0 failed; core 87; audio 145.

- Revision 2, tasks 3 and 4 (2026-11-02, DeepSeek V4.1 Flash): **Autoscroll** now follows the playhead — the persisted toggle pages the arrange while playing, keeping the playhead inside the middle 80% and returning it to the left tenth, paging back on rewind, and standing aside while a pointer button is held or a wheel event is handled. **Snap to events and cursor**: `ArrangeViewState::snap_type` persists Grid / Grid + Events / Grid + Cursor, chosen from the grid popup and named in the snap tooltip; drags collect clip and MIDI-part edges each frame and snap to a target only when it is within an 8-pixel tolerance and closer than the grid point. An unset grid still means snapping is off. Tests: 4 autoscroll, 1 snap rule and 1 snap persistence. App 326 passed / 0 failed; core 87; audio 145.

- Revision 2, task 7 (2026-11-02, DeepSeek V4.1 Flash): `Project::glue_audio_clips` joins contiguous pieces of one source on one track — same source file, equal gain, no Pitch & Time edit, no fade lineage, contiguous in both timeline beats and source seconds — keeping the earliest event's identity, zeroing the outer fades and refusing anything that would need rendering. Edit > Glue runs it as one undo step and reports why it refused. Tests: 2 core (contiguous split rejoins; every refusal case) and 1 app (undo step plus the two refusals). App 327 passed / 0 failed; core 90; audio 145.

- Revision 2, task 9 (2026-11-02, DeepSeek V4.1 Flash): undo history entries now carry a label, set by the command that commits them (Split at Cursor, Split Loop, Duplicate, Repeat, Fill Loop, Crop, Delete Range, Glue, Nudge, Mute/Lock Events, Group/Ungroup, Move to Cursor/Locator, Insert/Rename/Remove Marker, Convert to Real Copy, Process Logical Editor) and falling back to "Edit". Undo and redo keep the label with the step they move. Edit > Undo History... lists steps newest-first with undone steps marked, the snapshot size on hover and the undo/redo counts, and jumps to any step through the ordinary undo/redo path (so media recovery and dirty state follow); the bounded history limit is unchanged. Tests: 5 (list order and labels, jump back/forward with the available-step fallback, unlabelled steps, a real command's own label, and the window rendering). App 332 passed / 0 failed; core 90; audio 145.

- Revision 2, task 10 (2026-11-02, DeepSeek V4.1 Flash): Project > Consolidate Media makes a project self-contained — it copies every referenced source that lives outside the project folder into `Audio/Imports` once, repoints every event, rewrites the media pool and the open audio sources, and labels the undo step "Consolidate Media". Sources already inside the project are counted and left alone; missing sources are reported and never dropped from the project; a failure during the copy rolls back the copies it made. A second run reports that everything is already local. Tests: 2 (shared source copied once and both events repointed, including pool size; missing source reported with the event intact). App 334 passed / 0 failed; core 90; audio 145.

- Revision 2, task 11 (2026-11-02, DeepSeek V4.1 Flash): `Project::missing_media` lists referenced sources that are not on disk and `relink_media` repoints events and the media pool. Project > Relink Missing Media (N)... appears only when something is missing, opens a folder picker, indexes that folder by lower-case file name (depth 6, 20 000 entries, first match wins) and applies the matches as one undo step labelled "Relink Media"; the relinked audio is opened so it plays immediately, and anything still missing is reported. Tests: 1 core (missing detection, repoint of event and pool, empty mapping no-op) and 1 app (nested folder match repoints two events and the pool, the label is recorded, and both "found none" and "no missing media" are reported). App 335 passed / 0 failed; core 91; audio 145.

- Revision 2, task 12 (2026-11-02, DeepSeek V4.1 Flash): the recovery autosave keeps several points instead of one — each write lands in `Metadata/Recovery/recovery-<ms>.opr`, only the newest `MAX_RECOVERY_POINTS` (5) are kept and the rest are deleted from disk. The open prompt lists every point with its age and size and recovers the one the user clicks (one undo step, as before), or discards them all; a project saved by an older build with a single `autosave.opr` still offers that file. Tests: 2 new (several points pruned on disk while the listing stays bounded; a legacy single autosave is offered and cleared) plus the existing recovery tests updated to the timestamped name. App 337 passed / 0 failed; core 91; audio 145.

- Revision 2, task 13 (2026-11-02, DeepSeek V4.1 Flash): project templates. `Project::as_template` keeps the structure a user sets up once — tracks, groups and membership, routing, gains, EQ, inserts, sends and automation lanes — and drops events, MIDI parts, markers, the media pool and the selection, with the timeline and ids reset. New `templates` module stores them as versioned `templates/*.otemplate.json` under the app data directory (`OPENTRACK_DATA_DIR` override for tests), with sanitized bounded names, staged writes and a bounded listing that skips corrupt or foreign files. Project > Save as Template... opens a name dialog; Project > New from Template lists saved templates with their track counts and starts a new project from one through the ordinary new-project path. Tests: 4 module (structure kept/content dropped, save-load round trip and replacement, name sanitizing, corrupt/foreign rejection) and 1 app (save from a real project, reload, and the empty-name refusal). App 342 passed / 0 failed; core 91; audio 145.

- Revision 2, task 14 (2026-11-02, DeepSeek V4.1 Flash): `session_health` collects the checks OpenTrack already performs into one window — save validation (error), routing validation (error, with the fix named), missing media (warning, naming Project > Relink Missing Media), failed VST3 inserts (warning, naming the red ! retry), a failed insert whose channel is gone (info), an undo snapshot over half its limit (warning) — plus an info summary of tracks/events/MIDI parts/markers, sample rate, tempo and the project folder/file. Findings are ordered errors → warnings → info and summarised in one line; Project > Session Health... shows them. Tests: 5 in the module (clean session, missing media plus bad route, failed inserts and large snapshot, stale failure, ordering) and 2 app (the window renders the issues and the fix; a clean project reports nothing to fix). App 349 passed / 0 failed; core 91; audio 145.

- Revision 2, task 15 (2026-11-02, DeepSeek V4.1 Flash): MIDI quantize with strength and swing. New `opentrack_core::midi_quantize` takes the selected notes (or the whole part when nothing is selected) to a beat grid, moving each note only `strength` of the way, delaying every second grid line by `swing` of half a step, and optionally snapping lengths to whole steps; every note is clamped inside its part, an already-quantized note reports no change, and unusable settings change nothing. The dialog (Edit > Quantize MIDI...) starts from the toolbar's quantize grid with strength/swing sliders and a lengths checkbox, and applies as one undo step labelled "Quantize Notes". Inert menu entries fall 151 -> 150. Tests: 7 core (full strength, 50% strength, swing, lengths and clamping, no-op, selection only, unusable settings) and 4 app (undo step and label, selection plus swing in the message, no-part refusal, dialog rendering). App 353 passed / 0 failed; core 98; audio 145.

- Revision 2, task 16 (2026-11-02, DeepSeek V4.1 Flash): MIDI transpose and velocity. New `opentrack_core::midi_transform` transposes the selected notes (or the whole part) by whole semitones — skipping a note that would leave 0..=127 instead of clamping it onto an octave the user did not ask for — and offsets velocity with clamping at 1 and 127. Both are bounded (`MAX_TRANSPOSE_SEMITONES`, `MAX_VELOCITY_DELTA`) and zero or over-large values change nothing. Edit > Transpose / Velocity... applies either as its own undo step ("Transpose Notes", "Note Velocity") and reports notes that could not move. Tests: 4 core (transpose with range skips, selection-only, velocity clamping at both ends, zero/unbounded no-ops) and 2 app (labelled undo steps plus the no-change refusal, selection-only transpose). App 355 passed / 0 failed; core 102; audio 145.

- Revision 2, task 17 (2026-11-02, DeepSeek V4.1 Flash): MIDI part split and merge in the project model. `Project::split_midi_part` cuts at an absolute beat with the cut at least a 64th note inside the part; notes and controller events before the cut stay, those after move to a new part, and a note crossing the cut is split so nothing played is lost. `Project::merge_midi_parts` joins same-track parts into the earliest part's span, re-adds every note and controller event at its musical position, and moves a selection that pointed at a removed part onto the merged one. The arrange's Split tool now delegates to the same core rule. Edit > Split MIDI Part at Cursor and Merge MIDI Parts are labelled undo steps; merge takes the selected part plus whatever touches or overlaps it and refuses otherwise. Tests: 2 core (split with crossing notes and out-of-part refusals; merge with note/controller positions and the two refusals) and 2 app (labelled split then merge; the two refusal messages). App 357 passed / 0 failed; core 104; audio 145.

- Revision 2, task 18 (2026-11-02, DeepSeek V4.1 Flash): punch in/out recording at control rate. A persisted toggle (Transport > Punch In/Out, `punch-recording-v1`) makes recording follow the locators: with the playhead before the in point, record mode arms and the transport rolls; `advance_punch` starts capture when the playhead reaches the in locator and finishes it at the out locator while playback continues. The decision is `punch_should_start`/`punch_should_stop`, which are pure and tested; unusable locators and a playhead past the out point are refused with a reason. This is control-thread granularity (a frame), not sample-accurate engine punch, and the docs say so. Tests: 5 (arming, refusal past the out point, missing range, the start/stop rules, disarming). App 362 passed / 0 failed; core 104; audio 145. Device review with an interface remains.

- Revision 2, task 19 (2026-11-02, DeepSeek V4.1 Flash): per-track input monitoring modes. `MonitorMode` (Off / Auto while armed / On) is persisted on `Track`; `Track::monitors_input()` is what the playback plan and `project_input_monitor_requested` read, replacing the old boolean, so Auto follows the record-enable state and On works unarmed. The strip's MON control cycles Off → Auto → On and its tooltip names the current state; the Channel Settings snapshot carries the mode, and applying it to a device bus strip forces Off. `ProjectFile::read` migrates an older project's `monitor: true` to On and clears the legacy flag, so monitoring survives the upgrade. Tests: 1 core (mode resolution, the cycle, round trip and the legacy default) and 3 app (snapshot carriage including the bus rule, the legacy file migration through a real read, the control's cycle). App 365 passed / 0 failed; core 105; audio 145.

- Revision 2, task 20 (2026-11-02, DeepSeek V4.1 Flash): loop-recording takes. `AudioClip` gained `take_group` and `take_active` (both serde-defaulted, so older projects read as active, ungrouped takes) plus a project `next_take_group`. When a recording runs with Cycle on and crosses a loop boundary, `Project::add_loop_take_clips` splits the single recorded file into one clip per pass — each starts at the loop start and reads further into the file, all share one take group, and the newest is active. `audio_events::clip_is_audible` now also drops inactive takes, so exactly one pass plays; inactive takes draw dimmed with a `T`. Edit > Make Take Active switches passes as one undo step labelled "Select Take", refuses a plain event and refuses several selected events at once. The recording status names the behavior. Tests: 3 core (three passes with offsets and the last active, the no-split cases, take field round trip and legacy default) and 2 app (only one take is audible and switching keeps every clip, plus the labelled undo step and both refusals). App 367 passed / 0 failed; core 108; audio 145. A real multi-pass take on an interface remains to be reviewed.

- Revision 2, task 21 (2026-11-02, DeepSeek V4.1 Flash): metronome configuration. `MetronomeSettings` gained `accent_mask` — one bit per beat of the bar, with 0 falling back to the plain downbeat click — so the engine accents the beats the user chose instead of only beat one; the level and downbeat pitch it already applied are unchanged. Metronome Setup adds a per-beat accent row clipped to the project's bar length (clearing the last accent restores the downbeat) and a 0-4 bar count-in; both persist as `metronome-accent-mask` and `metronome-count-in-bars`. A count-in rolls the transport back by the lead, forces the click on for those bars without touching the user's toggle, and starts capture when the playhead reaches the original position; a guard stops the started recording from arming another count-in, punch takes precedence, and a start with no room is refused. Tests: 3 engine (mask accents chosen beats, empty mask falls back to the downbeat, defaults unchanged) and 4 app (mask clipped to the bar with the empty fallback, count-in rollback and refusals, the target-beat rule plus the re-arm guard, persistence). App 371 passed / 0 failed; audio 148; core 108. Listening review of the click remains.

- Revision 2, task 22 (2026-11-02, DeepSeek V4.1 Flash): stem export. File > Export > Selected Tracks (Stems)... reuses the mixdown dialog with a `stems` flag: it picks a folder instead of a file and renders each audible track through the same `OfflineRenderer`, format, range and sample rate as a mixdown, with only that track's channel in the graph. `stem_snapshot` drops the track's sends (they belong to the mix) and forces its output to the stereo master while keeping EQ, inserts, hosted inserts, gain and automation; `stem_file_name` numbers and sanitizes names (`001 Drums.wav`, `004 Bass _ DI.wav`) so a folder sorts in track order. The task records how many files to expect, skips a track with no audio, and the status reports a shortfall instead of claiming success. The "Selected Tracks..." disabled entry became the working command, so inert entries fall 150 → 149. Tests: 5 (stem routing, a group stem keeping its channel but losing sends, name sanitizing, the dialog defaults, and an end-to-end render of two real WAVs into a folder with equal frame counts). App 376 passed / 0 failed; core 108; audio 148. Listening/device review of an export remains.

- Revision 2, task 23 (2026-11-02, DeepSeek V4.1 Flash): bounce selection in place. `Project::replace_audio_clips_with_bounce` replaces same-track events with one rendered event, refuses a locked event or a selection that includes another track, and keeps the replaced audio in the media pool. Edit > Bounce Selection in Place computes the selection's span, renders it through that track's own channel with the mixdown renderer (sends dropped, output to the master, float32 at the project rate) into a uniquely named file under `Audio/Imports`, and runs in a background thread with a progress window and Cancel. The finished render is placed as one undo step labelled "Bounce in Place", the new event is selected, and the track stays selected; a failed render or unreadable file removes the partial output and reports why. Tests: 2 core (replacement and media retention; locked/foreign/empty refusals) and 1 app that renders a real two-event bounce and checks the file, the three-second span, the selection and the history label. App 377 passed / 0 failed; core 110; audio 148. Device/listening review of a bounce remains.

- Revision 2, task 24 (2026-11-02, DeepSeek V4.1 Flash): MIDI export of a selection. `opentrack_core::midi_file` gained `MidiExportScope` (track ids, part ids, optional beat range) and `export_standard_midi_scope`; `export_standard_midi` now delegates with an empty scope, so the whole-project file is byte-identical in shape. A note crossing a range boundary is clipped to it rather than dropped, controller events follow the same rule, and a track that contributes nothing to a selection is omitted (the whole-project export keeps its historic shape). File > Export > MIDI Selection... opens a small dialog with Selected MIDI part / Selected MIDI track(s) / Whole project and "Limit to the loop range", defaults to the selected part when there is one, validates that the choice has a target and that the loop range is real, and writes through the same background worker and atomic write as the whole-project command. Tests: 2 core (part/track/empty scope; loop-range clipping with outside notes and CC dropped) and 4 app (scope builder per choice, loop range validation, the two missing-target messages, dialog rendering). App 381 passed / 0 failed; core 112; audio 148. Importing the exported file back remains a listening/DAW review.

- Revision 2, task 28 (2026-11-02, DeepSeek V4.1 Flash): callback budget audit. `CallbackMetrics` gained three bounded atomics (blocks, overruns, worst load permille) updated on the audio thread with no allocation or lock, exposed as `PlaybackEngine::callback_budget()`; the realtime status now reads `DSP n% (worst m%)`. A test-only global allocator with a per-thread counter proves the callback path allocates zero times for the per-sample path and the hosted block path (EQ enabled, metronome on, 512-frame blocks, 8 blocks each). Hosted stage timing (each track, each group, the output stage) now records in test builds without the opt-in environment variable, and a test asserts every stage summary is present and bounded. Measured here (debug test binary, 48 kHz / 512 frames, three hosted tracks plus a group and the output stage): track stages p95 62-65 us, max 65-70 us; output p95 39 us, max 50 us, all under 1% of the 10.67 ms block. Audio 152 passed / 0 failed (4 new); app 381; core 112.

- Revision 2, task 30 (2026-11-02, DeepSeek V4.1 Flash): startup and large-project load. Two new app tests generate a 140-track project (120 audio tracks x 25 events = 3,000 clips, 20 MIDI tracks x 50 notes = 1,000 notes, 3.6 MB on disk) and time save, read, validate and startup with generous ceilings. Measured in the debug test binary: write 145 ms, read 88 ms, validate 2.5 ms, application startup 374 ms. The load path improved too: `Project::validate` used to deep-clone the entire project twice just to validate the two device bus strips, and the group-nesting rule scanned every track once per group. It now validates a cloned strip by itself (skipping the id counters that describe the project, not the strip) and precomputes the parents that contain a group in one pass, so validation is linear and clone-free. Both selection checks moved from the track-list validator to `validate()` because they describe the whole project. App 383 passed / 0 failed (2 new); core 112; audio 152.

- Revision 2, task 31 (2026-11-02, DeepSeek V4.1 Flash): accessibility and keyboard coverage. Disabled/secondary text in both themes now reaches the 3:1 WCAG ratio for non-text UI via `theme::readable_muted` (the dark theme measured 2.27:1 before). Seven bound-but-dead reference shortcuts now dispatch real commands: Select All (Ctrl+A), Select None (Ctrl+Shift+A), Delete (Delete/Backspace), Snap On/Off (J), Autoscroll (F), Quantize (Q) and VST Connections (F4). A test audits the bundled keymap against `dispatch_reference_command` through the source text on every run and asserts twenty essential commands stay handled; a second test asserts body and muted contrast in both themes. `docs/workflow/accessibility-2026-11-02.md` records the numbers (75 of 152 bound commands handled, 121 of 198 chords), lists the 77 uncovered commands by category, documents focus/label behaviour, and states the remaining gaps: screen readers, a high-contrast theme and a manual keyboard-only review. App 388 passed / 0 failed (5 new); core 112; audio 152. Inert menu entries remain 149.

- Revision 2 closed (2026-11-02, DeepSeek V4.1 Flash), task 32: `docs/workflow/roadmap-32-steps-v2.md` now records the delivered set, the blocked set and the revision-3 starting points. Delivered with headless verification: tasks 1-24, 27 (compiled for Windows, unverified at runtime), 28, 30, 31 and 32. Blocked: 25/26 (CPR fixtures) and 29 (no runnable Linux VST3 plug-in, and no Steinberg SDK in the repository). Final suites: app 388 passed / 0 failed / 9 ignored, core 112, audio 152 / 3 ignored, scanner 29 + 4, plugins 18, vst3 14 / 2 ignored, win-jobs 1; declared-but-disabled menu entries 159 -> 149 with 107 enabled menu commands; keyboard coverage 75 of 152 bound commands (121 of 198 chords). Remaining honest gaps, each recorded in its task: device/listening review for punch, loop takes, monitoring, metronome, stems and bounce; Windows runtime for job containment; a manual keyboard-only pass; assists for the 77 undispatched shortcuts. Revision 3 starts with the CPR adapter when fixtures arrive, then the Linux editor path with a test plug-in, keyboard coverage, device reviews, and the next window of menu work chosen by workflow.

- Revision 3 opened (2026-11-02, DeepSeek V4.1 Flash): `docs/workflow/roadmap-32-steps-v3.md` targets the two remaining implementable gaps — disabled menu entries and undispatched keymap commands — measured at 149 entries and 81/152 commands. Slice 1 (transport locate family) is delivered: Locate Next/Previous Event (N/B), Locate Selection (L), Locate Selection End, Locators to Selection (P), Loop Selection (Shift+G) and Play Selection Range (Alt+Space) work from both the Transport menu and the reference keys; Track Folding > Expand All / Collapse All folds every group track; Precount On toggles a one-bar count-in. New core helpers `next_event_start_after`, `previous_event_start_before` and `set_all_groups_collapsed` are strict, span audio and MIDI, and only fold group tracks. Locator commands are one labelled undo step ("Set Locators"), folding is one labelled step ("Fold Groups"), and commands without a selection report instead of editing. Disabled menu entries fall 149 → 137; shortcut coverage rises 75 → 81 of 152. Tests: 2 core and 5 app. App 393 passed / 0 failed; core 114; audio 152. The accessibility report was regenerated with the new coverage and the 71 remaining uncovered commands.

- Revision 3, slice 2 (2026-11-02, DeepSeek V4.1 Flash): keyboard coverage for navigation, transport and view. New dispatch arms: Navigate Up/Down/Top/Bottom and Add Up/Down (track selection, clamped, additive variant keeps the rest), Set Left/Right Locator at the cursor (one labelled "Set Locators" undo step that pushes the other locator out of the way), cursor Forward/Rewind by grid (a bar when snap is off), Fast Forward/Rewind by bar, Nudge Cursor by a sixteenth, To Left/Right Locator, Auto Punch In/Out, ruler format exchange, Split Range (reuses the locator split), Zoom Full and Zoom to Event, Info view / Inspector / track-colour toggles, and automation read/write on every track as one labelled undo step. Shortcut coverage rises 90 → 109 of 152 commands; the accessibility report was regenerated with the remaining 43. Tests: 4 app modules (4 + 5). App 402 passed / 0 failed; core 114; audio 152.

- Revision 3, slice 3 (2026-11-02, DeepSeek V4.1 Flash): Audio > Process Fade In, Fade Out and Silence. `Project::fade_audio_clips` applies a 0.25 s default fade to the selected events' leading or trailing edge, clamped to the event and to the room the opposite fade leaves so the two can never exceed the event; `silence_audio_clips` zeroes the event gain and leaves the media in the pool. Both skip locked events, report when nothing changed, and run as one labelled undo step ("Fade In", "Fade Out", "Silence"). Disabled menu entries 137 → 134. Tests: 2 core (clamping, the opposite-fade room, locked skips, invalid seconds; silence with a locked event and the media retained) and 1 app (three labelled steps, the repeat refusal and the no-selection refusal). App 403 passed / 0 failed; core 116; audio 152.

- Revision 3, slice 4 (2026-11-02, DeepSeek V4.1 Flash): File > New Version (Ctrl+Alt+S). The command writes a numbered copy beside the open project through the ordinary save path, switches the open path to the copy and marks it saved, so the user keeps editing the version they just created — `Song.opr` → `Song (2).opr` → `Song (3).opr`. `version_base_name` keeps the numbering from chaining suffixes after a second use. Without an OPR path the command falls back to Save As; without a managed folder it asks for one. Disabled menu entries 134 → 133; shortcut coverage 109 → 110 of 152. Test: 1 app (two versions, the chained name, the saved flag and the status). App 404 passed / 0 failed; core 116; audio 152.

- Revision 3, slice 2 follow-up (2026-11-02, DeepSeek V4.1 Flash): four more shortcut arms — Left/Right Selection Side to Cursor (E/D, which move the corresponding locator to the cursor through the same labelled undo step), Project Setup (Shift+S) and Toggle Selection (Ctrl+Space, which adds or removes the current track from the arrange selection). Shortcut coverage 110 → 114 of 152; the accessibility report was regenerated with the remaining 38 uncovered commands. Test: 1 added to the shortcut module. App 405 passed / 0 failed; core 116; audio 152.

- Revision 3, slice 5 (2026-11-02, DeepSeek V4.1 Flash): media pool cleanup. Media > Remove Unused Media, Empty Trash and Remove Missing Files now work. `Project` gained `referenced_media`, `unused_media`, `remove_unused_media`, `missing_unreferenced_media` and `remove_missing_unreferenced_media`; a missing source an event still references is kept so Relink can resolve it, and Remove Unused Media never touches the files on disk. Empty Trash drops every unused pool entry and deletes the unused files that live inside the project folder through the existing recoverable-deletion path (Metadata/Undo), while unused files outside the project stay on disk because they may be shared; Undo restores both the deleted files and the pool entries. Menu entries are enabled only when the pool actually has such entries, and each command is one labelled undo step ("Remove Unused Media", "Empty Trash", "Remove Missing Files"). Disabled menu entries 133 → 130. Tests: 2 core (unused detection/removal with the referenced file kept; a missing referenced source kept for relinking) and 3 app (undo step and file retention; missing-entry removal; Empty Trash deleting only project files and Undo restoring them). App 408 passed / 0 failed; core 118; audio 152.

- Revision 3, slice 6 (2026-11-02, DeepSeek V4.1 Flash): Paste at Origin, Quantize Setup, VST Instruments and VST Performance. Edit > Paste at Origin (Alt+V) reuses the paste path with the clipboard's own anchor instead of the cursor, so the copy returns to the beats it came from; `paste_audio_events_at` is the shared body and the existing paste is unchanged. MIDI > Quantize Setup opens the quantize dialog with the toolbar grid. Devices > VST Instruments (F11) opens the instrument-track dialog with the catalog, and VST Performance (F12) opens the realtime diagnostics panel. Disabled menu entries 130 → 126; shortcut coverage 114 → 117 of 152. Test: 1 app (origin paste ignores the cursor; both device keys open their panels). App 409 passed / 0 failed; core 118; audio 152.

- Revision 3, slice 7 (2026-11-02, DeepSeek V4.1 Flash): Project Statistics and Beat Calculator. New `project_tools` module holds a pure `project_statistics` (track kinds, audio events, MIDI parts/notes/controllers, markers, media-pool entries, missing media, automation points, inserts, sends) and a beat calculator with `beats_to_seconds`/`seconds_to_beats`, a last-edited field so the other value follows, and "From Locators" prefilling the loop length. Audio > Statistics opens the counts table with rate/tempo/signature; Project > Beat Calculator opens the converter. Project > Browser (Ctrl+B) now opens the Media tab from its own menu entry, matching the shortcut that already worked. Disabled menu entries 126 → 123. Tests: 3 module (statistics counts and rows, calculator conversions and broken tempo, empty project) and 2 app (both windows render their content). App 414 passed / 0 failed; core 118; audio 152.

- Revision 3, slice 8 (2026-11-02, DeepSeek V4.1 Flash): project notepad. `Project::notepad` is project data: bounded to 100,000 characters, omitted from the file when empty, and read as empty by older projects (`set_notepad` truncates rather than refuses). Project > Notepad opens a multiline editor over a draft; closing it applies the text as one labelled undo step ("Notepad") only when it changed. Disabled menu entries 123 → 122. Tests: 1 core (round trip, legacy default, bound) and 2 app (apply/undo plus the no-change no-op; window rendering). App 416 passed / 0 failed; core 119; audio 152.

- README upkeep (2026-11-02, DeepSeek V4.1 Flash): the README's status sections were stale — it still described automation write as disabled, omitted markers, stems, bounce, punch, monitoring modes, loop takes, templates, recovery points, pool cleanup, project tools and MIDI export, and listed delivered work under "Still needed". `README.md` now describes the current build: a rewritten `Available today` (arrangement, recording/media, mixing, plug-ins, editors, finishing, project tools, platform/diagnostics), a rewritten `Still needed` limited to genuine gaps (runtime/listening review, CPR and interchange, routing limits, 122 disabled entries and 117/152 shortcuts, platform scope, command interfaces, reliability), and a refreshed `Near-term gaps` linking current.md, both roadmaps and the accessibility report. The upkeep rule is now standing guidance in `AGENTS.md` and step 6 of `docs/workflow/README.md`, and a rule in the revision-3 plan: every slice refreshes README status and measured numbers. Docs-only change; no app rebuild required.

- Revision 3, slice 9 (2026-11-02, DeepSeek V4.1 Flash): Nudge Start/End Left/Right. Alt+Left/Right move the selected events' start by one grid step; Alt+Shift+Left/Right move their end. New pure core helper `nudged_clip_edge` plus `Project::nudge_audio_clip_edge`: a start move carries `source_offset_seconds` so the audio does not slide and cannot pass the source's first sample; an end move cannot pass the source's last sample; both keep a 1 ms minimum, clamp fades into the new length, and a locked event is refused. The app resolves each clip's source length from `audio_sources` and falls back to the known extent when a source is not loaded (so an unloaded source can only be trimmed inwards). Each command is one labelled undo step ("Nudge Event Start"/"Nudge Event End") and reports when nothing could move. Shortcut coverage 117 → 121 of 152; disabled entries remain 122. Tests: 3 core (start offset/limits, end limits/minimum, locked and fade clamping) and 1 app (all four commands on real media, the locked refusal and the no-selection refusal). App 417 passed / 0 failed; core 122; audio 152. README and the accessibility report numbers were refreshed in the same slice.

- Native format bump to OPR version 28 (2026-11-02, DeepSeek V4.1 Flash): the native format is still `.opr` (UTF-8 JSON with the `opentrack-project` marker); "version 27" was only its internal schema version, not a different extension. The schema had gained fields after version 27 without a bump — project markers, loop-recording take groups, per-track monitoring mode and the notepad — so a build that only knew version 27 could open such a file and silently drop those fields on its next save. `FORMAT_VERSION` is now 28; versions 1-27 migrate on open, with the new fields defaulting (no markers/takes, monitoring Off, empty notepad, legacy `monitor: true` still becoming On). A tightened migration test rewrites a real document to version 27, removes those fields, and asserts both the defaults and that pre-existing fields (group collapse) survive. `docs/native-project-format.md` records version 28 and no longer says write capture is unimplemented; the README states OPR version 28 and that builds through 27 refuse the file rather than stripping it. App 418 passed / 0 failed; core 122; audio 152.

- Revision 3, slice 10 (2026-11-02, DeepSeek V4.1 Flash): Insert Silence. Edit > Insert Silence (Ctrl+Shift+E) inserts one bar at the cursor across the project. `Project::insert_silence_at` splits every audio event and MIDI part crossing the point (so the right-hand audio moves instead of stretching), shifts clips, parts, markers and automation points that sit at or after it, re-sorts markers, and returns the number of moved items. A locked event crossing the point refuses the whole edit and leaves the project unchanged; a locked event after the point simply moves with the timeline. Invalid positions and lengths are refused. The command is one labelled undo step ("Insert Silence") and reports the count. Shortcut coverage 121 → 122 of 152; disabled entries remain 122. Tests: 2 core (split-and-shift with marker/automation assertions and the refusal cases; locked crossing refused, locked-after shifted) and 1 app (one-bar insert and the locked refusal). App 419 passed / 0 failed; core 124; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 11 (2026-11-02, DeepSeek V4.1 Flash): Cut Time and Delete Time. `Project::remove_time_range` removes `[start, end)`, splits audio events and MIDI parts at both boundaries so surviving material keeps its audio, closes the gap by shifting later clips/parts/automation, and shifts markers only for whole-project edits. A locked event overlapping the range refuses the whole edit and leaves the project unchanged; a locked event after the range moves with the timeline. Ctrl+Shift+X (Cut Time) edits every track and moves markers; Shift+Backspace (Delete Time) edits only the selected track(s) and leaves markers in place, refusing when nothing is selected. Each command is one labelled undo step and reports the changed-item count. Shortcut coverage 122 → 124 of 152; disabled entries remain 122. Tests: 2 core (split-and-shift across audio, MIDI, markers and automation; locks, track scope and invalid ranges) and 1 app (whole-project cut with marker movement, scoped delete leaving the other track and markers untouched, the no-selection refusal and the locked refusal). App 420 passed / 0 failed; core 126; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 12 (2026-11-02, DeepSeek V4.1 Flash): typed position, locators and tempo. Shift+P / Shift+L / Shift+R / Shift+T open one numeric dialog (`NumericInputKind`) seeded from the current transport position, locator or tempo. Applying a position seeks; a locator sets that end and pushes the other out of the way as one labelled "Set Locators" undo step; a tempo is validated to 1-1000 BPM and applied as one labelled "Set Tempo" step with the audio plan rebuilt. Negative positions and locators, non-finite values and out-of-range tempos are refused without touching the project or the undo history. Shortcut coverage 124 → 128 of 152; disabled entries remain 122. Tests: 2 app (all four kinds with their undo labels; the refusal cases leave the project and history untouched). App 422 passed / 0 failed; core 126; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 13 (2026-11-02, DeepSeek V4.1 Flash): Paste Time. Ctrl+Shift+V computes the clipboard's span, ripple-inserts that span at the cursor through `Project::insert_silence_at`, then pastes the clipboard there as one labelled undo step ("Paste Time"). A locked event crossing the cursor refuses the edit and leaves the project unchanged; the check treats crossing events as blocking, not only events that start later. The destination logic was extracted into `build_paste`, shared by the ordinary, at-origin and time pastes, so track/group/channel-width rules stay in one place. Shortcut coverage 128 → 129 of 152; disabled entries remain 122. Tests: 2 app (ripple plus paste with undo restoring the single-step arrangement; empty clipboard and locked-crossing refusals). App 424 passed / 0 failed; core 126; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 14 (2026-11-02, DeepSeek V4.1 Flash): Close Project. File > Close (Ctrl+W) returns to the no-project shell. An unmodified project closes immediately; unsaved changes open a Save and Close / Discard / Cancel prompt. Save writes the project file through the ordinary save path and closes only when that succeeded; Discard closes without writing (the test verifies the file is not created); Cancel keeps the project and its edits. The reset path was factored into `clear_project_state`, shared by new-project and close, which now also clears the clipboard, the close prompt, the transport, the windows and the workspace and re-installs global presets. Shortcut coverage 129 → 130 of 152; the disabled-entry measure stays 122 (Close was rendered disabled by a bare `menu_command(false)`, not in the `disabled_menu_commands` inventory). Tests: 4 app (immediate close and the no-project report; prompt and cancel; discard writes nothing while save-and-close writes and reopens; Ctrl+W reaches the prompt). App 428 passed / 0 failed; core 126; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 15 (2026-11-02, DeepSeek V4.1 Flash): plain-arrow navigation. Navigate Left/Right move the transport cursor by one grid step (a quarter bar when snap is off), clamped at the project start, but only while `ctx.memory().focused()` is empty so a focused control keeps its own arrow handling. Navigate Add Left/Right extend the locator range by a step through the shared "Set Locators" undo step; Add Left stops at the project start and reports. The keymap audit now sees these arms (the guard sits inside the arm rather than in a match guard). Shortcut coverage 130 → 134 of 152; disabled entries remain 122. Test: 1 app (cursor movement and clamping, both range extensions with the undo label, and the start-of-project refusal). App 429 passed / 0 failed; core 126; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 16 (2026-11-02, DeepSeek V4.1 Flash): Find Selected in Pool. Ctrl+F reveals the selected audio event's source in the Media window. `media_pool::show` now takes the revealed path, marks that row with a "▶ " prefix in the accent colour and scrolls it into view; the reveal path is transient app state and is cleared when a project closes. The command opens the Media window, records the path and names the file in the status line; an empty selection and a MIDI-part selection report why there is nothing to show. Shortcut coverage 134 → 135 of 152; disabled entries remain 122. Tests: 3 app (reveal and window focus, both refusals, the Ctrl+F dispatch) and 1 media-pool render test (only the revealed row is marked). App 433 passed / 0 failed; core 126; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 17 (2026-11-02, DeepSeek V4.1 Flash): Adjust Fades to Range. `Project::fit_audio_clip_fades_to_range` fits a selected event's fades to the locators: the fade-in covers the material before the left locator and the fade-out the material after the right one, keeping the range itself at full level; an event fully inside the range loses its fades, and events outside the range and locked events are untouched. Fade lengths are converted from beats to seconds for the project tempo and clamped so they never exceed the event. Audio > Adjust Fades to Range (A) runs as one labelled undo step and reports when no unlocked event crosses the range or the locators are not a real range. Shortcut coverage 135 → 136 of 152; disabled entries remain 122. Tests: 1 core (crossing/inside/outside events, the fade-sum bound, locked skipping, idempotence) and 1 app (undo label, the no-change refusal, the range and selection refusals). App 434 passed / 0 failed; core 127; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 18 (2026-11-02, DeepSeek V4.1 Flash): numeric marker slots. `Project::Marker` gained an optional `slot` (1-9). `set_marker_slot` creates a marker in a slot or moves the existing one while keeping its id and name; `marker_in_slot` addresses it for recall; validation rejects duplicate and out-of-range slots. Ctrl+1/2/3-9 set or move a slot at the cursor, Shift+1/2/3-9 locate to it, and Shift+Numpad1-9 (Recall Cycle Marker) locates as well; the slot is decoded from the chord's digit rather than guessed. Setting a slot is one labelled "Set Marker" undo step; locating is not an edit. OPR moved to version 29 with a v28-to-v29 migration test (no slots → None) and the stricter rule that a file with slots is refused by older builds instead of silently dropping them. Shortcut coverage 136 → 143 of 152; only nine shortcuts remain, all naming editors or features the app does not have. Disabled menu entries remain 122. Tests: 1 core (create/move/identity/uniqueness/validation), 2 app (set-move-locate with real chords, Recall Cycle Marker, empty-slot report and slot 1), 1 migration. App 437 passed / 0 failed; core 128; audio 152. README, native-format docs and accessibility numbers refreshed.

- Revision 3, slice 19 (2026-11-02, DeepSeek V4.1 Flash): Auto Grid. `ArrangeViewState.auto_grid` (serde-defaulted view preference) makes `snap_grid_beats` use the ruler's own zoom-derived subdivision through `auto_grid_step_beats`: a bar at wide zoom, then a quarter, eighth and sixteenth as the ruler refines, clamped to a sixteenth and the bar. With it off the selected grid is used as before, and snap-off still returns `None` in both modes. Shift+Q toggles it, the grid popup carries an "Auto Grid (follow zoom)" checkbox, and the status names the resulting step. The setting persists with view state and defaults off for older projects. Shortcut coverage 143 → 144 of 152; disabled entries remain 122. Tests: 1 view-state (steps at four zooms, off-mode fallback, snap-off, round trip and legacy default) and 1 app (toggle, refined nudge step, restore on off). App 439 passed / 0 failed; core 128; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 20 (2026-11-02, DeepSeek V4.1 Flash): Edit In-place. Ctrl+Shift+I opens the choice of editor for the selection: a selected audio event queues the Sample Editor target (the lower-zone media surface), while a selected MIDI part opens the MIDI editor and hides the sample editor. An empty selection reports instead of guessing. Shortcut coverage 144 → 145 of 152; disabled entries remain 122. Test: 1 app covering the audio path, the MIDI path, and the empty-selection report. App 440 passed / 0 failed; core 128; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 21 (2026-11-02, DeepSeek V4.1 Flash): project overview. Alt+O and Window > Project Overview show a compact whole-project strip in an egui window: one row per track (up to 16 rows) with audio events in grey and MIDI parts in blue, the visible arrange window as a translucent band derived from `timeline_scroll_x`/`last_view_width`/`bar_width`, and the playhead. Clicking or dragging inside seeks the transport and reports the beat; drawing is bounded to 2,000 marks. `show_overview` persists with view state and defaults off for older projects. Shortcut coverage 145 → 146 of 152; disabled entries remain 122. Tests: 2 app (toggle and persistence with a legacy default; render plus a simulated click at 75% seeking to ~12 of 16 beats). App 442 passed / 0 failed; core 128; audio 152. README and accessibility numbers refreshed.

- Revision 3, slice 22 (2026-11-02, DeepSeek V4.1 Flash): event Gain dialog and the editor menu entries. Audio > Process > Gain... opens a dB dialog seeded from the first selected event's gain; `Project::set_audio_clip_gain_db` converts to linear, clamps to 0-4.0 (about +12 dB), treats -60 dB and below as silence, and skips locked events. Applying is one labelled "Event Gain" undo step; out-of-range and non-finite values, an empty selection and a no-change result are reported without writing history, and opening without a selection clears any stale draft. MIDI > Open In-Place Editor and Scores > Open Selection now use the same editor path as Ctrl+Shift+I (Sample Editor for audio, MIDI editor for a selected part). Disabled menu entries 122 → 119; shortcut coverage stays 146 of 152. Tests: 1 core (dB conversion, clamping, silence floor, locked skip, idempotence) and 1 app (dialog seeding, apply with undo label, the refusals). App 443 passed / 0 failed; core 129; audio 152. README refreshed.

- Revision 3, slice 23 (2026-11-02, DeepSeek V4.1 Flash): Auto Fades Settings and Play from Selection. Project > Auto Fades Settings... sets `default_fade_seconds` (0-10 s, persisted as an application preference, seeded at 0.25 s) and the Fade In/Out commands now use it instead of a constant; existing fades are untouched. Transport > Play from Selection Start/End seek to the selection's start or end and request playback, reporting when nothing is selected; playback still depends on an open audio device, so the status names the seek and the transport starts only when the engine is available. Disabled menu entries 119 → 116; shortcut coverage stays 146 of 152. Tests: 2 app (the configured fade length reaching the command; both play edges plus the empty-selection refusal). App 445 passed / 0 failed; core 129; audio 152. README refreshed.

- Revision 3, slice 24 (2026-11-02, DeepSeek V4.1 Flash): play-until transport commands. Transport > Play until Selection Start/End and Play until Next Marker set a transient `play_until` beat and start playback; `advance_play_until` stops the transport with a status when the playhead reaches the target, and any manual stop clears a leftover target so a stale stop cannot end a later playback. Targets at or behind the cursor, a missing selection and a missing marker are reported without starting. Disabled menu entries 116 → 113; shortcut coverage stays 146 of 152. Tests: 3 app (targets for selection edges and the next marker, the refusal cases, and stopping exactly at the target plus the manual-stop clearing). App 448 passed / 0 failed; core 129; audio 152. README refreshed.

- Revision 3, slice 25 (2026-11-02, DeepSeek V4.1 Flash): menu aliases for existing commands. Media > Insert into Project now opens the Add Track dialog for Audio, Instrument and MIDI tracks (the same dialog the Add Track menu uses); Media > Find Missing Files... calls the shared `find_missing_media_dialog` that the Project menu's Relink command uses, and reports when nothing is missing; MIDI > Functions > Merge calls `merge_selected_midi_parts`. No new behavior was duplicated. Disabled menu entries 113 → 108; shortcut coverage stays 146 of 152. Test: 1 app (Find Missing Files reports with no missing sources and opens no picker). App 449 passed / 0 failed; core 129; audio 152. README refreshed.

- Revision 3, slice 26 (2026-11-02, DeepSeek V4.1 Flash): Media > Search Media. The command opens the Media window and focuses a filter field; `media_pool::show` filters rows with `media_matches` (case-insensitive file-name or full-path substring, whitespace-only means no filter) and offers a clear button plus an "No media matches" row. The term is transient app state and is cleared when a project closes; without a project the command reports. Disabled menu entries 108 → 107; shortcut coverage stays 146 of 152. Tests: 3 (matcher edge cases; render showing only matching rows; the command opening and focusing the window plus the no-project report). App 452 passed / 0 failed; core 129; audio 152. README refreshed.

- Revision 3, slice 27 (2026-11-02, DeepSeek V4.1 Flash): media-pool manifest. Media > Export Pool... writes a plain-text manifest — a versioned comment header then `path<TAB>event count` per source, so it can be read, diffed and hand-edited. Media > Import Pool... adds the manifest's existing files to the pool as one labelled "Import Pool" undo step, reports how many were already present and which are missing, and never copies media into the project. Comments, blank lines, duplicate entries and hand-written one-path-per-line lists are accepted; a missing file is reported and not added. Disabled menu entries 107 → 105; shortcut coverage stays 146 of 152. Tests: 2 (manifest text with usage counts plus round-trip/edge cases; export to a real file then import into an emptied pool with the undo label and the no-op second import). App 454 passed / 0 failed; core 129; audio 152. README refreshed.

- Revision 3, slice 28 (2026-11-02, DeepSeek V4.1 Flash): MIDI Insert Velocities. New `midi_transform::set_velocity` and `ramp_velocity`: the ramp orders by note start beat (not file order) and spaces the selected velocities evenly between the start and end values. MIDI > Insert Velocities opens a start/end dialog seeded from the first selected note or 100; equal values set every selected note (or the whole part), different values ramp. Applying is one labelled "Insert Velocities" undo step; a no-change result and a missing part are reported. Disabled menu entries 105 → 104; shortcut coverage stays 146 of 152. Tests: 1 core (set, time-ordered ramp, selection restriction, idempotence) and 1 app (set and ramp with the undo label, the no-change refusal, the no-part refusal). App 455 passed / 0 failed; core 130; audio 152. README refreshed.

- Revision 3, slice 29 (2026-11-02, DeepSeek V4.1 Flash): MIDI Build N-Tuplet. `midi_transform::build_tuplet` places exactly `count` (2-16) selected notes on even slots from the earliest selected note, gives each the slot length, and clamps the requested span to the part end so no note crosses it; a selection whose size does not match the count is refused with a message instead of silently dropping or adding notes. Scores > Build N-Tuplet... opens a dialog seeded from the selection (note count clamped 2-16, span = selected extent) and applies as one labelled "Build N-Tuplet" undo step; a no-change result and a missing part are reported. Disabled menu entries 104 → 103; shortcut coverage stays 146 of 152. Tests: 1 core (even slots, part-end clamping, count/span refusals) and 2 app (apply with undo label and refusals; dialog seeding for three-note and one-note selections). App 457 passed / 0 failed; core 131; audio 152. README refreshed.

- Revision 3, slice 30 (2026-11-02, DeepSeek V4.1 Flash): MIDI Explode Pitches. `Project::explode_midi_part_by_pitch` splits the selected MIDI part into one new MIDI track per distinct pitch: each new part keeps the original start/duration and only that pitch's notes, the source part is removed, and controller events (which have no pitch) are copied once into the lowest-pitched part so they are neither lost nor duplicated. The pitch count is bounded to 16 and the whole edit runs on a candidate, so a refusal leaves the project unchanged. MIDI > Functions > Explode Pitches applies it as one labelled "Explode Pitches" undo step, selects the first exploded track/part, and reports a missing part. Disabled menu entries 103 → 102; shortcut coverage stays 146 of 152. Tests: 2 core (three pitches with a duplicated pitch keeping both notes, controller placement, validation; empty and over-bounded refusals leaving the project unchanged) and 1 app (undo label, track/selection outcome, no-part report). App 458 passed / 0 failed; core 133; audio 152. README refreshed.

- Revision 3, slice 31 (2026-11-02, DeepSeek V4.1 Flash): Set Timecode at Cursor. `Project::timecode_offset_seconds` (seconds, bounded to 99:59:59, serde default 0.0) shifts the timecode shown on the ruler; `timecode_seconds_at(beat)` reads it and `set_timecode_at_cursor(beat, seconds)` computes the offset, refusing non-finite or out-of-range values without editing. Project > Set Timecode at Cursor opens a seconds dialog seeded from the cursor with a live HH:MM:SS:FF preview, applies as one labelled "Set Timecode" undo step, and reports an unchanged value. The ruler's timecode formats (30 fps and 60 fps) include the offset; bars, seconds and samples do not. OPR moved to version 30 with a v29 migration test (missing offset → 0.0). Disabled menu entries 102 → 101; shortcut coverage stays 146 of 152. Tests: 1 core (offset computation, bounds, invalid loaded offset), 1 migration, and 2 app (undo step and refusals; ruler labels with and without the offset). App 461 passed / 0 failed; core 134; audio 152. README and native-format docs refreshed.

- Revision 3 closed (2026-11-02, DeepSeek V4.1 Flash), slices 31-32: Project > Automation Panel lists every track with stored automation plus the selected track, shows each lane's target and point count, and toggles that track's lane visibility through the same `automation_visible` state the arrange reads, with a Hide All action (2 app tests). The revision-3 packet is closed in `docs/workflow/roadmap-32-steps-v3.md` with 31 functional slices plus consolidation. Final measurements: disabled menu entries 159 → 100; keymap-bound shortcuts 75 → 146 of 152; OPR version 30 with versions 1-29 migrating; suites app 463 / 0 failed / 9 ignored, core 134, audio 152 / 3 ignored, scanner 29 + 4, plugins 18, vst3 14 / 2 ignored, win-jobs 1. Remaining work is feature-scoped: six shortcuts with no target, interchange adapters (CPR, MusicXML, OMF, Track Archive, Audio CD), audio-file processing, absent editors/features, and the user-dependent device/listening reviews. README, accessibility report and native-format docs are current.

- Revision 3 post-close, slice 32 (2026-11-02, DeepSeek V4.1 Flash): Audio > Process > Reverse. New `opentrack_audio::reverse_wav_range` reads only the event's used range from the source WAV (streaming through hound, 512 MiB bound) and writes it reversed as float32 to a new file. The app command resolves a unique path under `Audio/Imports`, reverses each selected event (bounded to 64), repoints the clip with `source_offset_seconds = 0`, clears `pitch_time` and `source_fades` because they describe the old source, clamps the fades, opens the new sources, and commits one labelled "Reverse" undo step. A failure deletes the partial files and leaves the project unchanged; empty selection, no managed folder and an unusable range are reported. Disabled menu entries 100 → 99; shortcut coverage stays 146 of 152. Tests: 2 audio (windowed reversal content; unusable ranges and missing file) and 1 app (new media under Audio/Imports with the undo label; a failed reversal leaves no file behind). App 464 passed / 0 failed; core 134; audio 154. README and roadmap refreshed.

- Revision 3 post-close, slice 33 (2026-11-02, DeepSeek V4.1 Flash): Audio > Process > Remove DC Offset. `opentrack_audio` gained `remove_dc_offset_wav_range`, sharing the bounded range reader with Reverse: it computes each channel's mean over the event's used frames, subtracts it, and writes new float32 media. The app command repoints the selected events (bounded to 64), clears pitch analysis and source-fade lineage, opens the new sources, and commits one labelled "Remove DC Offset" undo step; the status reports the largest removed offset in dBFS (e.g. −13.9 dBFS for a 0.2 offset). Failures delete their files and leave the project unchanged; empty selection, no managed folder and unusable ranges are reported. Disabled menu entries 99 → 98; shortcut coverage stays 146 of 152. Tests: 1 audio (stereo per-channel means removed, offsets reported) and 1 app (new media with the undo label; failure cleanup). App 465 passed / 0 failed; core 134; audio 155. README and roadmap refreshed.

- Revision 3 post-close, slice 34 (2026-11-02, DeepSeek V4.1 Flash): Audio > Process > Stereo Flip. `opentrack_audio::flip_stereo_wav_range` swaps L/R pairs in the event's used range into new float32 media and refuses a mono source instead of duplicating it. The app command repoints the selected stereo events (bounded to 64), counts mono events and leaves them untouched, clears pitch analysis and source-fade lineage, opens the new sources, and commits one labelled "Stereo Flip" undo step; failures clean up their files. Disabled menu entries 98 → 97; shortcut coverage stays 146 of 152. Tests: 1 audio (channel swap across the window, short range, mono/empty refusals) and 1 app (repoint with undo label; mono skip without editing). App 466 passed / 0 failed; core 134; audio 156. README and roadmap refreshed.

- Revision 3 post-close, slice 35 (2026-11-02, DeepSeek V4.1 Flash): Audio > Process > Normalize. `opentrack_audio::normalize_wav_range` scans the event's used range, computes its peak, scales to the dialog's target (default −0.1 dBFS, range −60..0), refuses a silent range, and reports `written: false` without creating a file when the material is already within 0.1% of the target. The app command repoints the normalized events (bounded to 64), counts already-level and silent ones, clears pitch analysis and source-fade lineage, opens the new sources, and commits one labelled "Normalize" undo step; failures clean up. Disabled menu entries 97 → 96; shortcut coverage stays 146 of 152. Tests: 1 audio (peak scaling, no-duplicate at target, silence and bad-target refusals) and 1 app (repoint with undo label, already-level and silent reports, empty-selection refusal). App 467 passed / 0 failed; core 134; audio 157. README and roadmap refreshed.

- Revision 3 post-close, slice 36 (2026-11-02, DeepSeek V4.1 Flash): MusicXML import. New `opentrack_core::musicxml` reads a partwise MusicXML score into parts without new dependencies: part names, divisions, pitches with alterations, durations, rests, chords, backups/forwards and a `<sound tempo>` hint, bounded to 8 MiB / 64 parts / 100,000 notes. Ties, slurs, articulations and grace notes are recorded as unsupported and reported rather than guessed; malformed documents, unclosed leaves and scores with no pitched notes are refused. File > Import > MusicXML... parses synchronously (bounded input), creates one MIDI track per part with its notes, selects the first one, and commits one labelled "Import MusicXML" undo step; the status names parts, notes, the score tempo hint (project tempo is left unchanged) and anything not imported. Disabled menu entries 96 → 95; shortcut coverage stays 146 of 152. Tests: 3 core (notes/chords/rests/backups/tempo/part spans; malformed and empty refusals; unsupported elements) and 1 app (tracks/parts/notes with the undo label; malformed file adds nothing). App 468 passed / 0 failed; core 137; audio 157. README and roadmap refreshed.

- Revision 3 post-close, slice 37 (2026-11-02, DeepSeek V4.1 Flash): transport display options. Window > Time Display and Remaining Record Time Display are now checkable menu items backed by persisted `ArrangeViewState.show_time_display` / `show_remaining_record_time` flags (serde default off, so older projects load unchanged). The project readout strip is built by the pure `recording_readout_line`: recording rate and format, `Max` from the free-space capacity, plus optional `Time` (transport position as H:MM:SS.mmm) and `Rem` (capacity minus the cursor position, saturating at zero). Missing capacity reports "unavailable" for both. The strip widened from 260 to 430 points so the optional segments are not clipped on the first frame. Disabled menu entries 95 → 93; shortcut coverage stays 146 of 152. Tests: 2 app (all four flag combinations, clamping, unavailable capacity, and the toggles changing the line) and 1 view-state (round trip plus legacy default off). App 471 passed / 0 failed; core 137; audio 157. README and roadmap refreshed.

- Revision 3 post-close, slice 38 (2026-11-02, DeepSeek V4.1 Flash): Window > Show Panel. The entry is now a checkable menu item bound to the persisted `ArrangeViewState.show_inspector` flag rather than a disabled placeholder. A new `toggle_inspector_panel` holds the flip and status text, and the existing `editors.show_hide_inspector` keymap command now calls it, so the menu entry, the shortcut and the toolbar button all drive one flag. The view state already serialized the flag with a legacy default of shown, so older projects are unchanged. Disabled menu entries 93 → 92; shortcut coverage stays 146 of 152. Test: 1 app (menu action and keymap command share the toggle and status text, round trip, legacy default on). App 472 passed / 0 failed; core 137; audio 157. README and roadmap refreshed.

- Revision 3 post-close, slice 39 (2026-11-02, DeepSeek V4.1 Flash): Media > Prepare Archive and Minimize File. Prepare Archive calls the existing `consolidate_project_media`, so the pool-entry intent is honoured by the already-tested gather-into-project-folder path instead of a duplicate implementation. Minimize File uses new `opentrack_audio::trim_wav_range` (bounded range reader with an exactness requirement, `TrimOutcome { frames, source_frames, written }`) and rewrites each source used by exactly one event to just that event's used range, repointing the clip (`source_offset = 0`, pitch analysis and source-fade lineage cleared, fades clamped), replacing the pool entry and re-keying the decoded source, as one labelled "Minimize File" undo step. Shared sources, ranges extending past their file, missing files and files already used in full are counted and reported rather than guessed at, and a pass that shrinks nothing writes no undo step. Disabled menu entries 92 → 90; shortcut coverage stays 146 of 152. Tests: 1 audio (frames kept exactly, whole-file no-op, past-the-end refusal) and 1 app (trimmed file is smaller with the undo label, shared/past-end sources untouched, the no-gain second pass). App 473 passed / 0 failed; core 137; audio 158. README and roadmap refreshed.

- Revision 3 post-close, slice 40 (2026-11-02, DeepSeek V4.1 Flash): Media > Select in Project. Media Pool rows are now clickable through an extra `selected` handle on `media_pool::show`, which stores the clicked source in transient `pool_selected` and outlines the chosen row in the accent colour; Find Selected in Pool sets the same selection so the two commands agree. Select in Project selects every event whose source is that pool entry and names the count, reporting separately when no row is chosen and when no event uses the file. The selection is cleared when a project closes. Disabled menu entries 90 → 89; shortcut coverage stays 146 of 152. Tests: 2 app (a simulated press/release on the kick row selecting it and surviving later frames; the command's no-selection, unused-file and two-event cases). App 475 passed / 0 failed; core 137; audio 158. README and roadmap refreshed.

- Revision 3 post-close, slice 41 (2026-11-02, DeepSeek V4.1 Flash): Audio > Process > Merge Clipboard. New `opentrack_audio::merge_wav_range` (bounded to 64 layers) reads the destination's used range, adds each copied event at its offset from the copy anchor clipped to the destination length, duplicates a mono layer into stereo or averages a stereo layer into a mono destination, refuses a layer whose sample rate differs instead of resampling silently, and writes new float32 media. The app command repoints the first selected event (`source_offset = 0`, pitch analysis and source-fade lineage cleared, fades clamped), swaps the decoded source, counts copied events that fall outside the destination, and commits one labelled "Merge Clipboard" undo step; a failed render deletes its partial file. Disabled menu entries 89 → 88; shortcut coverage stays 146 of 152. Tests: 1 audio (layer placement and clipping inside the destination, mono-into-stereo, stereo-into-mono averaging, sample-rate refusal) and 1 app (merged media replacing the event with the undo label; the no-clipboard and no-destination reports). App 476 passed / 0 failed; core 137; audio 159. README and roadmap refreshed.

- Revision 3 post-close, slice 42 (2026-11-02, DeepSeek V4.1 Flash): MIDI Explode by channel. `Project::explode_midi_part_by_channel` groups notes and controller events by MIDI channel into one new MIDI track per channel; a channel carrying only controller data still gets a part, so no data is dropped, and the channel set is bounded to 16 with the whole edit run on a candidate so a refusal leaves the project unchanged. MIDI > Functions > Explode now calls the same app path as Explode Pitches (`explode_selected_midi_part(by_pitch)`), differing only in grouping, undo label ("Explode" vs "Explode Pitches") and wording; the first exploded track/part is selected. Disabled menu entries 88 → 87; shortcut coverage stays 146 of 152. Tests: 2 core (per-channel notes and controllers including a controller-only channel; empty and over-bounded refusals) and 1 app (channel explode with its undo label and selection, plus the pitch variant keeping its own label). App 477 passed / 0 failed; core 139; audio 159. README and roadmap refreshed.

- Revision 3 post-close, slice 43 (2026-11-02, DeepSeek V4.1 Flash): MIDI List Editor. MIDI > Open List Editor lists the selected part's notes sorted by start time with start, pitch name (shared with the Sample Editor naming), length, velocity and channel; clicking a start time selects that note and locates the transport, and Delete Notes removes the selection. The note-deletion logic was extracted into `delete_selected_midi_notes`, now shared with the MIDI editor's Delete key, and gained an explicit "Delete Notes" undo label and a removed-note count. The window reports when no MIDI part is selected and closes with the project. Disabled menu entries 87 → 86; shortcut coverage stays 146 of 152. Tests: 2 app (the table's contents and start ordering plus the no-part report; deletion with its undo label, the empty-selection and missing-part refusals). App 479 passed / 0 failed; core 139; audio 159. README and roadmap refreshed.
