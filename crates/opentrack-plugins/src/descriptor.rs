// SPDX-License-Identifier: Apache-2.0

//! Stable descriptors for discovered plug-in classes.
//!
//! These types are the cross-process contract between the scanner, the catalog
//! cache, the plug-in manager, and eventual project-side references. They must
//! not grow dependencies on a specific plug-in ABI. The VST3 layer maps its
//! factory data into these summaries and is responsible for validating values
//! before constructing them.
//!
//! Strings are untrusted scanner output. Producers should cap them at the
//! documented limits; consumers should still treat every count as advisory.

use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::PluginClassId;

/// Maximum characters stored in a descriptor name, vendor, or version field.
pub const MAX_DESCRIPTOR_TEXT_CHARS: usize = 256;

/// Maximum bus summaries retained per class.
///
/// Actual bus counts may be larger; the descriptor is a bounded summary.
pub const MAX_BUS_SUMMARIES: usize = 32;

/// Maximum parameter summaries retained per class.
///
/// [`PluginDescriptor::parameter_count`] remains the authoritative total even
/// when this summary list is truncated.
pub const MAX_PARAMETER_SUMMARIES: usize = 1024;

/// Plug-in format supported by the catalog.
///
/// VST2, 32-bit bridging, CLAP, and other formats are outside the current
/// scope and therefore have no stable variant yet.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginFormat {
    /// Steinberg VST3.
    Vst3,
}

impl PluginFormat {
    /// Stable lowercase name used in cache keys and report text.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vst3 => "vst3",
        }
    }
}

impl fmt::Display for PluginFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Architecture recorded for a module or class.
///
/// This is the architecture read from the module image, not the current host
/// process. The x64 host only accepts [`Self::X64`] for loading.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginArchitecture {
    /// 32-bit x86.
    #[serde(rename = "x86")]
    X86,
    /// 64-bit x86.
    #[serde(rename = "x64")]
    X64,
    /// 64-bit ARM.
    #[serde(rename = "arm64")]
    Arm64,
    /// A machine type OpenTrack does not recognize yet.
    Other,
}

impl PluginArchitecture {
    /// True when the primary x64 Windows host can load this architecture.
    pub const fn is_host_compatible(self) -> bool {
        matches!(self, Self::X64)
    }

    /// Short stable label for reports and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X64 => "x64",
            Self::Arm64 => "arm64",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for PluginArchitecture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Result of scanning one plug-in class or module.
///
/// `ReadyForEnumeration` means only that the image passed the architecture
/// precheck. It does not mean the factory loaded or that classes were found.
/// `Enumerated` is reserved for a completed future class enumeration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginScanStatus {
    /// No scan has been attempted.
    NotScanned,
    /// The module passed the PE precheck; class enumeration has not run yet.
    ReadyForEnumeration,
    /// The factory and class metadata were read successfully.
    Enumerated,
    /// The module exists but targets a different CPU architecture.
    UnsupportedArchitecture,
    /// The file is not a valid loadable module image.
    InvalidModule,
    /// The module could not be read because of an I/O or access error.
    Unreadable,
    /// The module was readable but factory loading failed.
    LoadFailed,
    /// The scanner exceeded its per-module time budget.
    TimedOut,
    /// The scanner process terminated unexpectedly.
    Crashed,
    /// The plug-in requires authorization or user interaction before scanning.
    NeedsAuthorization,
    /// The module is blocked until the user explicitly retries it.
    Quarantined,
}

