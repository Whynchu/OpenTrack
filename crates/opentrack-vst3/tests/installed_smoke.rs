// SPDX-License-Identifier: Apache-2.0

//! Opt-in smoke test against an installed VST3 effect.
//!
//! Normal `cargo test -p opentrack-vst3` never requires a plug-in. Run with:
//!
//! ```text
//! cargo test -p opentrack-vst3 --test installed_smoke -- --ignored --nocapture
//! ```
//!
//! `OPENTRACK_VST3_SMOKE_PLUGIN` overrides the module path (default: TDR Nova).
//! Set `OPENTRACK_VST3_EDITOR_HWND` to a decimal or `0x`-prefixed HWND to also
//! exercise native editor open/resize/close.

use std::env;
use std::ffi::c_void;
use std::path::PathBuf;

use opentrack_plugins::{HostedMidiEvent, HostedProcessor, PluginCategory, PluginClassId};
use opentrack_vst3::{Vst3Effect, Vst3Module};

const DEFAULT_PLUGIN: &str = r"C:\Program Files\Common Files\VST3\TDR Nova.vst3";
const DEFAULT_MULTI_OUT_INSTRUMENT: &str =
    r"C:\Program Files\Common Files\VST3\Toontrack\Superior Drummer 3.vst3";
const DEFAULT_WAVES_PLUGIN: &str =
    r"C:\Program Files\Common Files\VST3\WaveShell1-VST3 15.0_x64.vst3";
const WAVES_CLA76_STEREO: &str = "37545356433663536c612d3736207374";

fn plugin_path() -> PathBuf {
    env::var_os("OPENTRACK_VST3_SMOKE_PLUGIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLUGIN))
}

fn waves_plugin_path() -> PathBuf {
    env::var_os("OPENTRACK_VST3_WAVES_SMOKE_PLUGIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_WAVES_PLUGIN))
}

#[test]
#[ignore = "requires an installed multi-output VST3 instrument"]
fn installed_multi_output_instrument_negotiates_and_processes_each_bus() {
    let path = env::var_os("OPENTRACK_VST3_MULTI_OUT_INSTRUMENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MULTI_OUT_INSTRUMENT));
    if !path.exists() {
        eprintln!(
            "skipping multi-output instrument smoke test: {} does not exist (set OPENTRACK_VST3_MULTI_OUT_INSTRUMENT)",
            path.display()
        );
        return;
    }

    let module = Vst3Module::open(&path).expect("multi-output instrument module opens");
    let class = module
        .enumerate()
        .expect("factory enumerates")
        .classes
        .into_iter()
        .find(|class| class.plugin_category() == PluginCategory::Instrument)
        .unwrap_or_else(|| panic!("no instrument class in {}", path.display()));
    eprintln!(
        "multi-output smoke class: {} ({})",
        class.name, class.class_id
    );
    let effect_mode = Vst3Effect::load(&path, class.class_id, 48_000, 256)
        .expect("ordinary instance loads with the main output only");
    let effect_buses = HostedProcessor::output_buses(&effect_mode);
    assert_eq!(effect_buses.len(), 1, "ordinary instances select main only");
    drop(effect_mode);

    let effect = Vst3Effect::load_instrument(&path, class.class_id, 48_000, 256)
        .expect("instrument loads with main and auxiliary outputs");
    let buses = HostedProcessor::output_buses(&effect);
    for bus in &buses {
        eprintln!(
            "  bus {} '{}' ({} channel(s))",
            bus.index, bus.name, bus.channels
        );
    }
    assert!(
        buses.len() > 1,
        "instrument must expose multiple routed buses"
    );

    let note = [HostedMidiEvent {
        sample_offset: 0,
        note_id: 60,
        channel: 9,
        pitch: 36,
        velocity: 110,
        note_on: true,
    }];
    let mut input_left = vec![0.0; 256];
    let mut input_right = vec![0.0; 256];
    let mut output_left = vec![0.0; 256 * buses.len()];
    let mut output_right = vec![0.0; 256 * buses.len()];
    let mut bus_peaks = vec![0.0_f32; buses.len()];
    for block in 0..8 {
        let midi = if block == 0 { &note[..] } else { &[] };
        effect
            .process_routed_with_events(
                &mut input_left,
                &mut input_right,
                &mut output_left,
                &mut output_right,
                buses.len(),
                &[],
                midi,
            )
            .expect("main and auxiliary outputs process into separate planes");
        for (bus, (left, right)) in output_left
            .chunks_exact(256)
            .zip(output_right.chunks_exact(256))
            .enumerate()
        {
            bus_peaks[bus] = bus_peaks[bus].max(
                left.iter()
                    .chain(right)
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs())),
            );
        }
    }
    eprintln!("  MIDI note bus peaks: {bus_peaks:?}");
    // Some instrument plug-ins initialize with no kit/preset loaded, so the
    // smoke test records peaks but only requires valid processing and buffers.
    assert!(
        output_left
            .iter()
            .chain(&output_right)
            .all(|sample| sample.is_finite())
    );
}

fn editor_hwnd() -> Option<isize> {
    let text = env::var("OPENTRACK_VST3_EDITOR_HWND").ok()?;
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let value = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).ok()?
    } else {
        text.parse::<usize>().ok()?
    };
    (value != 0).then_some(value as isize)
}

