// SPDX-License-Identifier: Apache-2.0

//! Catalog and cache schema.
//!
//! The scanner writes a [`PluginCatalog`] to the per-user cache. Class identity
//! is `(format, class ID)`; duplicate module paths are retained as diagnostics
//! rather than replacing the canonical descriptor. See
//! `docs/plugin-hosting-plan.md` section 4.12 for the cache policy this schema
//! supports.
//!
//! The schema does not contain plug-in state. Project-side state sidecars are a
//! separate, later concern.

use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::PluginArchitecture;
use crate::PluginClassId;
use crate::PluginDescriptor;
use crate::PluginFormat;
use crate::PluginScanStatus;

/// Current catalog/cache schema version.
pub const CATALOG_SCHEMA_VERSION: u32 = 1;

/// File name proposed by the hosting plan for the v1 cache.
pub const CATALOG_FILE_NAME: &str = "catalog-v1.json";

/// File-system facts used to invalidate a cached module record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PluginModuleFingerprint {
    /// Absolute module path at scan time.
    pub path: PathBuf,
    /// File size in bytes at scan time.
    pub size_bytes: u64,
    /// Last modification time as Unix milliseconds, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_unix_ms: Option<u64>,
}

impl PluginModuleFingerprint {
    /// Creates a fingerprint from file metadata values.
    pub fn new(path: PathBuf, size_bytes: u64, modified_unix_ms: Option<u64>) -> Self {
        Self {
            path,
            size_bytes,
            modified_unix_ms,
        }
    }
}

/// Cache key for one module.
///
/// The key follows the hosting plan: module path, size, mtime, scanner version,
/// host version, and format. A change to any component invalidates the record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PluginCacheKey {
    /// File-system identity of the module.
    pub fingerprint: PluginModuleFingerprint,
    /// Format namespace for the class IDs in this module.
    pub format: PluginFormat,
    /// Scanner build that produced the record.
    pub scanner_version: String,
    /// Host build that accepted the record.
    pub host_version: String,
}

/// Cached result for one module image.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PluginModuleRecord {
    /// Invalidation key.
    pub key: PluginCacheKey,
    /// Architecture read from the image.
    pub architecture: PluginArchitecture,
    /// Scan outcome.
    pub scan_status: PluginScanStatus,
    /// Classes found in this module, deduplicated by class ID.
    pub class_ids: Vec<PluginClassId>,
    /// When this module record was produced, in Unix milliseconds.
    pub scanned_at_unix_ms: u64,
    /// Optional bounded diagnostic text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<String>,
}

/// A class identity found at more than one module path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PluginDuplicate {
    /// Format namespace for `class_id`.
    pub format: PluginFormat,
    /// Duplicated class identity.
    pub class_id: PluginClassId,
    /// Descriptor path retained as canonical.
    pub kept_path: PathBuf,
    /// Additional path that exposed the same class.
    pub duplicate_path: PathBuf,
}

/// Versioned plug-in catalog written by the scanner.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginCatalog {
    /// Schema version from [`CATALOG_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Scanner build that produced the catalog.
    pub scanner_version: String,
    /// When the catalog was generated, in Unix milliseconds.
    pub generated_at_unix_ms: u64,
    /// Module scan records, including rejected modules.
    pub modules: Vec<PluginModuleRecord>,
    /// Class descriptors. At most one per `(format, class_id)`.
    pub plugins: Vec<PluginDescriptor>,
    /// Duplicate class/path observations for diagnostics.
    pub duplicates: Vec<PluginDuplicate>,
}

impl PluginCatalog {
    /// Creates an empty catalog with the current schema version.
    pub fn new(scanner_version: impl Into<String>, generated_at_unix_ms: u64) -> Self {
        Self {
            schema_version: CATALOG_SCHEMA_VERSION,
            scanner_version: scanner_version.into(),
            generated_at_unix_ms,
            modules: Vec::new(),
            plugins: Vec::new(),
            duplicates: Vec::new(),
        }
    }

