//! Document editing runtime — controller, document tree, block entities,
//! undo/selection/viewport logic, and block-event protocol.

pub mod actions;
pub mod controller;
pub mod editing;
pub mod render;
pub mod tree;
pub mod viewport;
pub mod views;
pub mod window;

// ── Re-exports ─────────────────────────────────────────────────────────────
pub(crate) use controller::EditorMode;
