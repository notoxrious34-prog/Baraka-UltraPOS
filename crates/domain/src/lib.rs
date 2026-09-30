//! Baraka UltraPOS domain layer.
//!
//! Pure business logic only. No database, no Tauri, no I/O.
//! Money is integer DZD centimes (`Money`); quantities and factors are
//! integer micros at scale 10^4. Zero floating-point math.

pub mod backup;
pub mod cash;
pub mod catalog;
pub mod errors;
pub mod inventory;
pub mod models;
pub mod primitives;
pub mod printing;
pub mod role;
pub mod sales;
pub mod suppliers;
pub mod user;
