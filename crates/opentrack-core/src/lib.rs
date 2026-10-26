//! Shared domain model for the Rust OpenTrack DAW.
//!
//! The project model, command registry, and saved window layouts will live here
//! so they can be developed independently of the eventual GUI framework.
pub mod commands;
pub mod cpr;
pub mod pitch_time;
pub mod project;
pub mod workspaces;
