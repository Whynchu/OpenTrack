# Pitch quality diagnostics

This standalone crate compiles the production Rust pitch renderer and pitch detector directly. It does not start OpenTrack, open an audio device, or modify the project/source files.

Run the synthetic quality checks from the repository root:

```powershell
cargo test --offline --manifest-path tools/pitch-quality/Cargo.toml --release --target-dir .build-cache/pitch-quality quality_tests -- --nocapture
```

To also render the saved EXAMPLE Vocal 01 analysis, set explicit read-only input paths first:

```powershell
$env:OPENTRACK_PITCH_SOURCE = 'C:\Users\Samue\Documents\EXAMPLE\Audio\Recordings\Vocal 01.wav'
$env:OPENTRACK_PITCH_EDIT = 'G:\dev\Opentrack\OpenTrack-\builds\x64\T042-audio-comparisons\Vocal01-PitchTimeEdit.json'
$env:OPENTRACK_PITCH_QUALITY_DIR = 'G:\dev\Opentrack\OpenTrack-\builds\x64\T042-quality\candidate'
cargo test --offline --manifest-path tools/pitch-quality/Cargo.toml --release --target-dir .build-cache/pitch-quality recorded_vocal_matrix -- --ignored --nocapture
```

WAV and JSON outputs go under the ignored `builds/x64/T042-quality/candidate/` folder. Metrics cover reconstruction, detected target pitch, note-cut invariance, source tails after time expansion/compression, vowel spectral envelope, unvoiced/transient preservation, stereo balance, octave-analysis glitches, and render preparation time. These checks do not establish perceptual transparency; listen to the current app build with representative user audio before accepting T042.
