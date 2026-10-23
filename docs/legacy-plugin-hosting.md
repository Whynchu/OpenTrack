# Legacy 32-bit plug-in hosting investigation

This note evaluates whether the x64 workstation should host original x86 plug-ins. It is separate from the planned first-party Rust rewrites of Cubase's bundled effects and instruments.

## Local evidence

- The installed `Cubase5.exe` has PE machine type `0x014c` (x86).
- REA's bounded inventory found 29 DLLs under `VSTPlugins`, each with PE machine type `0x014c`. The three installed paths ending in `.vst3` were not probed, so their module architectures are unknown.
- The DLL inventory suggests legacy VST modules, but the exact ABI and versions have not been confirmed. The original modules have not been run or loaded by OpenTrack-.
- The Rust toolchain currently has both `x86_64-pc-windows-msvc` and `i686-pc-windows-msvc` targets installed. No helper process or plug-in host has been implemented.

## Process boundary

Windows does not allow an x64 process to load an x86 DLL for execution. Any such compatibility feature needs an x86 process boundary, such as a separate Rust helper; it cannot be an in-process library of the x64 application. Microsoft documents this restriction in [Running 32-bit Applications](https://learn.microsoft.com/windows/win32/winprog64/running-32-bit-applications) and describes cross-bitness IPC options in [Process Interoperability](https://learn.microsoft.com/en-us/windows/win32/winprog64/process-interoperability).

Audio processing cannot synchronously wait on a helper over ordinary request/response IPC from the real-time callback. A future design would need bounded, preallocated audio buffers and an explicit missed-block policy, with control, scanning, and state operations kept off the callback. No IPC choice has been made or measured.

## Distribution constraint

Steinberg's VST developer documentation says the VST 2 files in its SDK fall under a proprietary license; binary distribution of a VST 2 host requires written permission, and the page states that new VST 2 license submissions have not been accepted since October 2018. It also says the VST 2 files cannot be redistributed under the VST 3 SDK's other license terms. See Steinberg's [license file guidance](https://github.com/steinbergmedia/vst3_dev_portal/blob/main/src/pages/VST+3+Licensing/Which+files+fall+under+which+license.md).

OpenTrack- contains no Steinberg VST 2 SDK headers or source. A distributable legacy-host path therefore has an unresolved licensing gate. Keep the compatibility helper out of release scope until a valid distribution path is established. This does not affect the x64 Rust rewrites, which remain the plan for bundled Cubase behavior.

## Work still required

- Confirm the actual plug-in ABI and export behavior from authorized, bounded inspection; do not infer the ABI from folder names.
- Determine whether a suitable license was signed for this product before the deadline described by Steinberg, or select a legally reviewed alternative.
- If hosting proceeds, define a versioned 32/64-bit wire protocol, process lifecycle, crash recovery, plug-in state transport, latency reporting, and per-block underrun behavior.
- Validate the helper with a legally usable test plug-in before evaluating any installed module.
- Keep DLLs and any licensed SDK files outside the OpenTrack- source and distribution.