impl PluginScanStatus {
    /// Stable lowercase status code used in text reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotScanned => "not_scanned",
            Self::ReadyForEnumeration => "ready_for_enumeration",
            Self::Enumerated => "enumerated",
            Self::UnsupportedArchitecture => "unsupported_architecture",
            Self::InvalidModule => "invalid_module",
            Self::Unreadable => "unreadable",
            Self::LoadFailed => "load_failed",
            Self::TimedOut => "timed_out",
            Self::Crashed => "crashed",
            Self::NeedsAuthorization => "needs_authorization",
            Self::Quarantined => "quarantined",
        }
    }

    /// True when the descriptor contains usable class metadata.
    pub const fn is_enumerated(self) -> bool {
        matches!(self, Self::Enumerated)
    }

    /// True when the module image is a valid candidate for future loading.
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::ReadyForEnumeration | Self::Enumerated)
    }

    /// True when the status represents a failure or a deliberate rejection.
    pub const fn is_rejected(self) -> bool {
        matches!(
            self,
            Self::UnsupportedArchitecture
                | Self::InvalidModule
                | Self::Unreadable
                | Self::LoadFailed
                | Self::TimedOut
                | Self::Crashed
                | Self::NeedsAuthorization
                | Self::Quarantined
        )
    }
}

impl fmt::Display for PluginScanStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Broad category used by pickers and the plug-in manager.
///
/// The detailed VST3 subcategory list stays in the format-specific layer until
/// the UI needs a stable facet model.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    /// Audio effect.
    Effect,
    /// Instrument or tone generator.
    Instrument,
    /// Metering/analysis plug-in.
    Analyzer,
    /// Category not classified by the host yet.
    Other,
}

/// Direction of an audio or event bus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginBusDirection {
    /// Bus receives data from the host.
    Input,
    /// Bus sends data to the host.
    Output,
}

/// VST3-style main/aux distinction without importing VST3 types.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginBusKind {
    /// Main audio path.
    Main,
    /// Auxiliary path, which may be a sidechain input in a specific format.
    Aux,
}

/// Bounded summary of one declared bus.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PluginBusSummary {
    /// Bus name reported by the plug-in, bounded by the producer.
    pub name: String,
    /// Data direction.
    pub direction: PluginBusDirection,
    /// Main or auxiliary role.
    pub kind: PluginBusKind,
    /// Number of channels declared for the bus.
    pub channel_count: u16,
    /// Whether the plug-in marks the bus active by default.
    pub default_active: bool,
}

impl PluginBusSummary {
    /// True for a main output bus.
    pub const fn is_main_output(&self) -> bool {
        matches!(self.direction, PluginBusDirection::Output)
            && matches!(self.kind, PluginBusKind::Main)
    }
}

/// Bounded summary of one exposed parameter.
///
/// This is catalog metadata only. Runtime parameter values and gestures belong
/// to the host/runtime layer, not the cache.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginParameterSummary {
    /// Stable parameter ID supplied by the format implementation.
    pub id: u32,
    /// Display title, bounded by the producer.
    pub title: String,
    /// Optional short title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_title: Option<String>,
    /// Optional unit string such as `dB` or `Hz`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Number of discrete steps, or zero for continuous parameters.
    pub step_count: i32,
    /// Default normalized value in the inclusive range 0.0 to 1.0.
    pub default_normalized: f64,
}

/// Stable description of one plug-in class.
///
/// The class identity is `(format, class_id)`. `module_path` is a discovery
/// hint and may be stale after an update, move, or duplicate install.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginDescriptor {
    /// Format that owns the class ID namespace.
    pub format: PluginFormat,
    /// 16-byte class identity.
    pub class_id: PluginClassId,
    /// Current display name reported by the plug-in.
    pub name: String,
    /// Vendor name.
    pub vendor: String,
    /// Plug-in version string.
    pub version: String,
    /// Broad category.
    pub category: PluginCategory,
    /// Vendor subcategories split from the format's category text.
    ///
    /// For VST3 this is the `subCategories` field split on `|`. The values are
    /// kept format-neutral because the scanner and picker only use them as
    /// display/facet hints.
    #[serde(default)]
    pub subcategories: Vec<String>,
    /// Path of the module that exposed this class.
    pub module_path: PathBuf,
    /// Architecture of that module image.
    pub architecture: PluginArchitecture,
    /// Bounded bus summaries.
    pub buses: Vec<PluginBusSummary>,
    /// Total reported parameter count, even when `parameters` is truncated.
    pub parameter_count: u32,
    /// Bounded parameter summaries.
    pub parameters: Vec<PluginParameterSummary>,
    /// Whether a native editor is expected to be available.
    pub has_editor: bool,
    /// Reported latency in samples at the scan preparation rate.
    pub latency_samples: u32,
    /// Advisory tail length in samples.
    pub tail_samples: u32,
    /// Outcome of the scan that produced this descriptor.
    pub scan_status: PluginScanStatus,
    /// When the scan was completed, in Unix milliseconds.
    pub scanned_at_unix_ms: u64,
    /// Optional bounded diagnostic text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<String>,
}

