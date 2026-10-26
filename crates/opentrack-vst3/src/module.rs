// SPDX-License-Identifier: Apache-2.0

//! VST3 module loading and factory enumeration.
//!
//! The module stays loaded while its `ComPtr<IPluginFactory>` exists. Field
//! order is intentional: the factory drops before the library unloads. This
//! crate never initializes a component or processes audio yet.

use std::fmt;
use std::path::{Path, PathBuf};

use libloading::Library;
use opentrack_plugins::{PluginCategory, PluginClassId};
use vst3::ComPtr;
use vst3::Steinberg::{
    IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3, IPluginFactory3Trait,
    IPluginFactoryTrait, PClassInfo, PClassInfo2, PClassInfoW, PFactoryInfo, kResultOk,
};

type GetPluginFactory = unsafe extern "system" fn() -> *mut IPluginFactory;

/// Failure while loading a module or reading its factory.
#[derive(Debug)]
pub enum Vst3LoadError {
    /// The dynamic library could not be opened.
    Open(String),
    /// `GetPluginFactory` is missing.
    MissingEntryPoint(String),
    /// The optional module `InitDll` export returned false.
    InitFailed,
    /// `GetPluginFactory` returned null.
    NullFactory,
    /// A factory metadata call failed.
    Factory(String),
    /// The module supplied an implausible class count.
    InvalidClassCount(i32),
}

impl fmt::Display for Vst3LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(message) => write!(formatter, "could not open VST3 module: {message}"),
            Self::MissingEntryPoint(message) => {
                write!(formatter, "missing GetPluginFactory export: {message}")
            }
            Self::InitFailed => formatter.write_str("module InitDll returned false"),
            Self::NullFactory => formatter.write_str("GetPluginFactory returned null"),
            Self::Factory(message) => write!(formatter, "factory metadata failed: {message}"),
            Self::InvalidClassCount(count) => {
                write!(
                    formatter,
                    "factory reported an invalid class count: {count}"
                )
            }
        }
    }
}

impl std::error::Error for Vst3LoadError {}

/// One class exposed by a VST3 factory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vst3ClassInfo {
    pub class_id: PluginClassId,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub category: String,
    pub subcategories: String,
    pub cardinality: i32,
    pub class_flags: u32,
}

impl Vst3ClassInfo {
    /// Maps VST3 category/subcategory text to the format-neutral picker facet.
    pub fn plugin_category(&self) -> PluginCategory {
        let subcategories = self.subcategories.split('|');
        for subcategory in subcategories {
            let subcategory = subcategory.trim();
            if subcategory.eq_ignore_ascii_case("Instrument")
                || subcategory.eq_ignore_ascii_case("Synth")
            {
                return PluginCategory::Instrument;
            }
            if subcategory.eq_ignore_ascii_case("Analyzer")
                || subcategory.eq_ignore_ascii_case("Meter")
            {
                return PluginCategory::Analyzer;
            }
        }
        if self.category.eq_ignore_ascii_case("Instrument") {
            PluginCategory::Instrument
        } else if self.category.contains("Audio Module") {
            PluginCategory::Effect
        } else {
            PluginCategory::Other
        }
    }

    pub fn is_audio_module(&self) -> bool {
        self.category
            .trim()
            .eq_ignore_ascii_case("Audio Module Class")
    }
}

/// Factory and class metadata read from one module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleEnumeration {
    pub factory_vendor: String,
    pub factory_url: String,
    pub factory_email: String,
    pub classes: Vec<Vst3ClassInfo>,
    pub diagnostics: Vec<String>,
}

/// A loaded VST3 module and its factory.
pub struct Vst3Module {
    // Drop releases the factory and calls ExitDll before the library unloads.
    factory: Option<ComPtr<IPluginFactory>>,
    exit_dll: Option<unsafe extern "system" fn() -> bool>,
    original_dir: Option<PathBuf>,
    _library: Library,
    _com: ComApartment,
}

