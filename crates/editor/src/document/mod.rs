//! Process-level document source of truth.
//!
//! [`DocumentBuffer`] is the authoritative in-memory state of one open
//! document; [`DocumentStore`] is the process-global registry mapping
//! documents to buffers and paths to documents. Every editor view (tab) is a
//! shallow reference to a buffer — never a private text copy — so split
//! editors and cloned windows always observe the same content.

pub mod buffer;
pub mod store;

pub use buffer::DocumentBuffer;
pub use store::{DocumentStore, PersistedDocument};
