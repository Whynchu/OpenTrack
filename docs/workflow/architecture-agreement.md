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

## Invariants to establish as features are touched

- One documented conversion path between musical time, samples, and viewport pixels. State units explicitly. Grid snapping is independent of display-label density.
- Horizontal seeking and vertical zoom can both update within one normal ruler gesture. The top range-drawing zone is a separate hit target. Zoom preserves its specified anchor; edge scroll uses elapsed time and current viewport geometry.
- Loop/skip state is persistent project data where specified; the audio engine executes boundaries, including boundaries within an audio block. A moving UI marker alone is not playback implementation.
- Audio callbacks perform no blocking file I/O, unbounded allocation, GUI access, or waiting on UI/plug-in processes. Communication and queues have bounded behavior; timing and discontinuity behavior need tests and listening checks.
- Editing commands distinguish content edits from transient selection/hover/view activity. Save/dirty-state semantics must be specified, not inferred from any UI change. Save/reopen and undo/redo cover new persistent operations.
- Track controls use shared per-track state. UI toggles with no processing consumer are explicitly incomplete.
- Schema changes include compatibility/default handling for supported .opr/.otp/.rwp inputs. CPR compatibility requires independent evidence.

Use targeted pure tests for time mapping, snap boundaries, range direction, adaptive ticks, and transport block boundaries; integration tests for commands/persistence; offline audio checks where applicable; live interaction and hardware checks for what headless tests cannot establish. Do not replace useful behavior delivery with a speculative framework rewrite.