impl Vst3Module {
    pub fn open(path: &Path) -> Result<Self, Vst3LoadError> {
        let original_dir = std::env::current_dir().ok();
        let com = ComApartment::new();
        #[cfg(windows)]
        let library: Library = unsafe {
            libloading::os::windows::Library::load_with_flags(
                path,
                libloading::os::windows::LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        }
        .map_err(|error| Vst3LoadError::Open(error.to_string()))?
        .into();
        #[cfg(not(windows))]
        let library = unsafe { Library::new(path) }
            .map_err(|error| Vst3LoadError::Open(error.to_string()))?;
        let init_dll: Option<unsafe extern "system" fn() -> bool> =
            unsafe { library.get(b"InitDll\0").ok().map(|symbol| *symbol) };
        if let Some(init) = init_dll
            && !unsafe { init() }
        {
            return Err(Vst3LoadError::InitFailed);
        }
        let exit_dll: Option<unsafe extern "system" fn() -> bool> =
            unsafe { library.get(b"ExitDll\0").ok().map(|symbol| *symbol) };
        let get_factory: libloading::Symbol<'_, GetPluginFactory> =
            unsafe { library.get(b"GetPluginFactory\0") }
                .map_err(|error| Vst3LoadError::MissingEntryPoint(error.to_string()))?;
        let raw = unsafe { get_factory() };
        let factory = unsafe { ComPtr::from_raw(raw) }.ok_or(Vst3LoadError::NullFactory)?;
        if let Some(dir) = &original_dir {
            let _ = std::env::set_current_dir(dir);
        }
        Ok(Self {
            factory: Some(factory),
            exit_dll,
            original_dir,
            _library: library,
            _com: com,
        })
    }

    fn factory(&self) -> &ComPtr<IPluginFactory> {
        self.factory.as_ref().expect("factory is alive")
    }

    pub fn enumerate(&self) -> Result<ModuleEnumeration, Vst3LoadError> {
        let mut factory_info = PFactoryInfo {
            vendor: [0; 64],
            url: [0; 256],
            email: [0; 128],
            flags: 0,
        };
        let factory_result = unsafe { self.factory().getFactoryInfo(&mut factory_info) };
        if factory_result != kResultOk {
            return Err(Vst3LoadError::Factory(format!(
                "getFactoryInfo returned 0x{factory_result:08x}"
            )));
        }

        let count = unsafe { self.factory().countClasses() };
        if !(0..=4096).contains(&count) {
            return Err(Vst3LoadError::InvalidClassCount(count));
        }

        let factory3 = self.factory().cast::<IPluginFactory3>();
        let factory2 = self.factory().cast::<IPluginFactory2>();
        let mut diagnostics = Vec::new();
        let mut classes = Vec::new();
        for index in 0..count {
            let class = if let Some(factory) = &factory3 {
                let mut info = PClassInfoW {
                    cid: [0; 16],
                    cardinality: 0,
                    category: [0; 32],
                    name: [0; 64],
                    classFlags: 0,
                    subCategories: [0; 128],
                    vendor: [0; 64],
                    version: [0; 64],
                    sdkVersion: [0; 64],
                };
                let result = unsafe { factory.getClassInfoUnicode(index, &mut info) };
                if result != kResultOk {
                    diagnostics.push(format!(
                        "class {index}: getClassInfoUnicode returned 0x{result:08x}"
                    ));
                    continue;
                }
                Vst3ClassInfo {
                    class_id: class_id(&info.cid),
                    name: wide_string(&info.name),
                    vendor: wide_string(&info.vendor),
                    version: wide_string(&info.version),
                    category: ascii_string(&info.category),
                    subcategories: ascii_string(&info.subCategories),
                    cardinality: info.cardinality,
                    class_flags: info.classFlags,
                }
            } else if let Some(factory) = &factory2 {
                let mut info = PClassInfo2 {
                    cid: [0; 16],
                    cardinality: 0,
                    category: [0; 32],
                    name: [0; 64],
                    classFlags: 0,
                    subCategories: [0; 128],
                    vendor: [0; 64],
                    version: [0; 64],
                    sdkVersion: [0; 64],
                };
                let result = unsafe { factory.getClassInfo2(index, &mut info) };
                if result != kResultOk {
                    diagnostics.push(format!(
                        "class {index}: getClassInfo2 returned 0x{result:08x}"
                    ));
                    continue;
                }
                Vst3ClassInfo {
                    class_id: class_id(&info.cid),
                    name: ascii_string(&info.name),
                    vendor: ascii_string(&info.vendor),
                    version: ascii_string(&info.version),
                    category: ascii_string(&info.category),
                    subcategories: ascii_string(&info.subCategories),
                    cardinality: info.cardinality,
                    class_flags: info.classFlags,
                }
            } else {
                let mut info = PClassInfo {
                    cid: [0; 16],
                    cardinality: 0,
                    category: [0; 32],
                    name: [0; 64],
                };
                let result = unsafe { self.factory().getClassInfo(index, &mut info) };
                if result != kResultOk {
                    diagnostics.push(format!(
                        "class {index}: getClassInfo returned 0x{result:08x}"
                    ));
                    continue;
                }
                Vst3ClassInfo {
                    class_id: class_id(&info.cid),
                    name: ascii_string(&info.name),
                    vendor: ascii_string(&factory_info.vendor),
                    version: String::new(),
                    category: ascii_string(&info.category),
                    subcategories: String::new(),
                    cardinality: info.cardinality,
                    class_flags: 0,
                }
            };
            if class.is_audio_module() {
                classes.push(class);
            }
        }
        classes.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.class_id.cmp(&right.class_id))
        });
        Ok(ModuleEnumeration {
            factory_vendor: ascii_string(&factory_info.vendor),
            factory_url: ascii_string(&factory_info.url),
            factory_email: ascii_string(&factory_info.email),
            classes,
            diagnostics,
        })
    }
}

