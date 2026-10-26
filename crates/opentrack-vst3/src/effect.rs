// SPDX-License-Identifier: Apache-2.0

//! One prepared stereo VST3 effect instance.
//!
//! Lifecycle, bus negotiation, block processing and state follow
//! `docs/plugin-hosting-plan.md` sections 4.3-4.11. The implementation keeps
//! the audio callback free of allocation, locks and control-thread calls: it
//! copies into preallocated planar buffers, drains a lock-free parameter queue
//! and calls `IAudioProcessor::process` only.

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_void};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use opentrack_plugins::{HostedProcessor, PluginClassId};
use vst3::Steinberg::Vst::BusTypes_::kMain;
use vst3::Steinberg::Vst::{
    AudioBusBuffers, AudioBusBuffers__type0,
    BusDirections_::{kInput, kOutput},
    BusInfo, IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler, IComponentTrait,
    IConnectionPoint, IConnectionPointTrait, IEditController, IEditControllerTrait,
    IParameterChanges,
    MediaTypes_::kAudio,
    ParameterInfo, ProcessContext,
    ProcessContext_::StatesAndFlags_::kPlaying,
    ProcessData,
    ProcessModes_::kRealtime,
    ProcessSetup,
    RestartFlags_::kLatencyChanged,
    SpeakerArr, SpeakerArrangement, String128,
    SymbolicSampleSizes_::kSample32,
};
use vst3::Steinberg::{
    FIDString, IBStream, IPlugFrame, IPlugView, IPluginBaseTrait, IPluginFactory, IPluginFactory3,
    IPluginFactory3Trait, IPluginFactoryTrait, int32, kNotImplemented, kResultFalse, kResultOk,
};
use vst3::{ComPtr, ComWrapper, Interface};

use crate::editor::{self, EditorSession};
use crate::host::HostContext;
use crate::module::Vst3Module;
use crate::parameter::ParameterChanges;
use crate::services::{HostServices, HostShared};
use crate::stream::MemoryStream;

/// Largest block size accepted by [`Vst3Effect::load`]/[`Vst3Effect::prepare`].
pub const MAX_BLOCK_FRAMES: u32 = 65_536;

/// Largest parameter count enumerated from a plug-in.
pub const MAX_PARAMETERS: i32 = 8_192;

const MAX_BUS_COUNT: i32 = 64;

/// Serialized component and controller state.
#[derive(Clone, Debug, Default)]
pub struct Vst3State {
    pub component: Vec<u8>,
    pub controller: Vec<u8>,
}

/// One normalized VST3 parameter as seen by the host.
#[derive(Clone, Debug)]
pub struct Vst3ParameterInfo {
    pub id: u32,
    pub title: String,
    pub short_title: String,
    pub units: String,
    pub step_count: i32,
    pub default_normalized: f64,
    pub normalized: f64,
}

/// The main input/output bus selected during instantiation.
#[derive(Clone, Copy, Debug)]
struct BusLayout {
    input_index: usize,
    output_index: usize,
    input_channels: i32,
    output_channels: i32,
    input_bus_count: usize,
    output_bus_count: usize,
}

struct Connection {
    component: ComPtr<IConnectionPoint>,
    controller: ComPtr<IConnectionPoint>,
}

/// Preallocated planar buffers and VST3 structures for the audio callback.
struct ProcessBuffers {
    input_left: Vec<f32>,
    input_right: Vec<f32>,
    output_left: Vec<f32>,
    output_right: Vec<f32>,
    input_buses: Vec<AudioBusBuffers>,
    output_buses: Vec<AudioBusBuffers>,
    context: ProcessContext,
    frames_processed: u64,
}

impl Default for ProcessBuffers {
    fn default() -> Self {
        Self {
            input_left: Vec::new(),
            input_right: Vec::new(),
            output_left: Vec::new(),
            output_right: Vec::new(),
            input_buses: Vec::new(),
            output_buses: Vec::new(),
            // ProcessContext is a plain-old-data structure. The control thread
            // replaces this placeholder during `prepare`.
            context: unsafe { std::mem::zeroed() },
            frames_processed: 0,
        }
    }
}

fn empty_audio_bus() -> AudioBusBuffers {
    AudioBusBuffers {
        numChannels: 0,
        silenceFlags: 0,
        __field0: AudioBusBuffers__type0 {
            channelBuffers32: std::ptr::null_mut(),
        },
    }
}

/// A loaded, initialized and prepared VST3 effect.
///
/// The object owns the module and every COM object created from it. The module
/// is always released last; `Drop` stops processing, deactivates, disconnects,
/// terminates and only then releases the component/controller and module.
pub struct Vst3Effect {
    // The editor holds a view owned by the controller, so it must be closed
    // before the controller is released.
    editor: Mutex<Option<EditorSession>>,

    shared: Arc<HostShared>,
    handler: ComPtr<IComponentHandler>,
    plug_frame: ComPtr<IPlugFrame>,

    component: ComPtr<IComponent>,
    processor: ComPtr<IAudioProcessor>,
    controller: ComPtr<IEditController>,
    separate_controller: bool,
    connection: Option<Connection>,

