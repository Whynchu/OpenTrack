# T066 - Channel Settings rack density and plug-in editor access

- Status: review_pending (source and focused checks complete; running-app review pending).
- Scope version: 1.
- Desired observable outcome: the Channel Settings window follows the channel you click, the insert/send racks are compact and clearly clickable, and the plug-in editor has one obvious open/close control.
- User request (2026-11-01): clicking the FX button of a channel other than the one currently open must reuse the same Channel Settings window and switch its contents; insert/send rows can be much shorter to match their text; opening the VST editor is confusing - there is no single open/close button, clickable areas are not obvious, the chevron and the plug-in name text should both always open the plug-in list, and a second button beside the power button (an eye) should show/hide the editor, including reopening it after it was closed with its own X.

## Scope

- R1 Retarget in place. `NativeWindowState::request_channel_settings` only starts a new window epoch when no window is open, so choosing another channel (mixer FX button, track header, inspector) reuses the existing viewport and updates its contents. Focus is still requested so the window comes forward.
- R2 Compact racks. Insert and send slots use a fixed one-line pitch (23 px row, 25 px pitch) instead of stretching to fill the panel. Each rack starts with one row, and a "+ Add Insert"/"+ Add Send" row below the used rows opens another, up to 24 rows each. The slot number, power button, optional view button, chevron and name share one centre line; plug-in names are right-anchored and send destinations are right-anchored too.
- R3 Clear click targets. The chevron and the name are one button: clicking anywhere between them always opens the plug-in list for inserts (or the destination list for sends) and shows a hover highlight. Empty rows open the add list from the same place.
- R4 One editor control. The plug-in editor is opened from a dedicated eye button beside the insert power button; the eye is lit while that insert's editor is shown, closes it when clicked again, and reopens it after the plugin's own X closed the window. The eye only appears for VST3 inserts whose plug-in reports an editor.
- R5 No regression to bypass, insert replacement/removal, send routing, legacy-compressor messaging, or the VST3 parameter fallback window.

## Architecture

Rust UI/state work in `channel_settings.rs`, `app.rs` (window state and panel state) only. The engine, project format and plug-in hosting are unchanged. `ChannelSettingsUiState` carries a mirror of the open editor (`vst3_editor`), the slots whose plug-in has an editor (`vst3_editor_available`), and a close request (`vst3_editor_close`); the app resolves the request against `vst3_editor_window`. No new dependencies.

## Checks

- `an_open_channel_settings_window_retargets_in_place`: an open window keeps its epoch and follows the bus/channel change; a closed window still gets a new epoch.
- `insert_and_send_rows_keep_one_text_line_and_share_the_chevron_centre`: label, power, chevron and (when present) eye share the row centre; the chevron and name form one hit region; the label keeps usable width at 146 px; eight rows plus the header stay compact.
- `racks_start_with_one_row_and_grow_to_the_cap`: one visible row by default, used slots always shown, and both racks cap at 24 rows.
- `a_vst3_insert_eye_opens_and_closes_the_editor`: clicking the eye requests the editor, and a lit eye requests the close.
- Existing insert/send, channel-window lifecycle and EQ layout checks.

## Handoff

Actual implementer: DeepSeek V4.1 Flash; baseline `cb5a8c6` plus local work. Insert and send racks are single-line rows; the plug-in list opens from both the name and the chevron with hover feedback; the eye sits beside the power button, reflects the open editor and toggles it. Window retargeting now keeps one viewport while the channel changes.

Verification: `cargo test -p opentrack --bin opentrack` passes 194 with the same 14 pre-existing `ruler_interaction_tests`/`sample_editor` failures reproduced on the unmodified commit; formatting and diff checks pass. Running-app review of row density, hover feedback, eye state and editor reopen remains pending; desktop/hardware checks were not performed. Local-ASIO release package SHA is recorded in `docs/workflow/current.md`.

Remaining gaps: the eye is only shown when the hosting runtime already reports an editor for that instance; a plug-in that fails to load, or one whose editor is unavailable, has no eye. Device-bus inserts use the bus runtime and are covered by the same panel.
