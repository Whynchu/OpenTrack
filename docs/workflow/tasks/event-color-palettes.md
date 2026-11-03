# Event color palettes and the platform target

- Status: accepted for source and automated checks; desktop review pending
- Scope version and requirement IDs: post-revision-5 request (user, 2026-11-02) - palette selection for the color selector, and confirmation that Linux is a real target
- Desired observable outcome: Event Colors offers a palette selector with at least five built-in flavors plus the default; choosing one fills all 16 slots, and any slot can still be edited afterwards. Windows x64 and Linux x86_64 are both stated target platforms with their real gaps named.
- Reference evidence: `crates/opentrack-app/src/app/color_palettes.rs` (new); `show_event_colors_dialog` and `apply_event_color_palette` in `app.rs`; `DEFAULT_EVENT_COLORS` in `project_file.rs`; README and AGENTS.md platform statements.
- Dependencies and decisions needed: none. The palette identity is not stored, so OPR stays 38.
- Code scope and architecture boundaries: one focused module holding the palettes and matching logic; the dialog gains a combo and one loader method. No audio, project-schema or menu-inventory change.
- Acceptance checks (action, expected result, fixtures, evidence to retain): the palette list holds at least five complete, distinct 16-color sets with the shipped default first; choosing a palette fills every slot, marks the project modified and names the palette; reopening the dialog recognises the palette in force; a hand-edited set reads as "Custom"; an unknown index changes nothing; the dialog draws the selector.
- Affected regressions: none expected; per-slot editing, Reset and the selected-slot behaviour are unchanged.
- Essential gates: persistence (the colors are already part of saved workspace state), no hidden rebinding of track colors, no format bump.
- Review owner and milestone stop: self-review; visual check of the six palettes in the running application stays with the user.

## Palette set

| Palette | Intent |
| --- | --- |
| Default | The 16 colors OpenTrack has always shipped, so existing projects are unchanged |
| Muted Earth | Desaturated clay/olive for long arrangement sessions |
| Cool Studio | Blues and teals for a quieter, low-saturation look |
| High Contrast | Vivid, clearly separated hues for dense event stacks |
| Pastel | Light tints that keep dark labels legible |
| Grayscale | Neutral ramp for color-blind-safe or print-oriented work |

Each palette is a complete 16-slot set rather than an offset: the arrange always has a full palette, and the result does not depend on the previous colors. "Default Color" is separate from the palette: it is the standard neutral grey (160,160,160) and is what a track has before a palette or a slot is applied. Applying a palette assigns every track the slot for its id, `(id - 1) % 16`, as one "Apply Event Color Palette" undo step, so the palette visibly recolors the arrange and each track keeps a stable color across reordering and renaming. A track added afterwards starts grey; picking a slot colors it and "Default Color" clears it back to grey.

## Follow-up: the first cut did not reach the arrange

(Model after the last correction: palette slots 1-16 keep their designed colors; grey is only the separate Default Color.)

The user reported that the palettes looked right in the dialog but did not apply. That was correct: the dialog and the toolbar swatch read `event_colors`, while every track strip, event, Mixer strip, Inspector header and Sample Editor waveform read `project_controls::track_color`, which used the **constant** `DEFAULT_EVENT_COLORS` plus `track.color_index`. Nothing painted with the chosen palette.

Fixed by making the drawing palette a single source:

- `project_controls` holds the active palette (thread-local, so parallel tests stay isolated) and exposes `set_active_event_colors` / `active_event_colors`.
- `track_color` takes an explicit `color_index` from the **active** palette; without one, a media track is the standard grey (Default Color), and a bus keeps its blue identity.
- The app writes the workspace `event_colors` into the drawing palette at the top of every frame, in `apply_event_color_palette`, and when a slot is edited, so a palette change, project load or undo recolors all surfaces in the same frame.
- `has_track_color` reports true when a track carries an explicit palette slot, so the Mixer and Inspector only accent colored tracks.
- The per-track selector's first swatch is the grey Default Color, and its palette row follows the active palette.

The palettes were also resaturated after the review: Muted Earth, Cool Studio, High Contrast, Pastel and Grayscale were regenerated with stronger chroma, and Grayscale uses a wider dark-to-light ramp. Every set keeps its designed 16 colors (user direction, after a wrong first attempt that put grey in slot 1); grey is the separate Default Color identity.

New coverage: `default_color_is_grey_and_explicit_slots_follow_the_active_palette` (grey default, explicit slots, palette switch recolors, id-to-slot mapping) and the app test asserts that applying a palette assigns slots by id, writes one undo step, recolors the same tracks on a switch, and leaves a later track grey.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: new `color_palettes` module (six palettes, `matching_palette`, `palette_name`); `EventColorsDialog` gained a palette field seeded on open; `apply_event_color_palette` validates the index, writes all 16 slots, marks the project modified and reports the palette name; the dialog gained a "Palette" combo above the slot grid, keeps the slot editor, and re-derives "Custom" after a slot edit or Reset. README and AGENTS.md now state the two-platform target.
- Commands/checks and actual results: `cargo test -p opentrack color_palettes` -> passed; `cargo test -p opentrack event_color_palette` -> 2 passed; app suite 582 passed / 4 known device-dependent roll failures / 9 ignored; core 166; audio 171 / 3 ignored; `cargo fmt --all --check` clean; inventory regenerated after the app edit.
- Runtime environment: Kubuntu Linux x86_64, built with `./build.sh`; no desktop session in the harness, so the visual review is pending.
- Ledger IDs updated: README (feature list, platform lines, Linux build instructions) and `AGENTS.md` (target platforms).
- Review findings: `mark_modified` writes its own status text, so the palette loader marks the project first and then sets its message; otherwise the palette name was immediately overwritten with "Modified".
- Failures/unknowns/pending acceptance: visual check of the six palettes and the dialog layout on the desktop; a decision on enabling cpal's `realtime` feature for Linux scheduling.
- Next ready task: continue filling the remaining disabled menu features; the list is in `docs/reviews/generated-source-inventory.md`.
