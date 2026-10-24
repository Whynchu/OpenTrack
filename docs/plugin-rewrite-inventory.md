# Cubase 5 plug-in evidence inventory (development deferred)

This historical inventory is retained as evidence. Bundled plug-in recreation is deferred; 1:1 copies are not required. Future plug-ins are designed individually, potentially studying purchased Waves products when requested. This table is not an implementation queue or part of the transport percentage. It records installed paths and bounded static evidence only. A filename or folder name does not establish a plug-in's controls, signal flow, or behavior.

## Evidence boundary

Source: `../../_analysis-tools/reports/cubase-install.evidence.json`, an REA safe artifact graph scan of the local reference install. The scan examined 2,839 entries, probed 44 files, and read 14,256 bytes. It recorded PE machine type `0x014c` (x86) for each of the 29 DLLs under `VSTPlugins`. A follow-up read-only REA export-table pass established x86 PE format and exports for three `.vst3` modules; those results are listed below. None of the plug-ins were run.

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
| `VSTPlugins/Tools/TestGenerator.dll` | PE x86; REA export reader found `VSTPluginMain` (ordinal 1), Evidence `ev_4708b1f4b90e1a5066e5634fb892c6bdba0cd7b10dde50e6bf6eeb067bc95fc7` | Manual-derived control list recorded; runtime study pending | Original DSP prototype integrated in the track insert chain; parity pending |
| `VSTPlugins/Vocoder/Vocoder.dll` | PE x86 | Not started | Not started |

## VST3 paths

| Installed path | Static result | Behavioral study | Rust rewrite |
| --- | --- | --- | --- |
| `vst3/Cubase Plug-in Set.vst3` | PE x86 (32-bit); exports `ExitDll`, `GetPluginFactory`, `InitDll`; SHA-256 `5302badb38ab2a54cd5c243fdb23d360a965a1f75b1bf0773149e93651e99662`; Evidence `ev_dab388ccec06490f47f00319140a48b547d2ecb4ecf4d0ea29f0fc6c707aa0fd` | Not started | Not started |
| `vst3/Groove Agent ONE.vst3` | PE x86 (32-bit); exports `ExitDll`, `GetPluginFactory`, `InitDll`; SHA-256 `d766e3325c0d07726779b87ce5d6f4ed7d7bf3da669b06f13aa603e40ec1644a`; Evidence `ev_20aac02d8c8a270637aa592b80ca96186f98fd0933b7964d13cf7f7e609a6310` | Not started | Not started |
| `vst3/SynthEngine.vst3` | PE x86 (32-bit); exports `ExitDll`, `GetPluginFactory`, `InitDll`; SHA-256 `9447f7d87e803e7b86495981b27fb7ffc3ae34d6223e700c5b39ceac487205ad`; Evidence `ev_d20a57ea6884ac23b21b76b4eea0983109dd17440af44dfabb6166550f34b268` | Not started | Not started |

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