    component_initialized: bool,
    controller_initialized: bool,
    active: bool,
    processing: bool,

    parameter_changes: ComWrapper<ParameterChanges>,
    parameter_changes_ptr: ComPtr<IParameterChanges>,
    buffers: UnsafeCell<ProcessBuffers>,
    layout: BusLayout,
    prepared: bool,
    max_block_frames: usize,
    sample_rate: f64,

    latency: AtomicU32,
    tail: AtomicU32,
    has_editor: bool,
    class_id: PluginClassId,

    // Borrowed by `component.initialize`; keep it alive for the whole instance.
    host_context: HostContext,

    // Declared last: dropping the module must happen after every COM object
    // above has been released (ExitDll/unload ordering).
    _module: Vst3Module,
}

// SAFETY: `HostedProcessor` is shared by the audio engine, but `process` is
// serialized on one audio callback thread. The only non-`Sync` member is the
// preallocated `ProcessBuffers` behind `UnsafeCell`, which is exclusively
// written by that callback. `prepare` requires `&mut self`, so it cannot run
// while the shared audio reference exists. Controller, editor and state calls
// use separate COM objects/atomics and are documented as control-thread work.
unsafe impl Sync for Vst3Effect {}

impl Vst3Effect {
    /// Loads `module_path`, instantiates `class_id`, negotiates a stereo (or
    /// mono-in/stereo-out) main bus and prepares it for block processing.
    pub fn load(
        module_path: &Path,
        class_id: PluginClassId,
        sample_rate: u32,
        max_block_frames: u32,
    ) -> Result<Self, String> {
        validate_prepare_arguments(sample_rate, max_block_frames)?;

        let module = Vst3Module::open(module_path)
            .map_err(|error| format!("could not load {}: {error}", module_path.display()))?;
        let factory = module.factory().clone();

        // IPluginFactory3 wants the host application context before classes are
        // instantiated. The context is moved into the effect below and stays
        // alive for the component/controller lifetime.
        let host_context = HostContext::new();
        if let Some(factory3) = factory.cast::<IPluginFactory3>() {
            // Host context is optional for IPluginFactory3. The SDK host
            // ignores this result; some factories return kNotImplemented even
            // though component creation works normally (for example Gateway).
            let _ = unsafe { factory3.setHostContext(host_context.as_funknown()) };
        }

        let shared = Arc::new(HostShared::default());
        let services = HostServices::new(Arc::clone(&shared));
        let handler = services
            .to_com_ptr::<IComponentHandler>()
            .ok_or_else(|| String::from("could not create the host IComponentHandler"))?;
        let plug_frame = services
            .to_com_ptr::<IPlugFrame>()
            .ok_or_else(|| String::from("could not create the host IPlugFrame"))?;
        let parameter_changes = ParameterChanges::new();
        let parameter_changes_ptr = parameter_changes
            .to_com_ptr::<IParameterChanges>()
            .ok_or_else(|| String::from("could not create input parameter changes"))?;

        let component = create_component(&factory, class_id)?;
        // Some WaveShell-style single-component wrappers only expose
        // IEditController after initialize; initialize before resolving the
        // controller. The component already owns the host context.
        let result = unsafe { component.initialize(host_context.as_funknown()) };
        if result != kResultOk {
            return Err(format!("component initialize failed with 0x{result:08x}"));
        }

        let mut controller_cid = [0 as c_char; 16];
        let controller_result = unsafe { component.getControllerClassId(&mut controller_cid) };
        let (controller, separate_controller) = if controller_result == kResultOk {
            match create_controller(&factory, &controller_cid) {
                Ok(controller) => (controller, true),
                Err(error) => {
                    unsafe {
                        let _ = component.terminate();
                    }
                    return Err(error);
                }
            }
        } else {
            // A single-component plug-in implements IEditController itself.
            let Some(controller) = component.cast::<IEditController>() else {
                unsafe {
                    let _ = component.terminate();
                }
                return Err(format!(
                    "component does not provide a controller class and does not implement IEditController (getControllerClassId returned 0x{controller_result:08x})"
                ));
            };
            (controller, false)
        };
        let Some(processor) = component.cast::<IAudioProcessor>() else {
            unsafe {
                let _ = component.terminate();
            }
            return Err(String::from(
                "component does not implement IAudioProcessor; it is not a VST3 audio effect",
            ));
        };

        let mut effect = Self {
            editor: Mutex::new(None),
            shared,
            handler,
            plug_frame,
            component,
            processor,
            controller,
            separate_controller,
            connection: None,
            component_initialized: true,
            controller_initialized: false,
            active: false,
            processing: false,
            parameter_changes,
            parameter_changes_ptr,
            buffers: UnsafeCell::new(ProcessBuffers::default()),
            layout: BusLayout {
                input_index: 0,
                output_index: 0,
                input_channels: 2,
                output_channels: 2,
                input_bus_count: 1,
                output_bus_count: 1,
            },
            prepared: false,
            max_block_frames: 0,
            sample_rate: 0.0,
            latency: AtomicU32::new(0),
            tail: AtomicU32::new(0),
            has_editor: false,
            class_id,
            host_context,
            _module: module,
        };

        if let Err(error) = effect.initialize_host() {
            effect.shutdown();
            return Err(error);
        }
        if let Err(error) = effect.prepare(sample_rate, max_block_frames) {
            effect.shutdown();
            return Err(error);
        }
        Ok(effect)
    }

