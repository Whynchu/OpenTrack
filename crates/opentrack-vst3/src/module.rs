// SPDX-License-Identifier: Apache-2.0

//! VST3 module loading and factory enumeration.
//!
//! The module stays loaded while its `ComPtr<IPluginFactory>` exists. Field
//! order is intentional: the factory drops before the library unloads. This
//! crate never initializes a component or processes audio yet.

use std::fmt;
use std::path::{Path, PathBuf};

use opentrack_plugins::{PluginCategory, PluginClassId};
use vst3::ComPtr;
use vst3::Steinberg::{
    IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3, IPluginFactory3Trait,
    IPluginFactoryTrait, PClassInfo, PClassInfo2, PClassInfoW, PFactoryInfo, kResultOk,
};

type GetPluginFactory = unsafe extern "system" fn() -> *mut IPluginFactory;

#[cfg(windows)]
type PlatformLibrary = libloading::os::windows::Library;
#[cfg(target_os = "linux")]
type PlatformLibrary = libloading::os::unix::Library;

#[cfg(windows)]
type ExitModule = unsafe extern "system" fn() -> bool;
#[cfg(not(windows))]
type ExitModule = unsafe extern "C" fn() -> bool;

/// Failure while loading a module or reading its factory.
#[derive(Debug)]
pub enum Vst3LoadError {
    /// The dynamic library could not be opened.
    Open(String),
    /// `GetPluginFactory` is missing.
    MissingEntryPoint(String),
    /// A required platform-specific entry or exit function is missing.
    MissingLifecycleEntryPoint(&'static str),
    /// The optional module `InitDll` export returned false.
    InitFailed,
    /// The required platform-specific module entry function returned false.
    ModuleEntryFailed(&'static str),
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
            Self::MissingLifecycleEntryPoint(name) => {
                write!(
                    formatter,
                    "missing required VST3 module lifecycle export: {name}"
                )
            }
            Self::InitFailed => formatter.write_str("module InitDll returned false"),
            Self::ModuleEntryFailed(name) => {
                write!(
                    formatter,
                    "VST3 module entry function {name} returned false"
                )
            }
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
    // Drop releases the factory, calls the module exit function, then unloads.
    factory: Option<ComPtr<IPluginFactory>>,
    lifecycle: ModuleLifecycle,
    #[cfg(target_os = "macos")]
    _bundle: mac_bundle::MacBundle,
    original_dir: Option<PathBuf>,
    #[cfg(any(windows, target_os = "linux"))]
    _library: PlatformLibrary,
    _com: ComApartment,
}

impl Vst3Module {
    pub fn open(path: &Path) -> Result<Self, Vst3LoadError> {
        let original_dir = std::env::current_dir().ok();
        let com = ComApartment::new();
        #[cfg(windows)]
        let library = unsafe {
            libloading::os::windows::Library::load_with_flags(
                path,
                libloading::os::windows::LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        }
        .map_err(|error| Vst3LoadError::Open(error.to_string()))?;
        #[cfg(target_os = "linux")]
        let library = unsafe { libloading::os::unix::Library::new(path) }
            .map_err(|error| Vst3LoadError::Open(error.to_string()))?;

        #[cfg(target_os = "macos")]
        let bundle = mac_bundle::MacBundle::open(path)?;

        #[cfg(windows)]
        let lifecycle = {
            let init_dll: Option<unsafe extern "system" fn() -> bool> =
                unsafe { library.get(b"InitDll\0").ok().map(|symbol| *symbol) };
            if let Some(init) = init_dll
                && !unsafe { init() }
            {
                return Err(Vst3LoadError::InitFailed);
            }
            let exit_dll: Option<ExitModule> =
                unsafe { library.get(b"ExitDll\0").ok().map(|symbol| *symbol) };
            ModuleLifecycle::new(exit_dll)
        };

        #[cfg(target_os = "linux")]
        let (library, lifecycle) = {
            let entry: unsafe extern "C" fn(*mut std::ffi::c_void) -> bool =
                unsafe { library.get(b"ModuleEntry\0") }
                    .map(|symbol| *symbol)
                    .map_err(|_| Vst3LoadError::MissingLifecycleEntryPoint("ModuleEntry"))?;
            let exit: ExitModule = unsafe { library.get(b"ModuleExit\0") }
                .map(|symbol| *symbol)
                .map_err(|_| Vst3LoadError::MissingLifecycleEntryPoint("ModuleExit"))?;
            let handle = library.into_raw();
            let initialized = unsafe { entry(handle) };
            // SAFETY: `handle` came from this loaded library and is restored
            // immediately, so it remains owned and unloadable on every path.
            let library = unsafe { libloading::os::unix::Library::from_raw(handle) };
            if !initialized {
                return Err(Vst3LoadError::ModuleEntryFailed("ModuleEntry"));
            }
            (library, ModuleLifecycle::new(Some(exit)))
        };

        #[cfg(target_os = "macos")]
        let lifecycle = {
            type BundleEntry =
                unsafe extern "C" fn(core_foundation_sys::bundle::CFBundleRef) -> bool;
            let entry = unsafe { bundle.function::<BundleEntry>(c"bundleEntry") }
                .map_err(|_| Vst3LoadError::MissingLifecycleEntryPoint("bundleEntry"))?;
            let exit = unsafe { bundle.function::<ExitModule>(c"bundleExit") }
                .map_err(|_| Vst3LoadError::MissingLifecycleEntryPoint("bundleExit"))?;
            if !unsafe { entry(bundle.as_raw()) } {
                return Err(Vst3LoadError::ModuleEntryFailed("bundleEntry"));
            }
            ModuleLifecycle::new(Some(exit))
        };

        #[cfg(any(windows, target_os = "linux"))]
        let get_factory: GetPluginFactory = unsafe { library.get(b"GetPluginFactory\0") }
            .map(|symbol| *symbol)
            .map_err(|error| Vst3LoadError::MissingEntryPoint(error.to_string()))?;
        #[cfg(target_os = "macos")]
        let get_factory: GetPluginFactory = unsafe { bundle.function(c"GetPluginFactory") }
            .map_err(Vst3LoadError::MissingEntryPoint)?;
        let raw = unsafe { get_factory() };
        let factory = unsafe { ComPtr::from_raw(raw) }.ok_or(Vst3LoadError::NullFactory)?;
        if let Some(dir) = &original_dir {
            let _ = std::env::set_current_dir(dir);
        }
        Ok(Self {
            factory: Some(factory),
            lifecycle,
            #[cfg(target_os = "macos")]
            _bundle: bundle,
            original_dir,
            #[cfg(any(windows, target_os = "linux"))]
            _library: library,
            _com: com,
        })
    }

    pub(crate) fn factory(&self) -> &ComPtr<IPluginFactory> {
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

struct ModuleLifecycle {
    exit: Option<ExitModule>,
}

impl ModuleLifecycle {
    fn new(exit: Option<ExitModule>) -> Self {
        Self { exit }
    }

    fn close(&mut self) {
        if let Some(exit) = self.exit.take() {
            unsafe { exit() };
        }
    }
}

impl Drop for ModuleLifecycle {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(target_os = "macos")]
mod mac_bundle {
    use std::ffi::{CStr, c_void};
    use std::mem::size_of;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use core_foundation_sys::base::{CFRelease, kCFAllocatorDefault};
    use core_foundation_sys::bundle::{
        CFBundleCreate, CFBundleGetFunctionPointerForName, CFBundleLoadExecutable, CFBundleRef,
        CFBundleUnloadExecutable,
    };
    use core_foundation_sys::string::{CFStringCreateWithCString, kCFStringEncodingASCII};
    use core_foundation_sys::url::CFURLCreateFromFileSystemRepresentation;

    use super::Vst3LoadError;

    /// Loaded VST3 bundle. It remains alive until the module exit function runs.
    pub(super) struct MacBundle {
        raw: CFBundleRef,
        executable_loaded: bool,
    }

    impl MacBundle {
        pub(super) fn open(module_path: &Path) -> Result<Self, Vst3LoadError> {
            let bundle_path = containing_vst3_bundle(module_path).ok_or_else(|| {
                Vst3LoadError::Open(format!(
                    "module is not inside a macOS .vst3 bundle: {}",
                    module_path.display()
                ))
            })?;
            let bundle_path = if bundle_path.is_absolute() {
                bundle_path
            } else {
                std::env::current_dir()
                    .map_err(|error| Vst3LoadError::Open(error.to_string()))?
                    .join(bundle_path)
            };
            let path_bytes = bundle_path.as_os_str().as_bytes();
            let url = unsafe {
                CFURLCreateFromFileSystemRepresentation(
                    kCFAllocatorDefault,
                    path_bytes.as_ptr(),
                    path_bytes.len() as isize,
                    1,
                )
            };
            if url.is_null() {
                return Err(Vst3LoadError::Open(format!(
                    "could not create a Core Foundation URL for {}",
                    bundle_path.display()
                )));
            }
            let raw = unsafe { CFBundleCreate(kCFAllocatorDefault, url) };
            unsafe { CFRelease(url.cast::<c_void>()) };
            if raw.is_null() {
                return Err(Vst3LoadError::Open(format!(
                    "could not create a Core Foundation bundle for {}",
                    bundle_path.display()
                )));
            }
            if unsafe { CFBundleLoadExecutable(raw) } == 0 {
                unsafe { CFRelease(raw.cast::<c_void>()) };
                return Err(Vst3LoadError::Open(format!(
                    "Core Foundation could not load the VST3 bundle {}",
                    bundle_path.display()
                )));
            }
            Ok(Self {
                raw,
                executable_loaded: true,
            })
        }

        pub(super) fn as_raw(&self) -> CFBundleRef {
            self.raw
        }

        /// Resolves a required function exported by the loaded bundle.
        ///
        /// # Safety
        /// `T` must be the function pointer type declared by the VST3 ABI for
        /// the requested symbol.
        pub(super) unsafe fn function<T: Copy>(&self, name: &CStr) -> Result<T, String> {
            let cf_name = unsafe {
                CFStringCreateWithCString(
                    kCFAllocatorDefault,
                    name.as_ptr(),
                    kCFStringEncodingASCII,
                )
            };
            if cf_name.is_null() {
                return Err("could not allocate a Core Foundation symbol name".to_owned());
            }
            let pointer = unsafe { CFBundleGetFunctionPointerForName(self.raw, cf_name) };
            unsafe { CFRelease(cf_name.cast::<c_void>()) };
            if pointer.is_null() {
                return Err(format!("bundle does not export {}", name.to_string_lossy()));
            }
            if size_of::<T>() != size_of::<*const c_void>() {
                return Err("resolved VST3 symbol has an incompatible pointer size".to_owned());
            }
            // SAFETY: callers pair each symbol name with its exact VST3
            // function-pointer type, and function/data pointers have the same
            // representation on supported macOS targets.
            Ok(unsafe { std::mem::transmute_copy(&pointer) })
        }
    }

    impl Drop for MacBundle {
        fn drop(&mut self) {
            if self.executable_loaded {
                unsafe { CFBundleUnloadExecutable(self.raw) };
            }
            unsafe { CFRelease(self.raw.cast::<c_void>()) };
        }
    }

    fn containing_vst3_bundle(module_path: &Path) -> Option<PathBuf> {
        module_path
            .ancestors()
            .find(|ancestor| {
                ancestor.is_dir()
                    && ancestor.extension().is_some_and(|extension| {
                        extension.to_string_lossy().eq_ignore_ascii_case("vst3")
                    })
            })
            .map(Path::to_path_buf)
    }
}

impl Drop for Vst3Module {
    fn drop(&mut self) {
        let _ = self.factory.take();
        self.lifecycle.close();
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
        // The module is loaded on the control/UI thread. VST3 editors expect
        // the conventional STA there; an MTA initialized by the loader prevents
        // the editor apartment from being established later.
        let result = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
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
        eprintln!("opening {}", Path::new(&path).display());
        let module = Vst3Module::open(Path::new(&path)).expect("module opens");
        let enumeration = module.enumerate().expect("factory enumerates");
        eprintln!(
            "factory '{}' exposed {} audio module classes",
            enumeration.factory_vendor,
            enumeration.classes.len()
        );
        for class in enumeration.classes {
            eprintln!("  {} | {}", class.name, class.class_id);
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
