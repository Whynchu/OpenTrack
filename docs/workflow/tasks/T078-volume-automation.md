# T078 - Track volume automation contract

Status: persistent point model and curve evaluation implemented; playback, editing, and writing remain unimplemented. This is the first slice for roadmap steps 17-20. Author: GPT-6.1 Terra.

## Current state

Tracks persist `automation_read` and `automation_write`, both defaulting to false in old projects. The arrange view can reveal a lane and show R/W controls. Its parameter popup only labels Volume; there are no persisted points or audio consumers. Mixer, inspector and Channel Settings share the track R/W flags. Channel fader gain is stored as a linear value from 0.0 (mute) through 2.0 (approximately +6 dB); visual fader taper lives in the app.

## First deliverable: read and play track volume

- A lane belongs to a stable track ID and a typed target. The first supported target is track volume; do not silently treat an unknown target as volume. Reserve other targets for later versions. Device-bus and plug-in parameter automation remain unavailable until explicitly implemented.
- Store finite, nonnegative positions in quarter-note beats and finite linear gains in the same 0.0 through 2.0 range as the fader. Points must be strictly ordered with no duplicate beat positions. Validation caps each track at 16 lanes, each lane at 100,000 points, and the project at 1,000,000 points; duplicate target lanes are rejected.
- Persist empty/missing lanes as no automation. Existing OPR/OTP/RWP projects must open without changing channel gain or enabling read/write. Native format version 27 adds the lanes and preserves existing content; earlier versions migrate on read. Builds through version 26 reject version 27 saves rather than silently discarding points. Unknown targets must either be preserved as opaque data or explicitly rejected on import; never discard them silently.
- With Read off or no points, use the existing channel gain. With Read on and at least one point, hold the first value before the first point, interpolate linearly in **linear gain** between adjacent points, and hold the last value after the last point. A zero value is true silence. This gives deterministic behavior at zero; the fader UI may continue using its existing dB-based taper.
- Evaluate against the same project beat and sample clock used by playback, including seeks, stop/start, loop/skip boundaries, and tempo changes. A seek or loop resets any cached search cursor. Clamp or validate before publishing; the audio callback must not allocate, lock, read UI state, or mutate the project. The selected automation value replaces the source channel fader gain before post-fader sends; pre-fader sends remain independent of it. Mute and solo still gate output.
- Playback must not rewrite the stored base fader gain or mark the project dirty. The displayed fader may follow a read-only engine snapshot; a manual gesture must remain responsive and must not create automation unless Write is enabled. Offline bounce must use the same evaluation and timing as playback.

## Editing and write behavior for the later slice

- The existing Volume lane is the entry point. Add, select, move and delete points with explicit project commands; snap timing to the active project grid unless the user invokes the established fine-adjust modifier. Moving a point must not cross or duplicate its neighbors. One drag is one undo transaction; cancel restores the original curve. The lane must not steal audio-event gestures outside its reserved area.
- Write is not a claim of Touch or Latch modes. In the initial bounded write workflow, capture only explicit fader movement while transport is playing and Write is enabled. A gesture records timestamped values from the engine clock into a bounded nonblocking queue; do not write project data from the audio callback. Preserve points outside the touched interval and define boundary values before replacing any interior points. Commit the whole gesture as one undo action; cancellation or queue overflow must leave the previous curve recoverable and report the failure. Loop crossings must be handled as separate timeline intervals, never silently overwrite untouched material.
- Read and Write are independent persisted flags. During a write gesture, manual control is audible immediately; after the gesture, Read resumes the recorded curve when enabled. With Write off, moving the fader does not record points. With transport stopped, editing the fader changes the base value only. Touch/Latch, plug-in automation, multi-parameter lanes and external controller mapping remain future work and must stay disabled or clearly identified until delivered.

## Required evidence

1. Completed for the model: legacy and version 27 round trips, malformed/oversized lanes, unknown targets, point ordering and limits. Core suite and all 17 project-file checks pass.
2. Exact sample-boundary evaluation before, on, between and after points; mute, loop, skip, seek, tempo change, and plan replacement; identical results in live and offline render paths.
3. Pre/post send interaction, grouped tracks, normal fader and mute behavior, undo/redo and save/reopen.
4. Headless gesture tests for point editing, write start/stop, cancellation, queue overflow and loop crossing; actual in-app visual and audio review, including hardware and hosted plug-ins.
5. Callback timing and allocation checks on representative projects. Do not claim write or live acceptance based only on source presence or compilation.