impl PluginDescriptor {
    /// True when the catalog may offer this class for insertion.
    pub const fn is_usable(&self) -> bool {
        self.scan_status.is_enumerated() && matches!(self.architecture, PluginArchitecture::X64)
    }

    /// First main output bus, which is the v1 stereo-main policy target.
    pub fn main_output(&self) -> Option<&PluginBusSummary> {
        self.buses.iter().find(|bus| bus.is_main_output())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_compatibility_is_explicit() {
        assert!(PluginArchitecture::X64.is_host_compatible());
        assert!(!PluginArchitecture::X86.is_host_compatible());
        assert!(!PluginArchitecture::Arm64.is_host_compatible());
        assert!(!PluginArchitecture::Other.is_host_compatible());
    }

    #[test]
    fn status_classes_are_distinct() {
        assert!(PluginScanStatus::Enumerated.is_enumerated());
        assert!(PluginScanStatus::ReadyForEnumeration.is_ready());
        assert!(!PluginScanStatus::ReadyForEnumeration.is_enumerated());
        assert!(PluginScanStatus::TimedOut.is_rejected());
        assert!(!PluginScanStatus::NotScanned.is_rejected());
    }

    fn descriptor() -> PluginDescriptor {
        PluginDescriptor {
            format: PluginFormat::Vst3,
            class_id: PluginClassId::ZERO,
            name: "Example".to_owned(),
            vendor: "Vendor".to_owned(),
            version: "1.0".to_owned(),
            category: PluginCategory::Effect,
            subcategories: vec!["Fx".to_owned(), "Dynamics".to_owned()],
            module_path: PathBuf::from("Example.vst3"),
            architecture: PluginArchitecture::X64,
            buses: Vec::new(),
            parameter_count: 0,
            parameters: Vec::new(),
            has_editor: false,
            latency_samples: 0,
            tail_samples: 0,
            scan_status: PluginScanStatus::Enumerated,
            scanned_at_unix_ms: 0,
            diagnostics: None,
        }
    }

    #[test]
    fn main_output_is_selected_without_aux() {
        let bus = |direction, kind, name: &str| PluginBusSummary {
            name: name.to_owned(),
            direction,
            kind,
            channel_count: 2,
            default_active: true,
        };
        let mut descriptor = descriptor();
        descriptor.buses = vec![
            bus(PluginBusDirection::Input, PluginBusKind::Main, "stereo in"),
            bus(PluginBusDirection::Input, PluginBusKind::Aux, "sidechain"),
            bus(
                PluginBusDirection::Output,
                PluginBusKind::Main,
                "stereo out",
            ),
        ];
        assert_eq!(
            descriptor.main_output().map(|bus| bus.name.as_str()),
            Some("stereo out")
        );
        assert!(descriptor.is_usable());
    }

    #[test]
    fn subcategories_round_trip_and_default_to_empty() {
        let descriptor = descriptor();
        let json = serde_json::to_string(&descriptor).expect("descriptor serializes");
        assert!(json.contains("\"subcategories\":[\"Fx\",\"Dynamics\"]"));
        let parsed: PluginDescriptor =
            serde_json::from_str(&json).expect("descriptor deserializes");
        assert_eq!(parsed.subcategories, descriptor.subcategories);

        let mut legacy = serde_json::to_value(&descriptor).expect("descriptor serializes");
        legacy
            .as_object_mut()
            .expect("descriptor is an object")
            .remove("subcategories");
        let parsed: PluginDescriptor =
            serde_json::from_value(legacy).expect("descriptor without subcategories deserializes");
        assert!(parsed.subcategories.is_empty());
    }
}
