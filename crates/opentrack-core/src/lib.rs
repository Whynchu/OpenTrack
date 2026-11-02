//! Shared domain model for the Rust OpenTrack DAW.
//!
//! The project model, command registry, and saved window layouts will live here
//! so they can be developed independently of the eventual GUI framework.
pub mod arranger;
pub mod automation;
pub mod commands;
pub mod cpr;
pub mod drum_map;
pub mod event_query;
pub mod media_folders;
pub mod midi_file;
pub mod midi_quantize;
pub mod midi_transform;
pub mod musicxml;
pub mod pitch_time;
pub mod project;
pub mod routing;
pub mod workspaces;
