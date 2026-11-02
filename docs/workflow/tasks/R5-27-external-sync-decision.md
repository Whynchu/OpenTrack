# R5-27 — External sync decision

- Status: accepted (decision record; no implementation)
- Scope version and requirement IDs: revision 5, step 27
- Desired observable outcome: a recorded decision on external clock/sync support and the verification path it needs.
- Reference evidence: `MIDI > SYNC` is a static transport label; no clock-source handling exists in `opentrack-audio`; the engine advances from its own sample clock.
- Dependencies and decisions needed: hardware clock source and transport, plus a MIDI output path for MTC/clock (see R5-26).
- Code scope and architecture boundaries: none.

## Decision

External sync is not implemented. A bounded implementation would need: a clock source choice (MIDI clock, MTC, or word clock), sample-accurate alignment between the external clock and the audio callback (including drift and start/stop edge handling), and a way to report lock loss. Every one of those needs a physical clock source to verify; without one, "sync" would be a label that silently does nothing.

`Transport > Use External Sync` therefore stays unavailable, and the transport's `SYNC`/`INT.` labels are presentation only. Revisit after R5-26's MIDI output exists and a clock source is available.
