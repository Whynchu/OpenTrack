# R5-21 — Interchange decision

- Status: accepted (user decision, documentation only)
- Scope version and requirement IDs: revision 5, step 21
- Desired observable outcome: one recorded decision about other project formats, referenced from the remaining menu rows. README revision-5 step 21 and the decisions section carry it.
- Reference evidence: user direction, 2026-11-02.
- Dependencies and decisions needed: none; this *is* the decision.
- Code scope and architecture boundaries: none.
- Essential gates: none.

## Decision

Compatibility with other project file formats (CPR, OMF and similar) begins only once OpenTrack has full feature support for the content those formats carry. Until then no adapter, converter, alias or placeholder dispatch is written, and nothing about Cubase compatibility is claimed in the product or the docs. The remaining rows stay unavailable with that reason attached, so a future adapter starts from a known feature baseline rather than mapping incomplete concepts.
