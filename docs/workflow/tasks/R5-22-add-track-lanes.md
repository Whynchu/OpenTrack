# R5-22 — Add Track lane mapping

- Status: accepted
- Scope version and requirement IDs: revision 5, step 22
- Desired observable outcome: the Add Track entries either perform their named action through an existing surface or stay honestly unavailable; no fake audio track kinds.
- Reference evidence (IDs/paths): Add Track menu; `toggle_marker_lane`, `toggle_second_ruler`, `open_tempo_editor`, `midi_transform_dialog`.
- Dependencies and decisions needed: R5-05 (tempo editor) for Signature/Tempo. `Video` needs a decoder decision (step 28); `Browse Sounds...` needs a browser.
- Code scope and architecture boundaries: menu wiring plus two small helpers; no new track kind, no schema change.
- Acceptance checks: Marker and Ruler toggle the existing lanes with status text; Signature and Tempo open the tempo editor; Transpose seeds the MIDI transpose dialog and only enables with a MIDI part; Video and Browse Sounds stay disabled; a test asserts the toggles and that no lane entry invents a track.
- Affected regressions: six Add Track rows move from unavailable to available/state-dependent.
- Essential gates: none beyond the previous steps.
- Review owner and milestone stop: self-review; desktop review of the submenu.

## Decision record

- `Project > Add Track > Marker` and `Ruler` map to the existing marker lane and second ruler: both are timeline presentations backed by project or workspace data, which is the closest honest meaning available without inventing a track kind.
- `Signature` and `Tempo` open the Tempo Track editor, which owns both values.
- `Transpose` opens the existing Transpose/Velocity dialog; a Cubase-style transpose *track* remains unimplemented and is not claimed.
- `Video` and `Browse Sounds...` remain unavailable with their existing reasons.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: replaced the disabled Add Track block with wired entries; extracted `toggle_marker_lane`/`toggle_second_ruler` so the menu and tests share one path.
- Commands/checks and actual results: `cargo test -p opentrack add_track_lane` → passed; app suite 571 passed / 4 known device failures / 9 ignored; inventory 69 unavailable rows.
- Runtime environment: Kubuntu Linux.
- Ledger IDs updated: README disabled-row count 69.
- Review findings: none.
- Failures/unknowns/pending acceptance: desktop review of the Add Track submenu; the Transpose row's enablement depends on a selected MIDI part, which the desktop pass should confirm reads clearly.
- Next ready task: R5-23.
