# Cubase 5 MIDI editor reference and OpenTrack- status

This document separates manual facts from current implementation choices. No MIDI editor behavior has been measured against a running Cubase session yet.

## Source

- Local file: ../../_analysis-tools/cubase-reference-view/Operation_Manual.pdf
- MIDI editors: printed pages 338–348.
- Key Editor overview and note display: printed pages 340–341.
- Drawing notes, snap, length quantize, and velocity: printed pages 344–345.
- Resizing and deleting notes: printed page 348.

## Cubase manual facts

- The Key Editor is Cubase's default MIDI editor and displays notes in a piano-roll-style grid (p. 338).
- In the note display, note width represents note length and vertical position represents MIDI note number. A piano keyboard appears to the left (p. 341).
- Snap constrains horizontal editing positions. In Bars+Beats mode, its grid follows the editor's quantize value (p. 344).
- Clicking with the Pencil inserts the length selected by Length Quantize. Dragging creates a longer note in multiples of that value (p. 345).
- New notes take their velocity from the insert-velocity field (p. 345).
- Cubase provides note selection, movement, resizing, and deletion; note length can also be edited numerically on the info line (pp. 347–348).
- MIDI notes live inside MIDI parts. The Project window arranges those parts on MIDI tracks (pp. 26, 28, 338).

## OpenTrack- implementation choices

- MIDI and Instrument tracks contain arrangeable MIDI parts. Create a four-bar part at the transport position with **+ MIDI Part**; this default length is an OpenTrack- choice pending a reference check.
- Select a part in the Project arrange view and open the MIDI Editor from the Window menu, or double-click the part.
- The keyboard shows pitch class plus MIDI key number, avoiding an assumed octave-name convention.
- Empty-cell clicks add notes snapped to the selected straight-note grid. Supported position and insertion-length choices are 1/64, 1/32, 1/16, 1/8, quarter, half, and whole notes.
- The initial insert velocity is 96. Users can select a note and adjust its length and velocity, drag its body to move it, drag its right edge to resize it, and right-click to delete it.
- The project format stores the part start and length in quarter-note beats. Notes use beat positions relative to the part, duration in beats, pitch 0–127, MIDI channel 0–15, velocity 1–127, and project-wide IDs.
- Empty-cell clicks add notes snapped to the selected straight-note grid and clipped to the part boundary. Position snap, insertion length, and insert velocity are application preferences. MIDI part, note, controller-event, note-channel, controller-channel, and instrument waveform state use RWP format version 14. Older notes and controller events default to MIDI channel 0.
- The controller lane can show Modulation (CC 1), Volume (CC 7), Pan (CC 10), or Expression (CC 11). Click in the lane to add a snapped controller point, drag a point to edit its beat and value, and right-click to remove it. Points are stored at beat positions relative to their MIDI part; the lane draws held steps between points.
- During playback, MIDI and Instrument track notes are rendered through the Rust instrument processor interface. Instrument tracks can select a sine, square, sawtooth, or triangle oscillator; notes use a bounded 64-voice scheduler, and supported controller events change modulation, volume, pan, and expression. Controller state is restored from bounded checkpoints after seeking. The generated signal passes through track EQ, inserts, mute/solo, and gain. These are OpenTrack- starter voices, not Cubase instrument rewrites.

## Unknown or not implemented

- Cubase's default part length, snap, Length Quantize, and insert velocity settings for the installed build.
- Cubase's exact octave naming preference, note editor colors, note audition behavior, and project-part boundary behavior.
- Cubase's controller-lane appearance, defaults, drawing rules, and controller response have not been compared in a running reference session. OpenTrack- only exposes four common controls in the editor; other valid controller numbers can be stored but the starter oscillator ignores them.
- Windows MIDI input is selected from the MIDI menu. Armed MIDI and Instrument tracks record notes and controller changes from the selected input; several MIDI tracks can record at once, including alongside multiple armed audio tracks. MIDI Thru sends live input to the armed starter oscillator through the default audio output. Input uses WinMM and has not been checked with physical hardware. MIDI output ports, part resizing/moving, richer instrument controls, and presets are not implemented.
- MIDI note persistence and output scheduling are implemented, but playback has not been compared with Cubase and the built-in oscillator voice does not claim instrument parity.
