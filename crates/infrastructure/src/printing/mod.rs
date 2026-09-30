//! Infrastructure printing modules (pure-Rust raster pipeline).
//!
//! No C toolchain, no system libraries, no async runtime anywhere here.

pub mod blocks;
pub mod capture;
pub mod decode;
pub mod font_cache;
pub mod rasterizer;
pub mod shaping;
pub mod transport_serial;
pub mod worker_thread;
