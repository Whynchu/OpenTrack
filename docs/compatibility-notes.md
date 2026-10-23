# Cubase 5 reverse-engineering status

These observations support OpenTrack-, a full Rust reimplementation. Static file observations are not proof of runtime behavior or compatibility. Keep REA evidence and manual-derived behavior distinct from implementation claims.

## REA installation inventory

The saved report at ../_analysis-tools/reports/cubase-install.evidence.json records an REA safe artifact graph scan. It examined 2,839 entries, probed 44 files, and read 14,256 bytes within its configured limits. It reported no application candidates or component classifications. It did identify Cubase5.exe and bundled plug-in binaries as x86 PE files.

This is bounded static inventory, not a runtime trace or an atomic snapshot of all file contents. It does not establish which code Cubase loads, how a plug-in behaves, or whether a binary corresponds to a particular running build. See the limitations in the evidence report.

REA 6.7.0 is installed in the analysis-tools directory. The current `rea doctor` report has no configured Ghidra, Hopper, or IDA provider, and the listed native UI capability is macOS-only on this Windows host. REA has therefore contributed static installation evidence here; no native decompilation or Cubase window screenshot is claimed.

## CPR project files

- The local template corpus contains 37 CPR files. During this scan, all 37 same-named files under `../Templates` matched the installed `C:\Program Files (x86)\Steinberg\Cubase 5\Templates` files by SHA-256.
- All 37 begin with RIFF, followed by NUNDROOT at offsets 8 through 15.
- For all 37 files, interpreting bytes 4 through 7 as a big-endian integer gives the file length minus 12.
- A bounded read-only walk of all 37 files found an outer sequence of blocks beginning at offset 12. Each block is a four-byte tag, a big-endian 32-bit payload length, then exactly that many bytes. Every observed tag is ROOT or ARCH; they alternate, with 4 or 6 pairs per file, and the final block ends exactly at EOF. No alignment padding was needed for these files.
- `cpr-inspect --blocks <file.cpr>` reports those observed outer block offsets and lengths. It does not decode the block payloads or establish the meaning of ROOT and ARCH.
- A read-only scan of all 37 templates found that each small ROOT block is exactly a sequence of two printable ASCII strings, each preceded by a big-endian 32-bit byte length. `cpr-inspect --root-labels <file.cpr>` now reports those strings without interpreting their meaning. The seven observed pairs and number of templates containing each pair are:

  | ROOT strings | Templates |
  |---|---:|
  | `Version / PAppVersion` | 15 |
  | `Arrangement1 / PArrangement` | 37 |
  | `Devices / FAttributes` | 15 |
  | `WindowLayouts / UWindowLayout` | 37 |
  | `ProjectLayouts / FAttributes` | 15 |
  | `Metadata / StMedia::PAttributes` | 37 |
  | `Devices / FMemoryStream` | 22 |

  The paired ARCH payload format remains unknown.
- The files contain readable serialized names such as PAppVersion, PArrangement, MTrackList, MAudioTrackEvent, MMidiTrackEvent, and PPool.
- In `default.cpr`, the decoded `Version / PAppVersion` ROOT strings start at offsets `0x18` and `0x23`. The printable run `PAppVersionARCH` crosses the boundary between the second ROOT string and the next outer `ARCH` tag; it is not one serialized name. That ARCH payload contains the literal `Version 5.0.0`, but its surrounding fields are not mapped.
- `cargo run -p cpr-inspect -- --all-ascii-runs <file.cpr>` now prints each printable ASCII run with its byte offset and length. The offsets help compare templates, but the mode does not decode object records.
- Literal project-version strings include Version 5.0.0, Version 5.1.0, and Version 5.1.1. They appear in 15 of the 37 templates; other files may encode version information differently.
- The evidence suggests a structured, tagged binary format rather than plain XML or a ZIP container. This remains an inference; record boundaries and semantics are not mapped.
- The ROOT labels support the inference that each outer section associates a human-readable section name with a serialized type name. This association has not been confirmed by opening/saving controlled projects or decoding the corresponding ARCH trees.
- A public work-in-progress parser for Cubase versions 2 and 3 also describes a ROOT/ARCH block hierarchy and track records, but those older-version findings have not been validated as a Cubase 5 payload schema. OpenTrack- does not copy or depend on that implementation. [Repository notes](https://github.com/omeriko9/Cubase-Project-File-Reverse-Engineering#readme).
- The public [Steinberg project-file documentation](https://www.steinberg.help/r/cubase-pro/15.0/en/cubase_nuendo/topics/project_handling/project_handling_about_project_files_c.html) describes `.cpr` as the central project document and says it references project media; it does not specify the binary record schema. No official format specification has been located in the sources reviewed for this investigation.
- REA 6.7.0's generic `inspect-artifact` command rejected the installed `default.cpr` as an unsupported binary format. The current CPR work therefore uses the Rust inspection utility and direct read-only observations; REA remains useful for the install inventory and native binary evidence when a provider is configured.

### Import investigation

Compare copies of templates with controlled projects saved by the reference application: add one audio track, one MIDI track, a marker, and a plug-in, then record byte-level differences. Map fields into OpenTrack-'s project model, preserve unknown data where practical, and report unsupported content.

Do not write into reference files. Do not claim CPR round-trip or general compatibility from template samples alone.

## Bundled plug-in binaries

- The supplied Cubase5.exe has PE machine type 0x014c (x86). The installer header also identifies an x86 launcher; this does not determine whether its payload contains other builds.
- VSTPlugins contains 29 DLL plug-in paths; each has PE machine type 0x014c (x86).
- vst3 contains three paths ending in .vst3, but their module contents were not probed, so their architecture is not established by this report.
- The DLL locations suggest legacy VST modules, but their exact ABI has not been confirmed.
- No plug-in binaries were executed during the recorded inventory. See [the plug-in rewrite inventory](plugin-rewrite-inventory.md) for paths and per-module status.

OpenTrack-'s goal is to rewrite the bundled plug-in behavior in Rust for its native x64 host. The inventory does not describe their DSP, controls, presets, automation, or state formats. Use REA and controlled reference-app sessions to build per-plug-in behavioral evidence before implementation or parity claims. Support for unrelated third-party legacy plug-ins is a separate compatibility decision; an x86 helper would not replace the planned Rust rewrites.

## Current limits

- No user-created song projects were found in the installation tree; the available CPR files are templates.
- The current application has an independent version 14 JSON RWP format with version 1 through 13 migrations. Version 5 direct-track MIDI notes are wrapped into MIDI parts during migration; version 6 gains default cycle locators; version 7 gains the default sine instrument; version 8 gains empty controller lanes; version 10 adds the OpenTrack- Compressor insert type; version 11 adds per-note MIDI channels; version 12 adds per-controller-event MIDI channels; version 13 adds global-preset origin metadata to workspaces; version 14 adds Solo Defeat. Older notes and controller events default to channel 0, and older workspaces have no global-preset origin. It is not a CPR parser or importer.
- The CPR utility reads the observed outer block framing, extracts exact printable strings from ROOT label payloads, and scans arbitrary printable ASCII runs; `--blocks` lists block boundaries, `--root-labels` decodes the observed ROOT label strings, and `--all-ascii-runs` includes byte offsets. It does not parse tracks, media references, mixer state, or plug-in state.
- No CPR importer, round-trip verification, or general-purpose plug-in host has been implemented. TestGenerator is integrated as an insert prototype, with reference behavior and DSP parity still pending.
