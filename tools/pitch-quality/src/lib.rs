//! Headless diagnostics compile the production renderer directly, without device
//! startup or the application/plug-in runtime. No duplicate DSP implementation.
#![allow(dead_code)]
#[path = "../../../crates/opentrack-audio/src/pitch_analysis.rs"]
mod pitch_analysis;
#[path = "../../../crates/opentrack-audio/src/pitch_synthesis.rs"]
mod pitch_synthesis;
