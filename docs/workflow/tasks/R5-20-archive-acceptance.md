# R5-20 — Track Archive round-trip acceptance

- Status: accepted
- Scope version and requirement IDs: revision 5, step 20
- Desired observable outcome: exporting and re-importing a project keeps folder hierarchy, FX returns and typed sends with remapped ids, stores shared media once, and never modifies the source media.
- Reference evidence (IDs/paths): `acceptance_round_trip_keeps_folders_fx_sends_and_dedupes_media` in `crates/opentrack-app/src/app/track_archive.rs`.
- Dependencies and decisions needed: R5-19 (accepted); the archive format itself is revision 4's.
- Code scope and architecture boundaries: one acceptance test; no product change.
- Acceptance checks: 3 tracks and 1 media file exported from a folder/audio/FX project with two clips sharing one file; re-import into a nonempty project remaps ids away from existing tracks, preserves `parent_folder_id`, keeps the `FX:<id>` send pointing at the remapped FX return, reports no disconnected routes, points both clips at one existing copy, and leaves the original media byte-identical.
- Affected regressions: none.
- Essential gates: hierarchy, routing, media dedupe, source preservation.
- Review owner and milestone stop: self-review.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash (opencode-go/deepseek-v4.1-flash); self-review only.
- Source identity: commit carrying this packet.
- Changes and reasons: added the acceptance test; no product change was needed, which is itself the result of the pass.
- Commands/checks and actual results: `cargo test -p opentrack acceptance_round_trip` → passed; app suite 570 passing with the four known device failures.
- Runtime environment: Kubuntu Linux; offline.
- Ledger IDs updated: inventory regenerated.
- Review findings: none; the export already deduplicated shared media and rewrote typed sends in revision 4, and the pass confirms it end to end.
- Failures/unknowns/pending acceptance: a very large archive and a Windows-host export are not exercised.
- Next ready task: R5-22.