    /// Negotiates buses, queries latency/tail and activates the plug-in.
    fn initialize_host(&mut self) -> Result<(), String> {
        let context = self.host_context.as_funknown();
        if !self.component_initialized {
            let result = unsafe { self.component.initialize(context) };
            if result != kResultOk {
                return Err(format!("component initialize failed with 0x{result:08x}"));
            }
            self.component_initialized = true;
        }

        if self.separate_controller {
            let result = unsafe { self.controller.initialize(context) };
            if result != kResultOk {
                return Err(format!("controller initialize failed with 0x{result:08x}"));
            }
            self.controller_initialized = true;
        }

        if self.separate_controller {
            let component_point = self.component.cast::<IConnectionPoint>();
            let controller_point = self.controller.cast::<IConnectionPoint>();
            let (Some(component_point), Some(controller_point)) =
                (component_point, controller_point)
            else {
                return Err(String::from(
                    "separate component/controller do not both implement IConnectionPoint",
                ));
            };
            let result = unsafe { component_point.connect(controller_point.as_ptr()) };
            if result != kResultOk {
                return Err(format!(
                    "component/controller connection failed with 0x{result:08x}"
                ));
            }
            // Split plug-ins expect a bidirectional connection: messages and
            // parameter edits flow both ways, and some controllers do not
            // publish parameters or editors until the reverse link exists.
            let result = unsafe { controller_point.connect(component_point.as_ptr()) };
            if result != kResultOk {
                unsafe {
                    let _ = component_point.disconnect(controller_point.as_ptr());
                }
                return Err(format!(
                    "controller/component connection failed with 0x{result:08x}"
                ));
            }
            self.connection = Some(Connection {
                component: component_point,
                controller: controller_point,
            });
        }

        let result = unsafe { self.controller.setComponentHandler(self.handler.as_ptr()) };
        if result != kResultOk {
            return Err(format!(
                "controller rejected the host component handler with 0x{result:08x}"
            ));
        }

        self.layout = negotiate_buses(&self.component, &self.processor)?;
        self.has_editor = probe_editor(&self.controller);
        Ok(())
    }

    pub fn class_id(&self) -> PluginClassId {
        self.class_id
    }

    /// Returns and clears the "plug-in edited its state" marker.
    ///
    /// Native-editor edits and plug-in requests set this flag; the application
    /// can mark the project dirty without polling individual parameters.
    pub fn take_dirty(&self) -> bool {
        self.shared.dirty.swap(false, Ordering::AcqRel)
    }

    /// Reallocates buffers and re-runs `setupProcessing`/activation.
    ///
    /// The caller must ensure `process` is not running; the signature requires
    /// exclusive access, which is how the graph enforces it.
    pub fn prepare(&mut self, sample_rate: u32, max_block_frames: u32) -> Result<(), String> {
        validate_prepare_arguments(sample_rate, max_block_frames)?;
        self.prepared = false;
        if self.processing {
            unsafe {
                let _ = self.processor.setProcessing(0);
            }
            self.processing = false;
        }
        if self.active {
            unsafe {
                let _ = self.component.setActive(0);
            }
            self.active = false;
        }

        let max_frames = max_block_frames as usize;
        // SAFETY: `&mut self` excludes `process`, which is the only other
        // accessor of these buffers.
        let buffers = unsafe { &mut *self.buffers.get() };
        buffers.input_left = vec![0.0; max_frames];
        buffers.input_right = vec![0.0; max_frames];
        buffers.output_left = vec![0.0; max_frames];
        buffers.output_right = vec![0.0; max_frames];
        buffers.input_buses = vec![empty_audio_bus(); self.layout.input_bus_count];
        buffers.output_buses = vec![empty_audio_bus(); self.layout.output_bus_count];
        buffers.frames_processed = 0;
        // SAFETY: ProcessContext is plain-old-data and zero is a valid value.
        buffers.context = unsafe { std::mem::zeroed() };
        buffers.context.sampleRate = sample_rate as f64;
        buffers.context.state = kPlaying as u32;

        let mut setup = ProcessSetup {
            processMode: kRealtime,
            symbolicSampleSize: kSample32,
            maxSamplesPerBlock: max_block_frames as int32,
            sampleRate: sample_rate as f64,
        };
        let result = unsafe { self.processor.setupProcessing(&mut setup) };
        if result != kResultOk {
            return Err(format!(
                "setupProcessing failed with 0x{result:08x} (sample rate {sample_rate}, block {max_block_frames})"
            ));
        }

        // Requirement: query latency after setupProcessing.
        self.refresh_processing_info();
        self.activate_main_buses()?;

        let result = unsafe { self.component.setActive(1) };
        if result != kResultOk {
            return Err(format!("setActive(true) failed with 0x{result:08x}"));
        }
        self.active = true;

        let result = unsafe { self.processor.setProcessing(1) };
        if result != kResultOk {
            unsafe {
                let _ = self.component.setActive(0);
            }
            self.active = false;
            return Err(format!("setProcessing(true) failed with 0x{result:08x}"));
        }
        self.processing = true;

        self.max_block_frames = max_frames;
        self.sample_rate = sample_rate as f64;
        self.prepared = true;
        Ok(())
    }

