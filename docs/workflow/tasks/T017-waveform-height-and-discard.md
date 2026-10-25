# T017 - Full-height waveforms and Discard recording cleanup

- Status: review_pending; publication explicitly authorized by user.
- Actual implementer: GPT-6.1 Sol.
- Specification: waveform amplitude uses the full event/lane height; closing a project and choosing Discard deletes recordings made since its last save.
- User evidence: T016 waveform appearance was positively reviewed, with the explicit full-height correction. This is not acceptance of unexercised filesystem/hardware behavior.

The shared waveform renderer now uses the full event height with one logical point of amplitude padding per channel. Names overlay the waveform instead of reserving a blank 13-point header. Signal values retain their actual amplitude; this does not normalize quiet recordings. Both completed and growing mono/stereo events share this renderer.

Project history tracks the set of recorder-created files since the last successful save independently of the bounded undo queue. Discard first restores saved-document recording references, then transactionally removes unsaved take files and clears their recovery history. Imported files and saved recordings are excluded. This also works after a recording step ages out of undo history. Discard restores the saved project state before attempting the next project action, so a failed Open cannot leave the discarded events pointing at removed audio. File-operation failure reports an error and prevents closing. New take names reserve paths needed by undo/redo and saved-document recovery, preventing a new take from colliding with an undone saved recording.

Verification: 108 workspace tests pass with local-asio (58 app, 39 audio, 11 core). The waveform mesh test passes with the expanded geometry; three new filesystem/name-allocation regressions cover Discard exclusions, history trimming and reserved take names. Standard WASAPI workspace check passed before the final platform-independent naming guard. x64 local-asio release built successfully using build.ps1 -Architecture x64 -LocalAsio.

Identity: x64 local-asio EXE SHA-256 88482EEE7B248AF7F32A36D7CBD69CF7921484B101976324A0900BE23BD3666F; crates/opentrack-app/src/app.rs: 90C36949FB8F19BED275828498D5BF0B797005930259BE15B393AC5DAC0DDC98; crates/opentrack-app/src/recording_undo.rs: 704FABD3573A1F026C98C23A9EA815DC3DB84B874E755113806E33F80135D43D; crates/opentrack-app/src/waveform_view.rs: 17207DDAE9247E43400402AC56FE7449336B6F653493647C5CB67D3E2C362F57 No GUI launch, screenshot or live recording/discard check was performed; EXAMPLE remains untouched. User visual/runtime acceptance remains pending.

Review: record a disposable take and inspect full-height waveforms. Close and Discard: the new take files must disappear. Repeat after Save: saved recordings and imports must remain. Recheck Undo/Redo and a new take after undoing a saved recording. Stop for feedback before another milestone.

The user authorized pushing the current accumulated source checkpoint, including the earlier uncommitted transport/audio/device/bus/project-folder work and crate rename. Builds and saved projects remain outside Git; prior accepted Mixer structure and README showcase are preserved. Publication does not establish full parity.