fn stereo_sine(frames: usize, offset: f32) -> (Vec<f32>, Vec<f32>) {
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    for frame in 0..frames {
        let phase = offset + frame as f32 * 0.11;
        left.push(phase.sin() * 0.1);
        right.push((phase * 0.5).cos() * 0.1);
    }
    (left, right)
}

#[test]
#[ignore = "requires an installed VST3 effect"]
fn installed_plugin_processes_audio_and_round_trips_state() {
    let path = plugin_path();
    if !path.exists() {
        eprintln!(
            "skipping installed smoke test: {} does not exist (set OPENTRACK_VST3_SMOKE_PLUGIN)",
            path.display()
        );
        return;
    }

    let module = Vst3Module::open(&path).expect("module opens");
    let enumeration = module.enumerate().expect("factory enumerates");
    for class in &enumeration.classes {
        eprintln!(
            "  class {} '{}' category {:?} sub '{}'",
            class.class_id,
            class.name,
            class.plugin_category(),
            class.subcategories
        );
    }
    let class = enumeration
        .classes
        .iter()
        .find(|class| class.plugin_category() == PluginCategory::Effect)
        .unwrap_or_else(|| panic!("no audio effect class in {}", path.display()));
    let class_id = class.class_id;
    let name = class.name.clone();
    drop(module);
    eprintln!("smoke test class: {name} ({class_id})");

    let effect = Vst3Effect::load(&path, class_id, 48_000, 64).expect("effect loads and prepares");
    eprintln!(
        "prepared: latency {} samples, tail {} samples, editor {}",
        effect.latency_samples(),
        effect.tail_samples(),
        effect.probe_editor()
    );

    let parameters = effect.parameters();
    eprintln!("enumerated {} parameters", parameters.len());
    assert!(
        !parameters.is_empty(),
        "plug-in should expose generic parameters"
    );
    for parameter in parameters.iter().take(5) {
        eprintln!(
            "  {:>4} {:<24} default {:.3} current {:.3} steps {}",
            parameter.id,
            parameter.title,
            parameter.default_normalized,
            parameter.normalized,
            parameter.step_count
        );
        assert!(parameter.normalized.is_finite());
        assert!((0.0..=1.0).contains(&parameter.normalized));
    }

    // Exercise a generic parameter edit, including the audio-thread queue.
    if let Some(parameter) = parameters
        .iter()
        .find(|parameter| parameter.step_count == 0)
    {
        let original = effect
            .parameter_value(parameter.id)
            .expect("known parameter value");
        let target = if original < 0.5 { 0.75 } else { 0.25 };
        effect.set_parameter(parameter.id, target);
        let updated = effect
            .parameter_value(parameter.id)
            .expect("parameter still exists");
        eprintln!(
            "parameter {} changed {:.3} -> {:.3} (requested {:.3})",
            parameter.id, original, updated, target
        );
        assert!(updated.is_finite());
        assert!(
            (updated - target).abs() < 0.25,
            "controller did not accept the normalized edit"
        );
    }

    let mut left = Vec::new();
    let mut right = Vec::new();
    for block in 0..4 {
        let (mut block_left, mut block_right) = stereo_sine(64, block as f32 * 0.7);
        effect
            .process(&mut block_left, &mut block_right)
            .expect("prepared block processes");
        assert!(
            block_left
                .iter()
                .chain(block_right.iter())
                .all(|sample| sample.is_finite()),
            "plug-in produced non-finite output"
        );
        left = block_left;
        right = block_right;
    }
    eprintln!(
        "last block peak: {:.6} / {:.6}",
        left.iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs())),
        right
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
    );

    let state = effect
        .save_state()
        .expect("save component/controller state");
    eprintln!(
        "saved state: {} component bytes, {} controller bytes",
        state.component.len(),
        state.controller.len()
    );
    assert!(
        !state.component.is_empty(),
        "component getState returned an empty chunk"
    );
    // Some controllers keep all state in the component chunk and return
    // kNotImplemented from IEditController::getState.

    effect.load_state(&state).expect("state reloads");
    let restored = effect.save_state().expect("state saves again after reload");
    assert!(!restored.component.is_empty());
    eprintln!(
        "re-saved state: {} component bytes; byte-identical to the original: {}",
        restored.component.len(),
        restored.component == state.component
    );

    let (mut left, mut right) = stereo_sine(64, 9.0);
    effect
        .process(&mut left, &mut right)
        .expect("processing works after state reload");
    assert!(
        left.iter()
            .chain(right.iter())
            .all(|sample| sample.is_finite())
    );

    if let Some(hwnd) = editor_hwnd() {
        assert!(
            effect.probe_editor(),
            "editor HWND supplied but the plug-in reports no editor"
        );
        effect
            .open_editor(hwnd as *mut c_void)
            .expect("native editor opens on the supplied HWND");
        effect
            .resize_editor(900, 600)
            .expect("native editor resizes");
        effect.close_editor();
        eprintln!("native editor opened, resized and closed on HWND 0x{hwnd:x}");
    } else {
        eprintln!("editor not exercised: set OPENTRACK_VST3_EDITOR_HWND to an HWND");
    }
}