    /// Activates the negotiated main buses and deactivates every auxiliary bus.
    fn activate_main_buses(&self) -> Result<(), String> {
        for index in 0..self.layout.input_bus_count {
            let state = u8::from(index == self.layout.input_index);
            let result = unsafe {
                self.component
                    .activateBus(kAudio, kInput, index as int32, state)
            };
            if result != kResultOk {
                return Err(format!(
                    "could not synchronize input bus {index} activation (0x{result:08x})"
                ));
            }
        }
        for index in 0..self.layout.output_bus_count {
            let state = u8::from(index == self.layout.output_index);
            let result = unsafe {
                self.component
                    .activateBus(kAudio, kOutput, index as int32, state)
            };
            if result != kResultOk {
                return Err(format!(
                    "could not synchronize output bus {index} activation (0x{result:08x})"
                ));
            }
        }
        Ok(())
    }

    /// Cached latency in samples. Refreshed by `prepare` and after
    /// `restartComponent` requests observed on the control thread.
    pub fn latency_samples(&self) -> u32 {
        self.latency.load(Ordering::Relaxed)
    }

    /// Cached tail length in samples.
    pub fn tail_samples(&self) -> u32 {
        self.tail.load(Ordering::Relaxed)
    }

    /// Enumerates normalized parameters from the edit controller.
    ///
    /// Control-thread only. Hidden parameters are included; the application
    /// decides which ones to show.
    pub fn parameters(&self) -> Vec<Vst3ParameterInfo> {
        self.sync_restart_flags();
        let count = unsafe { self.controller.getParameterCount() };
        if count <= 0 {
            return Vec::new();
        }
        let count = count.min(MAX_PARAMETERS);
        let mut parameters = Vec::with_capacity(count as usize);
        for index in 0..count {
            let mut info = empty_parameter_info();
            let result = unsafe { self.controller.getParameterInfo(index, &mut info) };
            if result != kResultOk {
                continue;
            }
            let normalized = unsafe { self.controller.getParamNormalized(info.id) };
            parameters.push(Vst3ParameterInfo {
                id: info.id,
                title: string128(&info.title),
                short_title: string128(&info.shortTitle),
                units: string128(&info.units),
                step_count: info.stepCount,
                default_normalized: clamp_normalized(info.defaultNormalizedValue),
                normalized: clamp_normalized(normalized),
            });
        }
        parameters
    }

    /// Reads one parameter's normalized value. Returns `None` for an id the
    /// controller does not expose or a non-finite value.
    pub fn parameter_value(&self, id: u32) -> Option<f64> {
        self.sync_restart_flags();
        if !self.parameter_exists(id) {
            return None;
        }
        let value = unsafe { self.controller.getParamNormalized(id) };
        value.is_finite().then(|| clamp_normalized(value))
    }

    fn parameter_exists(&self, id: u32) -> bool {
        let count = unsafe { self.controller.getParameterCount() };
        if count <= 0 {
            return false;
        }
        for index in 0..count.min(MAX_PARAMETERS) {
            let mut info = empty_parameter_info();
            let result = unsafe { self.controller.getParameterInfo(index, &mut info) };
            if result == kResultOk && info.id == id {
                return true;
            }
        }
        false
    }

    /// Sets one normalized parameter through the controller and publishes the
    /// same value to the audio processor for the next block.
    ///
    /// Control-thread only. The edit is also pushed to the lock-free queue used
    /// by plug-in editors, so parameter changes do not require a graph rebuild.
    pub fn set_parameter(&self, id: u32, normalized: f64) {
        if !normalized.is_finite() {
            return;
        }
        let value = normalized.clamp(0.0, 1.0);
        self.sync_restart_flags();
        let result = unsafe { self.controller.setParamNormalized(id, value) };
        self.shared.dirty.store(true, Ordering::Release);
        if result == kResultOk {
            self.shared.parameters.push(id, value);
        }
    }

    /// Saves component and controller state through host `IBStream`s.
    ///
    /// Control-thread only. Do not call concurrently with [`Self::load_state`]
    /// or while the plug-in is being torn down.
    pub fn save_state(&self) -> Result<Vst3State, String> {
        self.sync_restart_flags();

        let component_stream = MemoryStream::empty();
        let component_ptr = component_stream
            .to_com_ptr::<IBStream>()
            .ok_or_else(|| String::from("could not create the component state stream"))?;
        let result = unsafe { self.component.getState(component_ptr.as_ptr()) };
        if result != kResultOk {
            return Err(format!("component getState failed with 0x{result:08x}"));
        }
        let component = component_stream.contents();

        let controller_stream = MemoryStream::empty();
        let controller_ptr = controller_stream
            .to_com_ptr::<IBStream>()
            .ok_or_else(|| String::from("could not create the controller state stream"))?;
        let result = unsafe { self.controller.getState(controller_ptr.as_ptr()) };
        let controller = if !self.separate_controller {
            // A single-component plug-in is the same object as its controller;
            // the component chunk is authoritative and must not be applied
            // twice on load. We still call `getState` as the host contract
            // requires.
            Vec::new()
        } else if result == kResultOk {
            controller_stream.contents()
        } else if result == kNotImplemented {
            // `IEditController::getState` is optional. Some plug-ins (including
            // TDR Nova) keep all state in the component chunk and return
            // kNotImplemented here; the component state remains authoritative.
            Vec::new()
        } else {
            return Err(format!("controller getState failed with 0x{result:08x}"));
        };

        Ok(Vst3State {
            component,
            controller,
        })
    }

