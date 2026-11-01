# T067 - Timeline double click toggles playback

- Status: review_pending (source and checks complete; running-app review pending).
- Scope version: 1.
- Desired observable outcome: double-clicking the timeline block at the top of the Project, the MIDI editor or the Sample Editor starts and stops playback.
- User request (2026-11-01): "double clicking on the timeline in the project window, or in midi editor or audio editor should play and stop the audio - only when on the timeline block at the top of each."

## Scope

- Double-click with the primary button on the ruler response in `show_project` (`project-time-ruler`), the MIDI editor ruler (`midi-time-ruler`) and the Sample Editor ruler (`sample-ruler`) requests one transport toggle.
- The request is stored per viewport and consumed once at the start of the next UI pass by `OpenTrackApp`, which calls the existing `set_transport_playing(!playing)` path. Ruler click/drag seeking, range editing, zoom and pan are unchanged.
- Only the ruler block counts; double-clicks elsewhere in the arrange area, note grid or waveform keep their existing behaviour.

## Architecture

UI-only. `request_transport_toggle` writes a boolean temp request; the app clears it in `eframe::App::ui`. No engine, persistence or dependency changes.

## Checks

- `a_ruler_double_click_requests_one_transport_toggle`: the request is set and consumed exactly once.
- Full app suite and formatting checks.

## Handoff

Actual implementer: DeepSeek V4.1 Flash; baseline `cb5a8c6` plus local work. Project, MIDI editor and Sample Editor rulers request playback toggling on a primary double click; the main UI applies it once per frame through the existing transport path.

Live review of the interaction (including the seek that the first click of the double click performs) remains pending. If the seek before play/stop is unwanted, the next step is to suppress the seek gesture when a double click is detected.
