//! `splitype-primitives` — Shared domain primitives, error contracts, and core utilities.
//!
//! This crate is zero-I/O and zero-UI, providing foundational types across
//! all other splitype crates.

pub mod callout;
pub mod error;
pub mod id;

pub use callout::CalloutKind;
pub use error::{CoreError, CoreResult};
pub use id::{BlockId, DocumentId, ElementId, NodeId};

/// Reverse-DNS application id used by GPUI, desktop launchers, and bundles.
pub const SPLITYPE_APP_ID: &str = "com.hengvvang.splitype";