    /// Restores state in VST3 host order: component chunk, then
    /// `setComponentState` on a separate controller, then controller chunk.
    ///
    /// Control-thread only. VST3 expects state changes while the processor is
    /// stopped or newly created; the caller owns that lifecycle, so this method
    /// does not deactivate the instance itself.
    pub fn load_state(&self, state: &Vst3State) -> Result<(), String> {
        if state.component.is_empty() && state.controller.is_empty() {
            return Ok(());
        }
        self.sync_restart_flags();

        if !state.component.is_empty() {
            let stream = MemoryStream::from_slice(&state.component);
            let stream_ptr = stream
                .to_com_ptr::<IBStream>()
                .ok_or_else(|| String::from("could not create the component state stream"))?;
            let result = unsafe { self.component.setState(stream_ptr.as_ptr()) };
            if result != kResultOk {
                return Err(format!("component setState failed with 0x{result:08x}"));
            }

            // A single-component plug-in is its own controller; applying the
            // component chunk through setComponentState as well would restore
            // the same state twice.
            if self.separate_controller {
                let controller_stream = MemoryStream::from_slice(&state.component);
                let controller_ptr = controller_stream
                    .to_com_ptr::<IBStream>()
                    .ok_or_else(|| String::from("could not create the controller state stream"))?;
                let result = unsafe { self.controller.setComponentState(controller_ptr.as_ptr()) };
                if result != kResultOk && result != kNotImplemented {
                    return Err(format!(
                        "controller setComponentState failed with 0x{result:08x}"
                    ));
                }
            }
        }

        if !state.controller.is_empty() {
            let stream = MemoryStream::from_slice(&state.controller);
            let stream_ptr = stream
                .to_com_ptr::<IBStream>()
                .ok_or_else(|| String::from("could not create the controller state stream"))?;
            let result = unsafe { self.controller.setState(stream_ptr.as_ptr()) };
            if result != kResultOk {
                return Err(format!("controller setState failed with 0x{result:08x}"));
            }
        }

        // State can change latency/tail or parameter values.
        self.refresh_processing_info();
        Ok(())
    }

    /// True when the controller created an `"editor"` view during load.
    pub fn has_editor(&self) -> bool {
        self.has_editor
    }

    /// Attaches the plug-in's native editor to an application-owned HWND.
    ///
    /// The view stays alive until [`Self::close_editor`]. Call from the UI
    /// thread; this never creates an application window itself.
    pub fn open_editor(&self, parent_window: *mut c_void) -> Result<(), String> {
        if parent_window.is_null() {
            return Err(String::from("editor parent window handle is null"));
        }
        let mut slot = self
            .editor
            .lock()
            .map_err(|_| String::from("editor state is poisoned"))?;
        if slot.is_some() {
            return Ok(());
        }
        self.shared
            .editor_hwnd
            .store(parent_window as isize, Ordering::Release);
        match editor::open_session(&self.controller, &self.plug_frame, parent_window) {
            Ok(session) => {
                *slot = Some(session);
                Ok(())
            }
            Err(error) => {
                self.shared.editor_hwnd.store(0, Ordering::Release);
                Err(error)
            }
        }
    }

    /// Detaches and releases the native editor, if any.
    pub fn close_editor(&self) {
        let mut slot = match self.editor.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(session) = slot.take() else {
            return;
        };
        self.shared.editor_hwnd.store(0, Ordering::Release);
        editor::close_session(session);
    }

    /// Resizes the open editor and informs the plug-in.
    pub fn resize_editor(&self, width: u32, height: u32) -> Result<(), String> {
        let mut slot = self
            .editor
            .lock()
            .map_err(|_| String::from("editor state is poisoned"))?;
        let session = slot
            .as_mut()
            .ok_or_else(|| String::from("editor is not open"))?;
        editor::resize_session(session, width, height)
    }

    fn refresh_processing_info(&self) {
        let latency = unsafe { self.processor.getLatencySamples() };
        let tail = unsafe { self.processor.getTailSamples() };
        self.latency.store(latency, Ordering::Relaxed);
        self.tail.store(tail, Ordering::Relaxed);
    }

    fn sync_restart_flags(&self) {
        let flags = self.shared.take_restart_flags();
        if flags & kLatencyChanged != 0 {
            self.refresh_processing_info();
        }
    }

