//! Pure filesystem abstraction shared by the explorer panel and the editor
//! file lifecycle.
//!
//! This crate is zero-UI and zero-GPUI: it only knows paths and filesystem
//! facts, and every function is safe to run on a background thread.

pub mod error;
pub mod ops;

pub use error::FsError;
pub use ops::*;
