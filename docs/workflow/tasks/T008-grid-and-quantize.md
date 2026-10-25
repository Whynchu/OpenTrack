# T008 — Activate Grid Type and Quantize Type

Status: review_pending. Actual implementer: GPT-6.1 Sol. Authority: user requested activating both controls to govern snapping. Scope transport-v4 adds grid-selection (common weight 2) to the seeking workflow; all prior requirement weights/membership are retained.

## Behavior and boundaries

- Both existing 84px toolbar fields are clickable popups with their observed Grid Type / Quantize Type tooltips. Existing toolbar positions, styling and grouping are retained.
- Grid Type choices: Bar, Beat, Use Quantize. Bar uses project beats-per-bar; Beat uses one quarter-note beat in the existing transport time model; Use Quantize uses the selected note interval. Quantize does not override an explicitly selected Bar/Beat grid.
- Quantize offers 1/1 through 1/128 straight notes, plus Triplet and Dotted submenus. Triplets multiply duration by 2/3; dotted notes by 3/2. The selected toolbar label displays the note plus T or a dot where applicable. These functional menu choices/grouping are implementation decisions; the full reference popup contents were not measured in this slice.
- One `ArrangeViewState::snap_grid_beats` resolver controls cursor click/release, range draw endpoints, rigid movement delta and corner resize. Snap off returns no interval. Rigid movement still preserves fractional range length; resizing keeps the fixed opposite edge and mode.
- Use Quantize displays its actual globally anchored lattice, including triplet/dotted positions. A power-of-two subset is shown when lines would be closer than 8px; snap retains the original interval. Musical ruler labels retain their independent adaptive density.
- Grid/quantize choices persist in workspace preferences. OPR v17 records them; older supported files migrate, with absent fields defaulting to Bar and straight 1/16. Preference changes participate in the existing workspace dirty fingerprint; navigation normalization does not erase them. This does not quantize existing MIDI notes/events or add Snap Type modes.
- No new dependencies, audio processing, Mixer or range model changes. EXAMPLE was not opened or modified.

## Checks

- Existing and final x64 release builds passed with no OpenTrack process running. No app window launched; desktop review remains with the user.
- 11 headless actual ruler/UI tests passed. New tests click the actual popup entries (including Triplet), verify backing settings, compare Bar/Beat/1/8/1/16 seeks, and exercise draw/move/resize with the selected 1/8 interval. Existing gesture, label, cursor, corner resize and pan checks pass.
- 13 timeline tests passed, including grid resolution and triplet/dotted unit conversions. 11 project-file tests passed, including OPR settings round-trip, legacy defaults and v16 migration. Formatting/diff checks passed.
- The prior unrelated full-suite theme assertion remains unresolved; this slice did not rerun that full suite. Live popup appearance, Windows pointer behavior, precise reference parity, audible and close gates remain pending.

## Identity

- EXE SHA-256: `26C78F5DEED2C03BE31A26ECD5558806A5F6E0A735FE3765BFC8531C92DE4933`.
- `app.rs`: `7968A58EF0A4B609C7D9F9D514E98885861FFC15385B6E89B65E7E3004404887`.
- `timeline.rs`: `E13D7B0D16D9C27C34C2BC9E0042A18FF7892C6B4115A9F9208A78D758DF673A`.
- `project_file.rs`: `E303EFCB4282F46264A6695EA89FFFAE688E735057A2B85DB51B70BC3E1705F3`.
- Base commit `cbd9835` plus uncommitted transport work. No current screenshot or live acceptance is claimed from build/headless results.

## User check

1. Enable Snap. Try Bar and Beat, seeking to the same off-grid point. Confirm measure versus beat boundaries.
2. Choose Use Quantize. Compare 1/8, 1/16, Triplet and Dotted choices while zoomed in enough to see their spacing. Verify cursor release, new range endpoints, range movement and corner resizing follow the interval.
3. Disable Snap and verify free movement. Re-enable it. Use Save As to a disposable OPR, reopen and check both fields retain their choices.
4. Recheck range duration preservation, opposite-corner stability, loop/skip playback, mixed seek/zoom, pinned ruler and scrollbars. Stop for feedback before the next packet.