    fn shutdown(&mut self) {
        self.close_editor();
        if self.processing {
            unsafe {
                let _ = self.processor.setProcessing(0);
            }
            self.processing = false;
        }
        if self.active {
            unsafe {
                let _ = self.component.setActive(0);
            }
            self.active = false;
        }
        if let Some(connection) = self.connection.take() {
            unsafe {
                let _ = connection
                    .component
                    .disconnect(connection.controller.as_ptr());
                let _ = connection
                    .controller
                    .disconnect(connection.component.as_ptr());
            }
        }
        if self.controller_initialized {
            unsafe {
                let _ = self.controller.terminate();
            }
            self.controller_initialized = false;
        }
        if self.component_initialized {
            unsafe {
                let _ = self.component.terminate();
            }
            self.component_initialized = false;
        }
    }

    /// Processes one stereo block in place.
    ///
    /// Audio-callback contract: no allocation, locks, I/O or controller calls.
    fn process(&self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        if left.len() != right.len() {
            return Err(String::from(
                "left and right channel lengths must match for a stereo block",
            ));
        }
        if !self.prepared {
            return Err(String::from("VST3 effect is not prepared"));
        }
        let frames = left.len();
        if frames == 0 {
            return Ok(());
        }
        if frames > self.max_block_frames {
            return Err(format!(
                "block of {frames} frames exceeds the prepared maximum of {}",
                self.max_block_frames
            ));
        }

        // SAFETY: the audio callback is serialized and exclusive; only this
        // thread touches the `UnsafeCell` once preparation is complete.
        let buffers = unsafe { &mut *self.buffers.get() };
        if buffers.input_left.len() < frames
            || buffers.input_right.len() < frames
            || buffers.output_left.len() < frames
            || buffers.output_right.len() < frames
        {
            return Err(String::from("prepared VST3 buffers are too small"));
        }

        self.parameter_changes.begin_block();
        while let Some(change) = self.shared.parameters.pop() {
            self.parameter_changes.add(change.id, change.value);
        }

        let input_channels = self.layout.input_channels.max(1) as usize;
        if input_channels == 1 {
            // Mono-in/stereo-out adaption: feed the average of the host stereo
            // signal to the single plug-in input channel.
            for frame in 0..frames {
                let mixed = 0.5 * (left[frame] + right[frame]);
                buffers.input_left[frame] = mixed;
            }
        } else {
            buffers.input_left[..frames].copy_from_slice(left);
            buffers.input_right[..frames].copy_from_slice(right);
        }
        buffers.output_left[..frames].fill(0.0);
        buffers.output_right[..frames].fill(0.0);

        let mut input_pointers = [
            buffers.input_left.as_mut_ptr(),
            buffers.input_right.as_mut_ptr(),
        ];
        let mut output_pointers = [
            buffers.output_left.as_mut_ptr(),
            buffers.output_right.as_mut_ptr(),
        ];

        let Some(input_bus) = buffers.input_buses.get_mut(self.layout.input_index) else {
            return Err(String::from("prepared VST3 input bus index is invalid"));
        };
        input_bus.numChannels = input_channels as int32;
        input_bus.silenceFlags = 0;
        // The pointer array lives on this stack frame for the duration of
        // `process`. Writing a union field is safe in Rust.
        input_bus.__field0.channelBuffers32 = input_pointers.as_mut_ptr();

        let Some(output_bus) = buffers.output_buses.get_mut(self.layout.output_index) else {
            return Err(String::from("prepared VST3 output bus index is invalid"));
        };
        output_bus.numChannels = self.layout.output_channels as int32;
        output_bus.silenceFlags = 0;
        // As above for the output channel pointer array.
        output_bus.__field0.channelBuffers32 = output_pointers.as_mut_ptr();

        let current_frame = buffers.frames_processed as i64;
        buffers.context.sampleRate = self.sample_rate;
        buffers.context.projectTimeSamples = current_frame;
        buffers.context.continousTimeSamples = current_frame;
        buffers.context.state = kPlaying as u32;

        let mut data = ProcessData {
            processMode: kRealtime,
            symbolicSampleSize: kSample32,
            numSamples: frames as int32,
            numInputs: buffers.input_buses.len() as int32,
            numOutputs: buffers.output_buses.len() as int32,
            inputs: buffers.input_buses.as_mut_ptr(),
            outputs: buffers.output_buses.as_mut_ptr(),
            inputParameterChanges: self.parameter_changes_ptr.as_ptr(),
            outputParameterChanges: std::ptr::null_mut(),
            inputEvents: std::ptr::null_mut(),
            outputEvents: std::ptr::null_mut(),
            processContext: &mut buffers.context,
        };

        let result = unsafe { self.processor.process(&mut data) };
        if result != kResultOk && result != kResultFalse {
            return Err(format!(
                "IAudioProcessor::process failed with 0x{result:08x}"
            ));
        }

        left.copy_from_slice(&buffers.output_left[..frames]);
        right.copy_from_slice(&buffers.output_right[..frames]);
        buffers.frames_processed = buffers.frames_processed.wrapping_add(frames as u64);
        Ok(())
    }
}

impl HostedProcessor for Vst3Effect {
    fn process(&self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        Vst3Effect::process(self, left, right)
    }

    fn latency_samples(&self) -> u32 {
        self.latency.load(Ordering::Relaxed)
    }

