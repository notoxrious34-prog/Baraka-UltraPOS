//! Baraka UltraPOS infrastructure layer.
//!
//! Owns SQLite persistence (WAL, transactions, embedded migrations) and
//! hardware adapters. No ORM: SQL is hand-authored and embedded.

pub mod adapters;
pub mod persistence;
pub mod printing;

pub use persistence::InfrastructureError;
