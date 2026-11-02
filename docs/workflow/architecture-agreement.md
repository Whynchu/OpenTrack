# Architecture agreement

This specifies the direction for incremental work, not a claim that all boundaries already exist. Audit the touched code before selecting an extraction. Preserve the current Rust crate structure and evaluate new dependencies before introducing them.

| Responsibility | Owner / constraint |
| --- | --- |
| Persistent project state | opentrack-core: stable track/event identity, musical time, ranges, parameters, versioned serialization; no screen coordinates |
| Editing operations | Core commands or an explicit application command layer: one mutation path, undo/redo, dirty state, validation; Project/Inspector/Mixer use the same track identity and commands |
| Gesture interpretation | App timeline interaction module: pointer input, capture, hit zones, viewport transform, cursor feedback; emit explicit seek/zoom/range actions |
| Rendering | App views: shared state in, commands out; avoid independent copies of playback or track state |
| Transport execution | opentrack-audio: sample clock, audible seek/cycle/skip execution; UI reads an engine transport snapshot during playback |
| Persistence and platform services | Explicit adapters for files, device APIs, windows, plug-in ABI; keep platform specifics outside domain calculations |
| Channel routing | opentrack-core `routing`: one typed vocabulary (`InputRoute`, `OutputRoute`, `Speaker`) parses persisted spellings, validates against the project and feeds every consumer; the app resolves it to the playback plan in one place. A route the engine cannot play is rejected or reported, never silently replaced by the master |
| Crash and discard recovery | Persistence: the recording journal, WAV header repair and the recovery autosave live in their own modules and are applied through the normal project-history path; the UI only decides when to ask |

## Invariants to establish as features are touched

- One documented conversion path between musical time, samples, and viewport pixels. State units explicitly. Grid snapping is independent of display-label density.
- Horizontal seeking and vertical zoom can both update within one normal ruler gesture. The top range-drawing zone is a separate hit target. Zoom preserves its specified anchor; edge scroll uses elapsed time and current viewport geometry.
- Loop/skip state is persistent project data where specified; the audio engine executes boundaries, including boundaries within an audio block. A moving UI marker alone is not playback implementation.
- Audio callbacks perform no blocking file I/O, unbounded allocation, GUI access, or waiting on UI/plug-in processes. Communication and queues have bounded behavior; timing and discontinuity behavior need tests and listening checks.
- Editing commands distinguish content edits from transient selection/hover/view activity. Save/dirty-state semantics must be specified, not inferred from any UI change. Save/reopen and undo/redo cover new persistent operations.
- Track controls use shared per-track state. UI toggles with no processing consumer are explicitly incomplete.
- Schema changes include compatibility/default handling for supported .opr/.otp/.rwp inputs. CPR compatibility requires independent evidence.

Use targeted pure tests for time mapping, snap boundaries, range direction, adaptive ticks, and transport block boundaries; integration tests for commands/persistence; offline audio checks where applicable; live interaction and hardware checks for what headless tests cannot establish. Do not replace useful behavior delivery with a speculative framework rewrite.

## Routing invariant (2026-11-01)

A route is a value, not a string. Persisted spellings stay byte-compatible with
existing projects, but they are parsed once by `opentrack_core::routing` and
every consumer (send rack, group-cycle check, live-monitor tap, playback plan)
reads the parsed value. `Project::validate_routing` reports unknown routes,
dangling or self group targets, feedback loops and mono inputs wider than the
capture bus. Reading a project stays permissive so an affected file still opens
and can be re-routed; writing refuses it. A route the engine cannot honor must
never fall back to the stereo master, because that hides a routing error behind
audible-but-wrong playback.

## Recovery invariant (2026-11-01)

Recording and unsaved-project recovery are persistence concerns. The recording
journal is written before any take file exists and settled only once the take
belongs to the project; a WAV header that was never finalized is repaired from
the file's real length, never guessed. A recovery autosave is written through the
same validated atomic save path as a save and is removed by a completed save or
an explicit discard. Restoring either kind of state goes through the normal
project-history path, so it is one undo step and never bypasses validation.

## Sample Editor invariant (2026-10-25)

Rust owns source/event identity, edit commands, analysis jobs, note segmentation, warp/pitch intent and rendering publication. Cubase-style Sample Editor is the primary visual/interaction reference; HachiTune/PitchNet are optional processing studies only. Pitch detector output (timestamp, F0, confidence/voicing) is distinct from editable note regions and synthesis. Never infer working DSP from note rectangles. Neural inference may sit behind an evaluated ABI/runtime adapter; no C++/JUCE host replacement. Preserve source files; derived audio/caches are versioned by source/edit/model identity and built away from callbacks. Closing/reopening projects, undo/redo, cancellation and stale-result rejection are acceptance requirements.

T025 media invariant: persistent pool membership and arrange placement are separate. Destructive user media removal is a prepared project+filesystem command with preflight, reversible recovery and no-overwrite undo; ordinary removal never infers file disposal. Recording-creation undo and explicit deletion have opposite transfer direction. Saved-project discard must retain required recovery beyond snapshot trimming. Editor grid/view state is separate from Project musical time; one native cursor hotspot uses the committed snapped/sample-aligned cut coordinate. Project-rate changes retain source rates/durations and confirm mixed-rate playback handling.
