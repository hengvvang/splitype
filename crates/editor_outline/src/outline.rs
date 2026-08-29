//! editor_outline — the floating outline HUD (consumer crate).
//!
//! Owns the outline panel state ([`OutlineHudState`]); heading *data*
//! comes from the modes through `editor::Pane::outline_items` (pure
//! `OutlineNode` values), so this crate depends only on `editor`. The
//! editor-side sync/navigation glue stays in the coordinating crate
//! until the `Editor` entity converges.

mod state;

pub use state::OutlineHudState;
