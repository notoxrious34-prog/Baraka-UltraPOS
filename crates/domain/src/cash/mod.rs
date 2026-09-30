//! Cash session domain: status, session, and movement primitives.
//!
//! Pure business logic only. Money is integer DZD centimes wrapped in
//! [`Money`]; no primitive integers are exposed directly for money.
//! SQLite is the source of truth; this module validates transitions.

pub mod cash_movement;
pub mod cash_session;

pub use cash_movement::{CashMovement, MovementType};
pub use cash_session::{CashSession, CashSessionStatus};
