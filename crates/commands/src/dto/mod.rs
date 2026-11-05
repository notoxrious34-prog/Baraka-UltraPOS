//! Data-transfer objects exchanged with the React frontend.
//!
//! All money crosses the bridge as integer centimes (`i64`).
//!
//! Organized by domain (TASK-BRK-106). Every path stays `crate::dto::X`
//! through these re-exports, so no caller changed.

pub mod admin;
pub mod auth;
pub mod cash;
pub mod catalog;
pub mod common;
pub mod customers;
pub mod documents;
pub mod inventory;
pub mod printing;
pub mod reports;
pub mod sales;
pub mod suppliers;

pub use admin::*;
pub use auth::*;
pub use cash::*;
pub use catalog::*;
pub use common::*;
pub use customers::*;
pub use documents::*;
pub use inventory::*;
pub use printing::*;
pub use reports::*;
pub use sales::*;
pub use suppliers::*;