    fn tail_samples(&self) -> u32 {
        self.tail.load(Ordering::Relaxed)
    }
}

impl Drop for Vst3Effect {
    fn drop(&mut self) {
        self.shutdown();
        // Fields then drop in declaration order; `module` is last.
    }
}

fn validate_prepare_arguments(sample_rate: u32, max_block_frames: u32) -> Result<(), String> {
    if sample_rate == 0 || sample_rate > 1_000_000 {
        return Err(format!("invalid sample rate {sample_rate}"));
    }
    if max_block_frames == 0 || max_block_frames > MAX_BLOCK_FRAMES {
        return Err(format!(
            "invalid maximum block size {max_block_frames}; expected 1..={MAX_BLOCK_FRAMES}"
        ));
    }
    Ok(())
}

fn class_id_to_tuid(class_id: PluginClassId) -> [c_char; 16] {
    std::array::from_fn(|index| class_id.as_bytes()[index] as c_char)
}

fn create_component(
    factory: &ComPtr<IPluginFactory>,
    class_id: PluginClassId,
) -> Result<ComPtr<IComponent>, String> {
    let cid = class_id_to_tuid(class_id);
    let mut instance: *mut c_void = std::ptr::null_mut();
    let result = unsafe {
        factory.createInstance(
            cid.as_ptr() as FIDString,
            IComponent::IID.as_ptr() as FIDString,
            &mut instance,
        )
    };
    if result != kResultOk {
        return Err(format!(
            "factory createInstance(IComponent) failed for class {} with 0x{result:08x}",
            class_id.to_hex()
        ));
    }
    unsafe { ComPtr::<IComponent>::from_raw(instance.cast()) }
        .ok_or_else(|| String::from("factory returned a null component"))
}

fn create_controller(
    factory: &ComPtr<IPluginFactory>,
    class_id: &[c_char; 16],
) -> Result<ComPtr<IEditController>, String> {
    let mut instance: *mut c_void = std::ptr::null_mut();
    let result = unsafe {
        factory.createInstance(
            class_id.as_ptr() as FIDString,
            IEditController::IID.as_ptr() as FIDString,
            &mut instance,
        )
    };
    if result != kResultOk {
        return Err(format!(
            "factory createInstance(IEditController) failed with 0x{result:08x}"
        ));
    }
    unsafe { ComPtr::<IEditController>::from_raw(instance.cast()) }
        .ok_or_else(|| String::from("factory returned a null controller"))
}

fn probe_editor(controller: &ComPtr<IEditController>) -> bool {
    let raw = unsafe { controller.createView(c"editor".as_ptr() as FIDString) };
    unsafe { ComPtr::<IPlugView>::from_raw(raw) }.is_some()
}

fn empty_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: 0,
        title: [0; 128],
        shortTitle: [0; 128],
        units: [0; 128],
        stepCount: 0,
        defaultNormalizedValue: 0.0,
        unitId: 0,
        flags: 0,
    }
}