    /// True when this catalog was written with the schema version compiled here.
    pub const fn is_current_schema(&self) -> bool {
        self.schema_version == CATALOG_SCHEMA_VERSION
    }

    /// Inserts or replaces the module record for the same path and format.
    ///
    /// Returns true when an existing record was replaced.
    pub fn upsert_module(&mut self, record: PluginModuleRecord) -> bool {
        let existing = self.modules.iter().position(|candidate| {
            candidate.key.format == record.key.format
                && candidate.key.fingerprint.path == record.key.fingerprint.path
        });
        match existing {
            Some(index) => {
                self.modules[index] = record;
                true
            }
            None => {
                self.modules.push(record);
                false
            }
        }
    }

    /// Adds a class descriptor, deduplicating by format and class ID.
    ///
    /// The first descriptor wins. A later descriptor with the same identity but
    /// a different module path is recorded in [`Self::duplicates`] for
    /// diagnostics.
    pub fn add_plugin(&mut self, descriptor: PluginDescriptor) -> AddPluginOutcome {
        let existing = self.plugins.iter().position(|candidate| {
            candidate.format == descriptor.format && candidate.class_id == descriptor.class_id
        });

        let Some(index) = existing else {
            self.plugins.push(descriptor);
            return AddPluginOutcome::Inserted;
        };

        if self.plugins[index].module_path == descriptor.module_path {
            return AddPluginOutcome::AlreadyPresent;
        }

        let already_recorded = self.duplicates.iter().any(|duplicate| {
            duplicate.format == descriptor.format
                && duplicate.class_id == descriptor.class_id
                && duplicate.duplicate_path == descriptor.module_path
        });
        if already_recorded {
            return AddPluginOutcome::AlreadyPresent;
        }

        let duplicate = PluginDuplicate {
            format: descriptor.format,
            class_id: descriptor.class_id,
            kept_path: self.plugins[index].module_path.clone(),
            duplicate_path: descriptor.module_path,
        };
        self.duplicates.push(duplicate.clone());
        AddPluginOutcome::Duplicate(duplicate)
    }

    /// Finds a descriptor by class identity.
    pub fn find_plugin(
        &self,
        format: PluginFormat,
        class_id: PluginClassId,
    ) -> Option<&PluginDescriptor> {
        self.plugins
            .iter()
            .find(|plugin| plugin.format == format && plugin.class_id == class_id)
    }
}

