# T024 - Editor transport, cut guide and presentation correction

Date:2026-10-25. Status:review_pending. Implementer:Codex current session; no model switch/subagents. User approves continued direction but excludes AudioWarp, asks where sample rate/bit depth live, requests OS tool artwork, light-gray snapped cut/transport guides, exact shared transport, mono/color waveform and improved reference layout.

## Boundaries / reference

Retain Rust and accepted Mixer; no neural/model dependencies or additional audio processing. Primary existing captures SE01-SE07 are indexed in [Sample Editor study](../../reviews/sample-editor-study.md), with user requests specifying this correction. No new reference/desktop interaction. App was detected running, and the user confirmed closing it before build. No app/hardware stream launched; EXAMPLE untouched; leave app closed. No current-app screenshot or visual parity claimed. Window conditions/current image remain pending live review. No push requested.

Sample rate and bit depth already work at Project > Project Setup (Shift+S): Sample Rate and Record Format (PCM16/PCM24/Float32). These affect new recording format; existing files retain their recorded format. This was source-confirmed, not a new conversion feature.

## Implementation

AudioWarp removed from editor and excluded permanently in AGENTS/study/current workflow. Pitch & Time remains planned/unavailable: detection, editable notes, correction and stretching are not enabled by this polish pass.

Project transport is now light gray, separate from recording/error red. Both Project scissors hover and Sample Editor scissors draw a light-gray cut guide at the snapped, event-relative sample-aligned location. Domain AudioClip::cut_offset_seconds is shared by preview and actual split, including rejection of event endpoints. Hover previews do not seek the engine or dirty the project. Actual transport remains distinct from the hover guide.

Sample Editor receives the same mutable Project transport beat; there is no second editor playhead state. It paints that position in main waveform and overview, formats its readout using the same Project transport formatter and converts through event start/tempo, independent of source trim. Ruler click/drag seeks the same state and engine; release uses the same snap interval. Playback follows the cursor when it leaves a zoomed view; overview click/drag navigates the view. Selection/zoom/seek do not dirty project content.

Editor presentation: compact shared toolbar icons and timebase/tempo/signature/snap/cursor fields, six-column event information strip, full-width colored overview, left event-title/Definition/Pitch & Time/Range hierarchy, adaptive global musical ruler (seconds optional), event start/end labels and colored waveform. Source metadata is read independently of asynchronous waveform readiness. Display channels use the smaller valid source/track count: a mono source in a stereo-width lane is no longer drawn twice, mono tracks draw one lane, stereo/stereo draws two. Track color carries through title, background and waveform. The previous no-edit guard remains; legacy quiet gains are not clamped merely by opening.

Tool artwork: Windows arrow resource is loaded using LoadCursorW(IDC_ARROW), rendered into a cached texture through a private GDI DIB, with copied bitmaps/DC restored/released. Cut/pencil/zoom use the installed Segoe Fluent Icons font, falling back to installed Segoe MDL2 Assets; neither font/cursor binary is copied into the repository or distribution. Object-hover uses native arrow, dragging native move. Windows has no stock scissors/pencil cursor identifier, so native crosshair plus Windows icon-font tool badge is used; the handmade ruler pencil was removed. Other unrelated control symbols remain original. Existing original-vector fallback remains for unavailable native/font resources; actual Windows machine has both fonts and native arrow test succeeds. Resource/cursor-theme artwork is read at launch/context initialization, not live refreshed during this slice.

Primary API/font documentation checked: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadcursorw and https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-fluent-icons-font . No new crate dependencies. OS boundary remains isolated in system_tool_icons.rs.

## Coverage / current evidence

