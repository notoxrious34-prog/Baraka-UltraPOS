//! Bounded ESC/POS decoder (re-exported from the domain crate: pure byte
//! parsing with no infrastructure dependencies, so application use cases
//! can decode captures directly).
pub use baraka_domain::printing::escpos_decode;
