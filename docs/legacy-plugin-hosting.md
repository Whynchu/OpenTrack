# Legacy 32-bit plug-in hosting investigation

This note evaluates whether the x64 OpenTrack should host original x86 plug-ins. It is separate from deferred, individually designed first-party effects and instruments; Cubase bundled rewrites are no longer required.

## Local evidence

- The installed `Cubase5.exe` has PE machine type `0x014c` (x86).
- REA's bounded inventory found 29 DLLs under `VSTPlugins`, each with PE machine type `0x014c`. A follow-up PE export inspection found all three installed `.vst3` modules are also x86 (32-bit); each exports `ExitDll`, `GetPluginFactory`, and `InitDll`. These are static facts, not proof of Cubase loading or complete ABI compatibility. Per-file hashes and Evidence IDs are in `plugin-rewrite-inventory.md`.
- On 2026-10-24, REA's read-only PE export reader found that `Cubase5.exe` (SHA-256 `787d1a82eb6401a3117055d749c449fbd632b795cc0bfd7f71f6b6d88419109d`) declares no export directory, while `VSTPlugins/Tools/TestGenerator.dll` (SHA-256 `f8948c642ca9e1b10bffde545ed88e3f242dcbf6d8b336bdcc7f639fc04f55b5`) exports `VSTPluginMain` at ordinal 1. Evidence IDs: `ev_9dde5a5e2e9d7a85fff124f26ad31c32d4105249162c34633ebb6d1d0009ad3a` and `ev_4708b1f4b90e1a5066e5634fb892c6bdba0cd7b10dde50e6bf6eeb067bc95fc7`. This confirms one entry-point symbol in one bundled x86 DLL; it does not prove Cubase loads it, identify the complete ABI version, or reveal its DSP.
- The DLL inventory suggests legacy VST modules, but the exact ABI and versions have not been confirmed. The original modules have not been run or loaded by OpenTrack-.
- The Rust toolchain currently has both `x86_64-pc-windows-msvc` and `i686-pc-windows-msvc` targets installed. No helper process or plug-in host has been implemented.

## Process boundary

Windows does not allow an x64 process to load an x86 DLL for execution. Any such compatibility feature needs an x86 process boundary, such as a separate Rust helper; it cannot be an in-process library of the x64 application. Microsoft documents this restriction in [Running 32-bit Applications](https://learn.microsoft.com/windows/win32/winprog64/running-32-bit-applications) and describes cross-bitness IPC options in [Process Interoperability](https://learn.microsoft.com/en-us/windows/win32/winprog64/process-interoperability).

Audio processing cannot synchronously wait on a helper over ordinary request/response IPC from the real-time callback. A future design would need bounded, preallocated audio buffers and an explicit missed-block policy, with control, scanning, and state operations kept off the callback. No IPC choice has been made or measured.

## Distribution constraint

Steinberg's VST developer documentation says the VST 2 files in its SDK fall under a proprietary license; binary distribution of a VST 2 host requires written permission, and the page states that new VST 2 license submissions have not been accepted since October 2018. It also says the VST 2 files cannot be redistributed under the VST 3 SDK's other license terms. See Steinberg's [license file guidance](https://github.com/steinbergmedia/vst3_dev_portal/blob/main/src/pages/VST+3+Licensing/Which+files+fall+under+which+license.md).

OpenTrack- contains no Steinberg VST 2 SDK headers or source. A distributable legacy-host path therefore has an unresolved licensing gate. Keep the compatibility helper out of release scope until a valid distribution path is established. This does not determine future first-party Rust effect designs. Bundled Cubase behavior recreation is deferred.

## Work still required

- Confirm the actual plug-in ABI/version and Cubase's call path from authorized, bounded inspection. `VSTPluginMain` is observed in TestGenerator and `GetPluginFactory` in three VST3 modules, but the complete ABI and runtime call sequence remain unknown; do not infer them from folder names.
- Determine whether a suitable license was signed for this product before the deadline described by Steinberg, or select a legally reviewed alternative.
- If hosting proceeds, define a versioned 32/64-bit wire protocol, process lifecycle, crash recovery, plug-in state transport, latency reporting, and per-block underrun behavior.
- Validate the helper with a legally usable test plug-in before evaluating any installed module.
- Keep DLLs and any licensed SDK files outside the OpenTrack- source and distribution.
