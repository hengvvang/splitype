//! Document editing runtime — controller, document tree, block entities,
//! undo/selection/viewport logic, and block-event protocol.

pub mod actions;
pub mod block;
pub mod chrome;
pub mod controller;
pub mod document;
pub mod footnotes;
pub mod history;
pub mod layout;
pub mod loader;
pub mod panels;
pub mod projection;
pub mod runtime;
pub mod selection;
pub mod serialize;
pub mod source_map;
pub mod table;
pub mod text_layout;
pub mod viewport;
pub mod workspace;

// ── Re-exports ─────────────────────────────────────────────────────────────
pub(crate) use controller::{EditorMode, RenderedSelectAllCycle};
