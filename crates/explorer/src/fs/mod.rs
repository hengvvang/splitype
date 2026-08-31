//! Filesystem primitives for the explorer panel — zero-UI, zero-GPUI.
//!
//! Every function only knows paths and filesystem facts and is safe to run
//! on a background thread.

pub mod error;
pub mod ops;

pub use error::FsError;
pub use ops::*;
