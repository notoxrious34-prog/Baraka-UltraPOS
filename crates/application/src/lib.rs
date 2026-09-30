//! Baraka UltraPOS application layer.
//!
//! Orchestrates domain validation. Depends on `baraka-domain` only.
//! Never depends on infrastructure, commands, rusqlite, or Tauri.

pub mod errors;
pub mod ports;
pub mod services;
pub mod use_cases;
