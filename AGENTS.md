# Project instructions

## Product goal

OpenTrack- is a standalone, full Rust reimplementation of Cubase 5, including first-party Rust rewrites of its bundled plug-ins. It is not a Cubase extension. REA is an investigation tool used to collect evidence about the reference application; it is not the product. The checked-in application remains an early prototype, so keep goals separate from implemented behavior.

## Reverse-engineering boundaries

- Keep first-party application, audio engine, compatibility, and plug-in rewrite code in Rust. Document OS APIs and external plug-in ABI boundaries.
- Use the reference application, local Operation Manual, and REA to record observable behavior, file compatibility, commands, and plug-in behavior. Distinguish observations, inferences, and unknowns.
- Keep the reference install and analysis tools outside this repository. Keep OpenTrack- source, notes, and original generated assets inside it.
- Use original code and fresh visual assets. Treat installed Cubase files and plug-in binaries as evidence, not material to copy or ship.
- The primary application target is x86_64 Windows. Isolate compatibility work behind crates or process boundaries. An optional legacy 32-bit plug-in helper is a separate compatibility feature, not a substitute for rewriting bundled plug-ins.

## Compatibility and implementation

- Follow [the fidelity plan](docs/fidelity-plan.md): observe a complete reference workflow, specify it, implement it, and compare it before expanding it.
- Treat source inspection, compilation, manual documentation, runtime observation, and verified parity as different evidence levels. A successful build does not prove workflow or visual fidelity.
- Every fidelity batch must name its reference evidence and acceptance steps. Unmeasured layout and behavior stay unknown; do not invent defaults and label them Cubase behavior.
- Stop at the review landmarks in the fidelity plan for user involvement. Do not begin the next substantial UI batch until the current review is resolved.

- Do not claim CPR read/write compatibility until real project files and round-trip behavior have been verified. The current RWP format is OpenTrack-'s own format.
- Rewrite bundled Cubase 5 plug-in behavior as original Rust implementations for OpenTrack-. Record each plug-in's observable inputs, outputs, state, and limits before claiming parity.
- Do not add dependencies or framework choices until they have been evaluated against product requirements.