fn string128(value: &String128) -> String {
    let end = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

fn clamp_normalized(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn arrangement_channels(arrangement: SpeakerArrangement) -> Option<i32> {
    if arrangement == SpeakerArr::kStereo {
        Some(2)
    } else if arrangement == SpeakerArr::kMono {
        Some(1)
    } else if arrangement == SpeakerArr::kEmpty {
        Some(0)
    } else {
        None
    }
}

fn bus_info(component: &ComPtr<IComponent>, direction: i32, index: i32) -> Result<BusInfo, String> {
    let mut info = BusInfo {
        mediaType: kAudio,
        direction,
        channelCount: 0,
        name: [0; 128],
        busType: 0,
        flags: 0,
    };
    let result = unsafe { component.getBusInfo(kAudio, direction, index, &mut info) };
    if result != kResultOk {
        return Err(format!(
            "getBusInfo({direction}, {index}) failed with 0x{result:08x}"
        ));
    }
    Ok(info)
}

fn negotiated_channel_count(
    processor: &ComPtr<IAudioProcessor>,
    direction: i32,
    index: i32,
    fallback: i32,
) -> Result<i32, String> {
    let mut arrangement = SpeakerArr::kEmpty;
    let result = unsafe { processor.getBusArrangement(direction, index, &mut arrangement) };
    if result != kResultOk {
        return Ok(fallback);
    }
    arrangement_channels(arrangement).ok_or_else(|| {
        format!(
            "unsupported speaker arrangement 0x{arrangement:016x} on bus {index} (direction {direction})"
        )
    })
}

fn negotiate_buses(
    component: &ComPtr<IComponent>,
    processor: &ComPtr<IAudioProcessor>,
) -> Result<BusLayout, String> {
    let input_count = unsafe { component.getBusCount(kAudio, kInput) };
    let output_count = unsafe { component.getBusCount(kAudio, kOutput) };
    if !(0..=MAX_BUS_COUNT).contains(&input_count) {
        return Err(format!("invalid audio input bus count {input_count}"));
    }
    if !(1..=MAX_BUS_COUNT).contains(&output_count) {
        return Err(format!(
            "plug-in has no supported audio output bus (count {output_count})"
        ));
    }

    let mut main_inputs = Vec::new();
    for index in 0..input_count {
        let info = bus_info(component, kInput, index)?;
        if info.busType == kMain {
            main_inputs.push((index, info));
        }
    }
    let mut main_outputs = Vec::new();
    for index in 0..output_count {
        let info = bus_info(component, kOutput, index)?;
        if info.busType == kMain {
            main_outputs.push((index, info));
        }
    }

    let [(input_index, input_info)] = main_inputs.as_slice() else {
        return Err(format!(
            "plug-in must expose exactly one main audio input bus (found {})",
            main_inputs.len()
        ));
    };
    let [(output_index, output_info)] = main_outputs.as_slice() else {
        return Err(format!(
            "plug-in must expose exactly one main audio output bus (found {})",
            main_outputs.len()
        ));
    };

    let mut input_arrangements = vec![SpeakerArr::kEmpty; input_count as usize];
    let mut output_arrangements = vec![SpeakerArr::kEmpty; output_count as usize];
    input_arrangements[*input_index as usize] = SpeakerArr::kStereo;
    output_arrangements[*output_index as usize] = SpeakerArr::kStereo;

    let mut set_result = unsafe {
        processor.setBusArrangements(
            input_arrangements.as_mut_ptr(),
            input_count,
            output_arrangements.as_mut_ptr(),
            output_count,
        )
    };
    if set_result != kResultOk && input_info.channelCount <= 1 {
        // Mono-in/stereo-out plug-ins commonly reject a stereo input.
        input_arrangements[*input_index as usize] = SpeakerArr::kMono;
        set_result = unsafe {
            processor.setBusArrangements(
                input_arrangements.as_mut_ptr(),
                input_count,
                output_arrangements.as_mut_ptr(),
                output_count,
            )
        };
    }

    let input_channels =
        negotiated_channel_count(processor, kInput, *input_index, input_info.channelCount)?;
    let output_channels =
        negotiated_channel_count(processor, kOutput, *output_index, output_info.channelCount)?;

    if !(1..=2).contains(&input_channels) {
        return Err(format!(
            "unsupported main input channel count {input_channels}; expected mono or stereo"
        ));
    }
    if output_channels != 2 {
        return Err(format!(
            "unsupported main output channel count {output_channels}; expected stereo"
        ));
    }
    if set_result != kResultOk {
        // Some plug-ins have fixed layouts and return false even when the
        // current arrangement is exactly what the host asked for. Accept that
        // only when both main buses already match.
        if input_channels > 2 || output_channels != 2 {
            return Err(format!(
                "plug-in rejected stereo bus negotiation ({input_channels} in, {output_channels} out) with 0x{set_result:08x}"
            ));
        }
    }

    Ok(BusLayout {
        input_index: *input_index as usize,
        output_index: *output_index as usize,
        input_channels,
        output_channels,
        input_bus_count: input_count as usize,
        output_bus_count: output_count as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_default_is_empty_and_clone_independent() {
        let state = Vst3State::default();
        assert!(state.component.is_empty());
        assert!(state.controller.is_empty());
        let mut copy = state.clone();
        copy.component.push(1);
        copy.controller.extend_from_slice(&[2, 3]);
        assert!(state.component.is_empty());
        assert!(state.controller.is_empty());
        assert_eq!(copy.component, vec![1]);
        assert_eq!(copy.controller, vec![2, 3]);

        let round_trip = Vst3State {
            component: vec![9; 16],
            controller: vec![7; 8],
        };
        assert_eq!(round_trip.clone().component, round_trip.component);
    }

    #[test]
    fn parameter_info_carries_identity_and_normalized_bounds() {
        let info = Vst3ParameterInfo {
            id: 42,
            title: String::from("Threshold"),
            short_title: String::from("Thr"),
            units: String::from("dB"),
            step_count: 0,
            default_normalized: 0.5,
            normalized: 1.0,
        };
        assert_eq!(info.id, 42);
        assert_eq!(info.title, "Threshold");
        assert_eq!(info.short_title, "Thr");
        assert_eq!(info.units, "dB");
        assert_eq!(info.step_count, 0);
        assert_eq!(info.default_normalized, 0.5);
        assert_eq!(clamp_normalized(info.normalized), 1.0);
        assert_eq!(clamp_normalized(f64::NAN), 0.0);
        assert_eq!(clamp_normalized(-1.0), 0.0);
    }

    #[test]
    fn class_id_round_trips_through_vst3_tuid_storage() {
        let bytes = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let id = PluginClassId::from_bytes(bytes);
        let tuid = class_id_to_tuid(id);
        let restored = PluginClassId::from_bytes(std::array::from_fn(|index| tuid[index] as u8));
        assert_eq!(restored, id);
    }

    #[test]
    fn prepare_arguments_are_bounded() {
        assert!(validate_prepare_arguments(48_000, 512).is_ok());
        assert!(validate_prepare_arguments(0, 512).is_err());
        assert!(validate_prepare_arguments(48_000, 0).is_err());
        assert!(validate_prepare_arguments(48_000, MAX_BLOCK_FRAMES + 1).is_err());
    }
}