/// Result of adding a descriptor to a catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AddPluginOutcome {
    /// The descriptor became the canonical entry for its class identity.
    Inserted,
    /// The same descriptor path was already present.
    AlreadyPresent,
    /// The class existed at a different path; the duplicate was recorded.
    Duplicate(PluginDuplicate),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::PluginBusDirection;
    use crate::PluginBusKind;
    use crate::PluginBusSummary;
    use crate::PluginCategory;
    use crate::PluginParameterSummary;

    fn descriptor(path: &str) -> PluginDescriptor {
        PluginDescriptor {
            format: PluginFormat::Vst3,
            class_id: PluginClassId::from_bytes([
                0xde, 0xad, 0xbe, 0xef, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a,
                0x0b, 0x0c,
            ]),
            name: "Example Compressor".to_owned(),
            vendor: "Example Audio".to_owned(),
            version: "1.2.3".to_owned(),
            category: PluginCategory::Effect,
            module_path: PathBuf::from(path),
            architecture: PluginArchitecture::X64,
            buses: vec![PluginBusSummary {
                name: "Stereo Out".to_owned(),
                direction: PluginBusDirection::Output,
                kind: PluginBusKind::Main,
                channel_count: 2,
                default_active: true,
            }],
            parameter_count: 1,
            parameters: vec![PluginParameterSummary {
                id: 7,
                title: "Threshold".to_owned(),
                short_title: Some("Thr".to_owned()),
                unit: Some("dB".to_owned()),
                step_count: 0,
                default_normalized: 0.5,
            }],
            has_editor: true,
            latency_samples: 64,
            tail_samples: 0,
            scan_status: PluginScanStatus::Enumerated,
            scanned_at_unix_ms: 1_700_000_000_000,
            diagnostics: None,
        }
    }

    fn module_record(path: &str) -> PluginModuleRecord {
        PluginModuleRecord {
            key: PluginCacheKey {
                fingerprint: PluginModuleFingerprint::new(
                    PathBuf::from(path),
                    4_096,
                    Some(1_700_000_000_000),
                ),
                format: PluginFormat::Vst3,
                scanner_version: "0.1.0".to_owned(),
                host_version: "0.1.0".to_owned(),
            },
            architecture: PluginArchitecture::X64,
            scan_status: PluginScanStatus::Enumerated,
            class_ids: vec![PluginClassId::ZERO],
            scanned_at_unix_ms: 1_700_000_000_000,
            diagnostics: None,
        }
    }

    #[test]
    fn duplicate_class_keeps_first_path() {
        let mut catalog = PluginCatalog::new("0.1.0", 1);
        assert_eq!(
            catalog.add_plugin(descriptor(r"C:\VST3\A.vst3")),
            AddPluginOutcome::Inserted
        );

        let outcome = catalog.add_plugin(descriptor(r"D:\Duplicate\A.vst3"));
        match outcome {
            AddPluginOutcome::Duplicate(duplicate) => {
                assert_eq!(duplicate.kept_path, PathBuf::from(r"C:\VST3\A.vst3"));
                assert_eq!(
                    duplicate.duplicate_path,
                    PathBuf::from(r"D:\Duplicate\A.vst3")
                );
            }
            other => panic!("expected duplicate outcome, got {other:?}"),
        }

        assert_eq!(catalog.plugins.len(), 1);
        assert_eq!(catalog.duplicates.len(), 1);
        assert_eq!(
            catalog.find_plugin(PluginFormat::Vst3, descriptor("ignored").class_id),
            Some(&catalog.plugins[0])
        );
    }

    #[test]
    fn repeated_identical_descriptor_is_a_no_op() {
        let mut catalog = PluginCatalog::new("0.1.0", 1);
        catalog.add_plugin(descriptor(r"C:\VST3\A.vst3"));
        assert_eq!(
            catalog.add_plugin(descriptor(r"C:\VST3\A.vst3")),
            AddPluginOutcome::AlreadyPresent
        );
        assert!(catalog.duplicates.is_empty());
        assert_eq!(catalog.plugins.len(), 1);
    }

    #[test]
    fn module_upsert_replaces_same_path_and_format() {
        let mut catalog = PluginCatalog::new("0.1.0", 1);
        assert!(!catalog.upsert_module(module_record(r"C:\VST3\A.vst3")));

        let mut replacement = module_record(r"C:\VST3\A.vst3");
        replacement.scan_status = PluginScanStatus::InvalidModule;
        assert!(catalog.upsert_module(replacement));

        assert_eq!(catalog.modules.len(), 1);
        assert_eq!(
            catalog.modules[0].scan_status,
            PluginScanStatus::InvalidModule
        );
    }

    #[test]
    fn catalog_round_trips_through_json() {
        let mut catalog = PluginCatalog::new("0.1.0", 1_700_000_000_000);
        catalog.upsert_module(module_record(r"C:\VST3\A.vst3"));
        catalog.add_plugin(descriptor(r"C:\VST3\A.vst3"));
        let duplicate = descriptor(r"D:\Duplicate\A.vst3");
        catalog.add_plugin(duplicate);

        let json = serde_json::to_string_pretty(&catalog).expect("catalog serializes");
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"class_id\": \"deadbeef0102030405060708090a0b0c\""));

        let parsed: PluginCatalog = serde_json::from_str(&json).expect("catalog deserializes");
        assert_eq!(parsed, catalog);
        assert!(parsed.is_current_schema());
    }
}
