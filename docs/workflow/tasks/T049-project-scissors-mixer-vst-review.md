# T049 - Project scissors, Sample Editor guide, Mixer selection and VST review

- Status: source changes implemented; x64 local-ASIO build passed; final in-app VST meter and user acceptance pending. Implementer: Codex session with OpenCode continuation (DeepSeek V4.1 Flash); no independent review.
- Scope version: `project-mixer-vst-review-1`.
- Requirement IDs: `project-scissors-toolbar-only`, `snap-aware-scissor-cursor`, `sample-editor-full-height-cut-guide`, `mixer-fader-and-track-selection`, `vst3-editor-always-on-top`, `vst3-native-editor-repaint`, `vst3-audio-insert-routing`, `opentrack-window-icon`.

## Requested outcome

Remove the Project event's selected-track cut band. Project cuts remain available
through the Scissors tool. Add the snapped cut guide through the Sample Editor's
ruler and waveform or Pitch & Time grid. Clicking an ordinary Mixer channel
surface selects that track in Project and Inspector. Native VST editor windows
provide an Always on Top option. Improve Waves editor initial painting and
diagnose the report that Waves interfaces are blank and audio does not reach VSTs.
Replace the current app mark with the user's earlier tape-track style.

## Implementation and evidence

- Removed the contextual cut-band hit test, split action and cursor behavior from
  Project events. Toolbar Scissors behavior remains.
- The Sample Editor's snapped guide now spans from the ruler through the waveform
  or Pitch & Time grid. The cursor stays at the real mouse point while the line
  shows the committed snapped location. Snap off keeps the scissors without a line.
- Mixer track selection now covers the full ordinary track strip; channel controls
  retain their own interactions. Bus selection behavior stays on its existing
  name area and channel-settings control.
- Added `Always on Top` to the native VST editor system menu. It raises the editor
  within the ordinary application z-order band; it does not use global topmost.
- Native VST views now run on a dedicated STA thread with their own message pump
  and repaint cadence. The view is created from the live audio effect's controller,
  so CLA-76 controls and VU/GR meters respond to the actual processor. A matching
  hosted-processing failure is shown in the app status bar; the audio engine passes
  the track's dry signal when an insert fails.
- Replaced the old app mark with the user's supplied reel artwork. The exact source
  is stored as `docs/images/opentrack-logo-source.png`; white and black variants
  are used in the application header, README and taskbar icon.
- Source tracing confirms enabled VST inserts are included in track playback
  settings, process the track signal before gain/pan/output routing, and fall back
  to dry on an error. Installed CLA-76 mono/stereo audio checks pass. The final
  in-app meter, save/reopen and long-run behavior remain user-review items.

## Review gates

- Project: select a track, hover and click through its lower waveform, then verify
  no cut occurs. Choose Scissors, test snapped and unsnapped cuts, and confirm the
  cursor remains under the mouse while the snapped guide shows the cut location.
- Sample Editor: test Scissors in waveform mode and Pitch & Time mode; verify the
  line spans from the ruler through the edit area and follows the committed grid
  location. Confirm note scissors still split notes without splitting audio events.
- Mixer: click empty space and controls on an ordinary track strip; confirm Project,
  Inspector and Mixer selection agree and the controls still operate.
- VST: open a Waves editor and an Ozone editor, check that the graphics paint,
  open the title-bar system menu and toggle Always on Top, then verify an OpenTrack
  window can be covered while other applications remain above the editor.
- Audio: play a known signal through a Waves insert and confirm audible processing
  reaches Stereo Out. If processing fails, confirm the status message names the
  track/insert and the dry path remains audible. A successful build alone does not
  establish this gate.
- Icon and header: review the supplied white/black reel mark in the application
  menu bar, title bar/taskbar and README in both light and dark desktop themes.

## Build and process state

The final pre-build process check found no running OpenTrack instance. The x64
local-ASIO release build succeeded with SHA-256
`84CCD48A721567199C6D6730E2716EC468EE803A8AD18F86EC03A11EB9D52836`. Formatting,
whitespace and targeted checks passed: app 133 passed with 8 opt-in ignored and
six separate pitch failures, VST3/scanner/catalog suites pass, hosted audio 6
passed, installed CLA-76 audio and dedicated-editor checks pass. The app was not
launched by this session after the final build. Do not claim final in-app meter
or audio parity from source inspection or compilation.
