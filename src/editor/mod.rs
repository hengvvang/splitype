//! Document editing runtime — controller, document tree, block entities,
//! undo/selection/layout-geometry logic, and block-event protocol.

pub mod actions;
pub mod controller;
pub mod editing;
pub mod geometry;
pub mod panels;
pub mod render;
pub mod tree;
pub mod window;

// ── Re-exports ─────────────────────────────────────────────────────────────
pub(crate) use controller::EditorMode;