#[test]
#[ignore = "requires an installed WaveShell VST3 plug-in"]
fn installed_waves_cla76_single_component_smoke() {
    let path = waves_plugin_path();
    if !path.exists() {
        eprintln!(
            "skipping WaveShell smoke test: {} does not exist (set OPENTRACK_VST3_WAVES_SMOKE_PLUGIN)",
            path.display()
        );
        return;
    }
    let class_id = PluginClassId::from_hex(WAVES_CLA76_STEREO).expect("CLA-76 class ID parses");

    let effect = Vst3Effect::load(&path, class_id, 48_000, 64)
        .expect("WaveShell CLA-76 loads as a single-component effect");
    eprintln!(
        "CLA-76: latency {} tail {} editor {}",
        effect.latency_samples(),
        effect.tail_samples(),
        effect.probe_editor()
    );

    let parameters = effect.parameters();
    eprintln!("CLA-76 parameters: {}", parameters.len());
    assert!(
        parameters.len() >= 100,
        "WaveShell CLA-76 should expose its full parameter list (got {})",
        parameters.len()
    );
    let first = &parameters[0];
    eprintln!(
        "  first parameter {} '{}' normalized {:.3}",
        first.id, first.title, first.normalized
    );
    assert!(first.normalized.is_finite());

    let (mut left, mut right) = stereo_sine(64, 1.0);
    effect
        .process(&mut left, &mut right)
        .expect("CLA-76 processes");
    assert!(
        left.iter()
            .chain(right.iter())
            .all(|sample| sample.is_finite()),
        "CLA-76 produced non-finite output"
    );
    eprintln!(
        "CLA-76 output peak: {:.6} / {:.6}",
        left.iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs())),
        right
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
    );

    let state = effect.save_state().expect("CLA-76 state saves");
    eprintln!(
        "CLA-76 state: {} component bytes, {} controller bytes",
        state.component.len(),
        state.controller.len()
    );
    assert!(!state.component.is_empty());
    effect.load_state(&state).expect("CLA-76 state reloads");
    let (mut left, mut right) = stereo_sine(64, 5.0);
    effect
        .process(&mut left, &mut right)
        .expect("CLA-76 processes after state reload");
    assert!(
        left.iter()
            .chain(right.iter())
            .all(|sample| sample.is_finite())
    );
}
