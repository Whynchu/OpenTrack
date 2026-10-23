# Cubase 5 plug-in rewrite inventory

This is the source inventory for OpenTrack-'s original Rust plug-in rewrites. It records installed paths and bounded static evidence only. A filename or folder name does not establish a plug-in's controls, signal flow, or behavior.

## Evidence boundary

Source: `../_analysis-tools/reports/cubase-install.evidence.json`, an REA safe artifact graph scan of the local reference install. The scan examined 2,839 entries, probed 44 files, and read 14,256 bytes. It recorded PE machine type `0x014c` (x86) for each of the 29 DLLs under `VSTPlugins`. It also listed three paths ending in `.vst3`, but those module contents were not selected for probing; their architecture is unknown from this report. None of the plug-ins were run during that scan.

The category names below reproduce their parent folders. They are inventory paths, not verified functional classifications. `Q.dll` appears under two distinct paths; the scan does not establish whether the files are identical.

## VSTPlugins DLL paths

| Installed path | Static result | Behavioral study | Rust rewrite |
| --- | --- | --- | --- |
| `VSTPlugins/Delay/DoubleDelay.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Delay/ModDelay.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Distortion/DaTube.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Dynamics/DeEsser.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Dynamics/MIDI Gate.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Filter/NuendoEQ2.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Filter/Q.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Filter/StepFilter.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Filter/Tonic.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Modulation/Metalizer.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Modulation/RingModulator.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Modulation/Tranceformer.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Other/BitCrusher.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Other/Chopper.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Q/Q.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Restoration/DeClicker.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Restoration/DeNoiser.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Restoration/Grungelizer.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Reverb/Reverb A.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Reverb/Reverb B.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Surround/Mix6to2.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Surround/MixConvert-Control Room.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Surround/MixConvert.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Surround/SurroundPan.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/SurroundDither/SurroundDither.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Tools/MultiScope.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Tools/SMPTEGenerator.dll` | PE x86 | Not started | Not started |
| `VSTPlugins/Tools/TestGenerator.dll` | PE x86 | Manual-derived control list recorded; runtime study pending | Original DSP prototype integrated in the track insert chain; parity pending |
| `VSTPlugins/Vocoder/Vocoder.dll` | PE x86 | Not started | Not started |

## VST3 paths

| Installed path | Static result | Behavioral study | Rust rewrite |
| --- | --- | --- | --- |
| `vst3/Cubase Plug-in Set.vst3` | Listed; module not probed | Not started | Not started |
| `vst3/Groove Agent ONE.vst3` | Listed; module not probed | Not started | Not started |
| `vst3/SynthEngine.vst3` | Listed; module not probed | Not started | Not started |

The adjacent `vst3/Groove Agent ONE.bin` file is also present in the inventory. Its relationship to the `.vst3` path and its contents have not been established, so it is not counted as a separate plug-in here.

## Manual-described behavior

The local Operation Manual says the detailed plug-in descriptions are in a separate Plug-in Reference. The local reference tree did not contain that PDF. Steinberg's official Cubase 5 support page lists `Plug-in_Reference.pdf` in the Cubase 5.0 documentation ZIP; the behavior notes below come from an online copy of the 114-page Cubase 5.0 reference. The installed tree appears to be a later Cubase 5 build, so these are manual-derived expectations to confirm against the running reference application.

- **TestGenerator**: the manual lists sine, square, sawtooth, triangle, white noise, brown noise, and pink noise; frequency 1–20,000 Hz; and gain up to 0 dB. It describes adding and activating the plug-in as an audio-track effect to generate a signal. The oscillator and noise algorithms, defaults, channel layout, and precise output behavior are not specified in the text.
- **MIDI Gate**: requires both audio and MIDI input. MIDI notes trigger its gate; the manual lists Attack, Hold, Release, Note To Attack, Note To Release, Velocity To VCA, and Hold Mode. This is a useful host-design case because one instance consumes audio and MIDI together.
- **Mix6To2**: the manual describes mixing up to six surround channels to stereo with per-channel levels and speaker-layout presets. It says this is a mixer, not a psychoacoustic surround simulation.
- **DaTube**: described as tube-amplifier emulation with Drive, Bass, Middle, Treble, Presence, and Volume controls; the DSP model is not specified.
- **SMPTEGenerator**: described as a time-code signal source, not an audio effect. It has free-run and transport-linked operation, which depends on project transport and frame-rate state.

The set also includes different host cases: a MIDI-triggered audio effect, a generated audio signal, a time-code source, a visual analyzer, and multi-channel processing. This points to typed audio/MIDI buses and non-audio plug-in roles in the eventual host; a stereo audio-effect-only interface would not cover the observed set.

The manual confirms user-facing intent and broad parameters; it does not establish numerical DSP parity. The Rust `TestGenerator` in `crates/workstation-audio/src/plugins/test_generator.rs` implements the documented waveform list and frequency/gain limits with original oscillator and noise algorithms. It is connected to the per-track insert chain and project state. Its output, defaults, and noise spectra have not been compared with Cubase.

Sources: [Steinberg Cubase 5 support page](https://o.steinberg.net/index.php?L=1&id=1782) (official listing of the Cubase 5.0 documentation set) and [Cubase 5.0 Plug-in Reference PDF](https://images.thomann.de/pics/prod/222910_plugineng.pdf) (rehosted copy of the manual).
## Per-plug-in evidence to collect

Before implementing a rewrite, capture the reference plug-in's identity and version, input/output bus layout, parameter names/ranges/defaults, automation behavior, preset and state serialization, latency or tail behavior, and reproducible audio or MIDI input/output observations. Mark each item as observed, inferred, or unknown. Keep reference captures and original binaries outside this repository; put derived measurements and original Rust code here.

TestGenerator is the first chosen prototype because its control list is documented and it exercises an audio-generating plug-in role. This is not a parity claim. Select subsequent plug-ins after inspecting their reference UI and establishing repeatable input/output captures, rather than inferring DSP from filenames.

The Mixer also includes an OpenTrack- Compressor as a general channel effect. It is not a rewrite of a specific Cubase 5 plug-in, and its original linked-peak design is outside the bundled plug-in inventory. To start a bundled rewrite, use the evidence procedure above and capture the selected reference plug-in before implementing its controls or DSP.
