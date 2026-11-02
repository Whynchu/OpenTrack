# T084 — Make the automated suite green and measure the arrange layout

- Status: review_pending (source + automated checks complete; no runtime gate)
- Scope version and requirement IDs: cross-cutting harness debt recorded against
  steps 25–29.
- Desired observable outcome: `cargo test -p opentrack` fails only on a real
  regression, so future slices have a trustworthy signal on Linux. Also fixes the
  one product bug the failing tests were hiding.
- Reference evidence: the baseline was 15 failures, all "pre-existing". They were
  not pre-existing defects in the product; they were tests that encoded one
  platform's pixel scale, plus one genuine interaction bug.
- Dependencies and decisions needed: none.
- Code scope and architecture boundaries:
  - `app.rs` test fixtures — `pixels_per_beat`, `x_for_beat`, `beat_at`,
    `y_for_track`, `click_row_control`, measured from the ruler the code draws.
  - `app/src/pitch_editing.rs` — anchor strip height bounded to 40% of the note.
  - `app/src/sample_editor.rs` — one test reads the drawn note rect instead of
    hardcoding the editor's header height.
- Acceptance checks (automated today):
  1. Ruler seek/zoom, range handles, rigid range drag, grid controls, automation
     lane rejection, full-project drag, group-drop collapse all pass on Linux.
  2. Crossfade is asserted through the request and `crossfade_selection`, the
     operation the app runs.
  3. A click on a short note's vertical centre is a body click at 12/16/24/60 px,
     the top edge still grabs the anchor, the lower edge stays the split band.
  4. The full Sample Editor split-and-drag test follows the real header height.
- Affected regressions: whole app suite.
- Essential gates: the suite result is the evidence.
- Review owner and milestone stop: none; this is test infrastructure.

## Handoff

- Actual implementer/reviewer model: DeepSeek V4.1 Flash.
- Source identity: commits `c3f08c9` and `3fbc71e`; release
  `builds/x64/OpenTrack` `31853798…`.
- Changes and reasons: hardcoded pixel scales made every layout change a silent
  test break; measuring the drawn ruler keeps the expectation honest. The anchor
  strip covered the middle of a 12 px note, which made body drags anchor moves and
  caused five of the six remaining failures.
- Commands/checks and actual results:
  `cargo test -p opentrack` → 289 passed, 0 failed, 9 ignored (from 244/15).
- Runtime environment and evidence paths: Linux; no runtime gate needed.
- Review findings, rework: the first attempt at the anchor test assumed the note
  rect from constants; the probe showed the editor's header is 11 px taller, so the
  test now reads the drawn rectangle.
- Failures/unknowns/pending acceptance: none. Nine ignored tests remain ignored by
  design (opt-in project fixtures).
- Next ready task: none.
