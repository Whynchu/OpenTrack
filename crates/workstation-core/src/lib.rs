//! Shared domain model for the Rust audio workstation.
//!
//! The project model, command registry, and saved window layouts will live here
//! so they can be developed independently of the eventual GUI framework.
pub mod commands;
pub mod cpr;
pub mod project;
pub mod workspaces;
