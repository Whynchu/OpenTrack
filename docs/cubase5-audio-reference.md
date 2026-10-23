# Cubase 5 audio reference notes

These notes are transcribed from the local Cubase 5 Operation Manual. They are source facts, not runtime measurements. The exact Cubase DSP curves, Q scaling, EQ defaults, and audio-device timing still need controlled comparisons in the reference application.

## Source

- Local copy: `../_analysis-tools/cubase-reference-view/Operation_Manual.pdf`
- Recording chapter: printed pages 67–72 (PDF pages 67–72).
- Channel EQ: printed pages 122–125 (PDF pages 122–125).

## Recording facts from the manual

- Cubase can record one or several audio and/or MIDI tracks at once. Record Enable puts a track into record-ready mode (p. 67).
- Record starts at the current project cursor in Stop mode and can begin during playback from the current cursor (pp. 67–68).
- Pressing Record during an active take ends recording while playback continues; pressing Stop ends recording and stops transport (p. 68).
- The manual lists 16-bit, 24-bit, and 32-bit float recording formats; WAV is a supported file type (p. 69).
- Audio tracks can be mono, stereo, or surround. Input buses are selectable per track; the audio channel configuration is maintained through the track signal path, including EQ and mixer settings (p. 70).
- The recording source can also be a group or FX channel bus (p. 71).
- Multiple channels can be soloed at once. Ctrl/Command-click Solo clears the other solo states; Alt/Option-click toggles Solo Defeat so a channel remains audible while another is soloed (p. 119).
- Cubase's built-in EQ has up to four bands. Each band covers 20 Hz–20 kHz and offers ±24 dB gain. EQ1 and EQ4 support parametric, shelf, and high/low-pass types; EQ2 and EQ3 are always parametric (pp. 123–124).
- EQ settings apply to both sides of a stereo channel (p. 122). The manual explains Q as bandwidth control but does not give the numeric Q range in these pages.

## OpenTrack- implementation decisions

- The audio recorder uses the selected mono/stereo input and writes 32-bit float WAV takes for every armed audio track through one shared input stream. Each armed track currently receives the same input signal; per-track input routing is still missing. Windows MIDI input can record on several armed MIDI/Instrument tracks at the same time. It does not expose audio input buses, record folders, surround formats, punch locators, cycle takes, or pre-record buffering. An optional audio input monitor and the MIDI Thru oscillator are OpenTrack- implementation choices and have not been measured against Cubase.
- Record starts transport at the current cursor. Pressing Record again ends the take while playback continues; Stop ends the take and stops transport.
- Cycle playback currently uses editable OpenTrack- left/right locators measured in quarter-note beats. The playback callback wraps at the right locator; this behavior and the default 0–16 beat range are implementation choices pending direct Cubase comparison. Cycle recording and take lanes are not implemented.
- The channel strip has four persisted bands with the manual's documented gain/frequency ranges and outer/middle filter choices. The current Q range (0.1–10), RBJ-style coefficient formulas, initial frequencies, and default outer filter types are OpenTrack- choices, not Cubase measurements.
- Mixer meters show live input peaks on an armed recording track and post-fader playback peaks per channel. The -60 to 0 dBFS scale, peak falloff, and clip indicator are implementation choices pending comparison with Cubase.
- TestGenerator is the first manual-documented Cubase plug-in exposed in the insert chain. It uses the previously documented original DSP prototype; noise spectra, antialiasing, defaults, and output parity remain unverified.
- Stereo Delay is an original OpenTrack- insert processor. It is not represented as a rewrite of a named Cubase effect until the reference plug-in behavior has been measured.

## Still unknown or unverified

- Cubase's exact default EQ band values and Q scaling, filter slopes, coefficient precision, and response curves.
- Exact take-start alignment between input and output device clocks, latency compensation, Cubase monitoring behavior, and stream behavior under device failure. OpenTrack-'s monitor latency and dropout behavior also need target-PC checks.
- Plug-in processing order and state semantics for each bundled Cubase effect.
- Whether current device drivers support the default input/output formats selected by OpenTrack- on the user's machine.