impl Drop for Vst3Module {
    fn drop(&mut self) {
        let _ = self.factory.take();
        if let Some(exit) = self.exit_dll.take() {
            unsafe { exit() };
        }
        if let Some(dir) = self.original_dir.take() {
            let _ = std::env::set_current_dir(dir);
        }
    }
}

#[cfg(windows)]
struct ComApartment {
    initialized: bool,
}

#[cfg(windows)]
impl ComApartment {
    fn new() -> Self {
        let result = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        Self {
            initialized: result.is_ok(),
        }
    }
}

#[cfg(windows)]
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { windows::Win32::System::Com::CoUninitialize() };
        }
    }
}

#[cfg(not(windows))]
struct ComApartment;

#[cfg(not(windows))]
impl ComApartment {
    fn new() -> Self {
        Self
    }
}

fn class_id(value: &[i8; 16]) -> PluginClassId {
    PluginClassId::from_bytes(std::array::from_fn(|index| value[index] as u8))
}

fn ascii_string(value: &[i8]) -> String {
    value
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| *byte as u8 as char)
        .collect()
}

fn wide_string(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn probe_installed_module_step_by_step() {
        let Some(path) = std::env::var_os("OPENTRACK_VST3_PROBE") else {
            eprintln!("set OPENTRACK_VST3_PROBE to a module path");
            return;
        };
        let _com = ComApartment::new();
        eprintln!("opening {}", Path::new(&path).display());
        #[cfg(windows)]
        let library: Library = unsafe {
            libloading::os::windows::Library::load_with_flags(
                &path,
                libloading::os::windows::LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        }
        .expect("library opens")
        .into();
        #[cfg(not(windows))]
        let library = unsafe { Library::new(&path) }.expect("library opens");
        eprintln!("library opened");
        let init_dll: Option<unsafe extern "system" fn() -> bool> =
            unsafe { library.get(b"InitDll\0").ok().map(|symbol| *symbol) };
        eprintln!("InitDll present: {}", init_dll.is_some());
        if let Some(init) = init_dll {
            eprintln!("InitDll result: {}", unsafe { init() });
        }
        let get_factory: libloading::Symbol<'_, GetPluginFactory> =
            unsafe { library.get(b"GetPluginFactory\0") }.expect("factory export");
        eprintln!("export found");
        let raw = unsafe { get_factory() };
        eprintln!("factory pointer {raw:p}");
        let factory = unsafe { ComPtr::from_raw(raw) }.expect("non-null factory");
        eprintln!("factory owned");
        let mut info = PFactoryInfo {
            vendor: [0; 64],
            url: [0; 256],
            email: [0; 128],
            flags: 0,
        };
        let result = unsafe { factory.getFactoryInfo(&mut info) };
        eprintln!("factory info result 0x{result:08x}");
        let count = unsafe { factory.countClasses() };
        eprintln!("class count {count}");
        let factory3 = factory.cast::<IPluginFactory3>();
        eprintln!("factory3 {}", factory3.is_some());
        if let Some(factory3) = factory3 {
            let mut class = PClassInfoW {
                cid: [0; 16],
                cardinality: 0,
                category: [0; 32],
                name: [0; 64],
                classFlags: 0,
                subCategories: [0; 128],
                vendor: [0; 64],
                version: [0; 64],
                sdkVersion: [0; 64],
            };
            let result = unsafe { factory3.getClassInfoUnicode(0, &mut class) };
            eprintln!("class 0 result 0x{result:08x}");
        }
        eprintln!("probe complete");
    }

    #[test]
    fn missing_modules_report_a_load_error_without_panicking() {
        let error = Vst3Module::open(Path::new("this-module-does-not-exist.vst3"));
        assert!(matches!(error, Err(Vst3LoadError::Open(_))));
    }

    #[test]
    fn categories_map_to_picker_facets() {
        let class = |category: &str, subcategories: &str| Vst3ClassInfo {
            class_id: PluginClassId::ZERO,
            name: "Example".into(),
            vendor: "Vendor".into(),
            version: "1".into(),
            category: category.into(),
            subcategories: subcategories.into(),
            cardinality: 1,
            class_flags: 0,
        };
        assert_eq!(
            class("Audio Module Class", "Fx|EQ").plugin_category(),
            PluginCategory::Effect
        );
        assert_eq!(
            class("Audio Module Class", "Instrument|Synth").plugin_category(),
            PluginCategory::Instrument
        );
        assert_eq!(
            class("Audio Module Class", "Analyzer").plugin_category(),
            PluginCategory::Analyzer
        );
        assert!(class("Audio Module Class", "Fx").is_audio_module());
        assert!(!class("Component Controller Class", "").is_audio_module());
    }
}
