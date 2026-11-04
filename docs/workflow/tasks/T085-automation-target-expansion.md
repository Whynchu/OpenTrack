# T085 - Automation target expansion

- Status: source implementation and focused checks pass; live desktop, device listening, and named real plug-in parameter acceptance remain pending.
- User outcome: automate the mix over time for ordinary arranging and mixing, save/reopen that intent, and hear the same automation in live playback and offline exports.
- Scope: Track/Group/FX and device-bus volume/pan/mute, enabled send level/pan, and stable VST3 parameter targets; preserve existing behavior and old projects.
- Behavior: Read evaluates on the project clock. Write records explicit control gestures during playback through bounded, undoable capture. VST3 events use bounded sample-offset queues. Insert identity and parameter ID prevent rack reorder or display-name changes from retargeting a lane.
- Persistence: OPR 46 stores stable insert IDs and the next-ID counter. Files through version 45 migrate; legacy or duplicated missing insert IDs are assigned before validation/save. Unsupported or absent plug-ins leave lanes as unresolved project data.
- Runtime constraints: callback evaluation is bounded and allocation-free; UI, I/O, and project mutation stay off the audio callback. Hosted VST3 points are bounded to a fixed event array and bounded VST3 parameter queues. Bus Read uses project beat time; input capture derives project time from capture frame origin.
- Editing: Automation Panel offers focused plug-in parameter lanes. Mixer/Channel Settings controls follow Read curves and Write captures supported gestures. Existing track/send arrange lane editing and snap/undo semantics remain.
- Boundaries: third-party plug-ins may ignore automation queues or have parameter-specific behavior; no universal compatibility claim. Hardware clocks are not synchronized to the project transport beyond the configured input capture origin. FX/device and plug-in automation needs live application review.

## Delivery sequence

1. **Source delivered, runtime review pending:** Track Pan/Mute, Group Read, and Send Level/Pan with routed live/offline behavior and Write capture. Send-slot targets are remapped during send deletion; they are still slot-addressed rather than persistent send entities.
2. **Source delivered, focused verification pending:** FX return channel volume/pan/mute and plug-in parameters; Stereo In/Out bus channel automation; Read-follow and Write controls; OPR 46 stable insert IDs and migration.
3. **Required before acceptance:** run focused core/audio/app/VST3 checks; inspect save/reopen identity and Write undo; review the UI in OpenTrack; exercise parameter automation with a real named VST3; listen on available output and verify capture monitoring; update evidence and report remaining compatibility limits.

## Initial implementation record (2026-11-03)

- Added stable `InsertEffect.id` and project `next_insert_id`; loaded legacy projects and saved new/duplicated inserts receive nonzero unique IDs. VST3 runtime instances are keyed by `(channel_id, insert_id)` while editor calls resolve the current rack slot.
- Added `PluginParameter { insert_id, parameter_id }` lanes. Mixer plug-in sliders follow Read and capture Write gestures through project history. Automation Panel can focus a hosted parameter lane.
- Added a format-neutral hosted automation event API. The audio callback builds a fixed 128-event array, including in-block lane points and sample offsets. VST3 `IParameterChanges` adds bounded sample-offset points; parameter queue count and per-parameter point count remain fixed. Unsupported processors can ignore automated processing.
- Added FX channel volume/pan/mute and insert parameter validation. Device Stereo In/Out Read evaluates volume/pan/mute against project sample time in capture/monitor and output processing. Channel controls expose Read/Write; mute/pan/fader presentation follows automation.
- Updated OPR to 46, README and native-format description. Earlier formats migrate through 45. The project format notes that older application versions reject v46.
- Actual model: Codex GPT-6. No independent review. Source/build evidence is not user acceptance.
- Verification: `cargo check --workspace` passes; `cargo fmt --all -- --check` and `git diff --check` pass. `cargo test -p opentrack-core automation --lib` passes (18); app automation filter passes (26); project-file group passes (36); audio automation filter passes (12), including send routed live/offline parity; the dedicated device-bus read test passes; VST3 parameter filter passes (5, 1 ignored); hosted scheduler offset and loop-wrap tests pass. No full workspace test run was performed.
- Pending: real plug-in automation behavior, all plugin lane UI/follow/undo paths, live/offline parity across buses, device listening/capture, desktop review, and user acceptance.
