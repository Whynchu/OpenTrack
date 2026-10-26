// SPDX-License-Identifier: Apache-2.0

//! VST3 module loading and factory class enumeration for OpenTrack.
//!
//! This crate owns the VST3 ABI boundary. It currently:
//!
//! - loads one x64 Windows module on the calling thread;
//! - reads factory/class metadata, including WaveShell child classes;
//! - exposes a minimal control-thread host context (`IHostApplication`,
//!   `IMessage`, `IAttributeList`).
//!
//! It does **not** create components, process audio, host editors, save state,
//! or implement the block graph/PDC. Those remain later milestones in
//! `docs/plugin-hosting-plan.md`.
//!
//! Module scanning must still run in the separate `opentrack-plugin-scan`
//! process because a buggy plug-in can terminate the loader. The scanner is a
//! crash boundary, not a security sandbox.

// The VST3 COM traits are unsafe functions by definition. Each implementation
// below validates null pointers and lengths before touching caller memory.
#![allow(unsafe_op_in_unsafe_fn)]

pub mod host;
pub mod module;

use std::path::Path;

use opentrack_plugins::{PluginArchitecture, PluginDescriptor, PluginFormat, PluginScanStatus};

pub use host::{HostContext, HostMessage};
pub use module::{ModuleEnumeration, Vst3ClassInfo, Vst3LoadError, Vst3Module};

/// Host-facing version used in cache invalidation and scanner handshakes.
pub const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Converts class metadata into the format-neutral catalog descriptor.
pub fn class_descriptor(
    module_path: &Path,
    architecture: PluginArchitecture,
    class: &Vst3ClassInfo,
    scanned_at_unix_ms: u64,
) -> PluginDescriptor {
    PluginDescriptor {
        format: PluginFormat::Vst3,
        class_id: class.class_id,
        name: class.name.clone(),
        vendor: class.vendor.clone(),
        version: class.version.clone(),
        category: class.plugin_category(),
        module_path: module_path.to_path_buf(),
        architecture,
        buses: Vec::new(),
        parameter_count: 0,
        parameters: Vec::new(),
        has_editor: false,
        latency_samples: 0,
        tail_samples: 0,
        scan_status: PluginScanStatus::Enumerated,
        scanned_at_unix_ms,
        diagnostics: Some(String::from(
            "factory class metadata only; buses, parameters and latency require instance introspection",
        )),
    }
}
