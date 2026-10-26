// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Format-neutral plug-in catalog types for OpenTrack.
//!
//! This crate is the schema foundation described by
//! `docs/plugin-hosting-plan.md` sections 4.1, 4.10, and 4.12. It defines
//! stable, serializable descriptors for discovered plug-in classes and the
//! catalog/cache envelope shared by the scanner, the application, and future
//! project storage.
//!
//! Deliberate boundaries:
//!
//! - No VST3 ABI types, SDK headers, or COM interfaces appear here. The future
//!   `opentrack-vst3` crate owns those.
//! - A class is identified by [`PluginClassId`] plus [`PluginFormat`], never by
//!   its module path. Paths are discovery hints and diagnostic evidence.
//! - Nothing in this crate loads a module or hosts audio. It is data only.
//! - Scanner output is untrusted input. The string bounds in [`descriptor`] are
//!   limits for producers; consumers should still validate catalog versions.

pub mod catalog;
pub mod class_id;
pub mod descriptor;

pub use catalog::{
    AddPluginOutcome, CATALOG_FILE_NAME, CATALOG_SCHEMA_VERSION, PluginCacheKey, PluginCatalog,
    PluginDuplicate, PluginModuleFingerprint, PluginModuleRecord,
};
pub use class_id::{CLASS_ID_BYTES, CLASS_ID_HEX_LEN, ClassIdParseError, PluginClassId};
pub use descriptor::{
    MAX_BUS_SUMMARIES, MAX_DESCRIPTOR_TEXT_CHARS, MAX_PARAMETER_SUMMARIES, PluginArchitecture,
    PluginBusDirection, PluginBusKind, PluginBusSummary, PluginCategory, PluginDescriptor,
    PluginFormat, PluginParameterSummary, PluginScanStatus,
};
