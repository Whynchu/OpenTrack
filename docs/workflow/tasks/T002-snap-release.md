# T002 — Snap the transport cursor on ruler release

Status: draft; depends on T001. Scope: transport-v1, seek-pointer and snap-release. Preferred implementer: Luna Max after specification; interaction reviewer: Sol or stronger.

Outcome: normal left drag seeks continuously under the pointer; release snaps according to the confirmed active grid/snap settings. Snap behavior is independent of ruler label stride. Preserve simultaneous vertical zoom and correct mapping after viewport scroll.

T001 must fill in: exact normal-ruler hit target; snap enabled/disabled rules; rounding/tie and project-start boundary; source seams; fixture setup and expected positions. These are readiness dependencies, not permission questions for the user when code/reference evidence can answer them.

Implementation scope: touched timeline coordinate/gesture code and shared seek command; necessary pure helper/tests. Preserve Mixer, range-drawing zone, middle pan, scrollbar, pinned header and right-click menu. Do not add a ruler tooltip.

Acceptance cases: grid on/off; exact grid point; off-grid release; release after horizontal and diagonal drag; release after edge scroll; project-start clamp; zoom levels whose labels skip bars; release outside original hit rectangle with captured drag. Verify expected musical position and marker, using the finalized T001 semantics. Check that view-only/seek activity does not create an unintended save prompt. Test pure boundary calculations and live pointer interaction; tie results to build/source identity.

Run the existing build before edits and x64 build afterward with OpenTrack closed. One app instance during review; close when finished. Use a disposable EXAMPLE copy if project content can change. Update ledger and report. Stop for independent interaction review; user milestone acceptance remains pending until the transport slice is demonstrated.