| Requirement / reference | Source | Expected action | Current match status |
| --- | --- | --- | --- |
| Light-gray snapped cut guide / user, SE07 | audio_events.rs; shared core cut_offset_seconds | Hover scissors between grid points, then click | Actual-input and coordinate test passes; live visual pending |
| Shared transport / user, SE01 | sample_editor.rs; show_mixer_window | Open event with nonzero start/source offset; seek ruler; play and switch tabs | Headless paint/seek tests pass; live playback feel pending |
| Mono + track color / user, SE01 | sample_editor.rs; waveform_view.rs; AudioSource metadata | Mono source in stereo track, mono/stereo tracks | Real mono WAV test sees one overview/main waveform, track color; live comparison pending |
| Editor hierarchy / SE01-SE06 | sample_editor.rs | Open editor, inspect left/top/right/bottom, musical/seconds view, zoom | Source implemented; no current image or measured visual parity |
| Windows tool artwork / user | system_tool_icons.rs; icons.rs; theme.rs | Top toolbar, held toolbox, ruler-pencil cursor | Native resource test passes; DPI/theme appearance pending |
| AudioWarp excluded / user | editor + AGENTS/study | Open editor sidebar | Headless absence check passes |

Display resolution/scaling/window placement/theme comparison and current screenshot: not observed in this slice, pending user review. Screenshots remain external to Git. Reference shapes/controls not yet implemented are documented rather than enabled as placeholders.

## Verification and review

144 release workspace tests pass (85 app,44 audio,15 core), no Rust compiler warnings. New checks read a real mono WAV, assert metadata and display count, count one colored main/overview waveform, verify actual shared marker position independent of source offset, actual ruler click changes transport without modifying Project, compare snapped guide with committed event split in both views, and load native Windows arrow artwork. Regression tests retain faded split sample identity, unity-sum crossfade, input toolbox, Preferences, Project/ruler/recording/faders and legacy files. x64 local-asio release build succeeds; git diff --check passes. These checks do not establish live visual/audio parity.

Review: inspect Project > Project Setup (Shift+S). Open mono recording from its body, verify colored single waveform and compact panel structure. Compare Project/editor/overview cursor position before and during playback; seek Sample Editor ruler with snap on/off, zoom and pan, then switch tabs. Hover scissors between grid lines and near event endpoints; guide and resulting cut must agree, with no engine seek from hover. Inspect OS pointer/scissors/pencil/zoom artwork at current Windows DPI/theme; test held toolbox and ruler range behavior. AudioWarp must be absent and Pitch & Time still clearly pending. Stop for feedback before analysis/correction stages.

Identity: builds/x64/OpenTrack.exe: B998CE89E013B65B1B147DF3831DB04212BC2310235C7BC90C044DC4DCFFEA7F; crates/opentrack-app/src/app.rs: 6BB3BD8FD5D74D46FF81648E7ECCD1C09AE8C5ACA4B10E9DDBC917DE94DBEF50; crates/opentrack-app/src/sample_editor.rs: D49174FEF268811CC24698D7ECDC472C2A5B7678351F6BFE5CF1CDE5D9016628; crates/opentrack-app/src/audio_events.rs: 4F13AAD2DB31CB9A238E2B58B5DDD457022473A0F274B0FDA258C63F1F2F15E1; crates/opentrack-app/src/system_tool_icons.rs: 94D150DF1473671980FFC3DF4E78B136C227CD28CF5A322D855156C3E9100517; crates/opentrack-app/src/icons.rs: 46F715306B872A52000664B0993BF55AC890E7425CF173569F4D75C832BFBE16; crates/opentrack-app/src/theme.rs: 7E2AE33AE2A9F30AFEBC14BEBCAD2A20A5C81DBC95D8DD244AD9C7B0C3C8E638; crates/opentrack-app/src/waveform_view.rs: 58FC289F06F63AA65B8349A736C98C8BA7AC7E1AC46901A9909957326FFFA757; crates/opentrack-audio/src/streaming.rs: AE2839B18070AF205C6B0712DD385B0C73D955CD09E8C447D270D82B67E425DE; crates/opentrack-core/src/project.rs: AD14BC9B1496EB8783BD53EC8AA21F69C7A81F4C6514D97D02A4079AD4FE394C
