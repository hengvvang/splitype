//! editor_outline — the floating outline HUD (consumer crate).
//!
//! Owns the outline panel state ([`OutlineHudState`]) and the HUD
//! presentation ([`render_floating_outline_hud`]); heading *data* comes
//! from the modes through `editor_model::Pane::outline_items` (pure `OutlineNode`
//! values), so this crate depends only on `editor` plus presentation
//! deps. Navigation and hover re-enter the coordinating crate through
//! [`OutlineHost`]; the editor-side sync glue stays in the coordinating
//! crate.

mod render;
mod state;

pub use render::{OutlineHost, render_floating_outline_hud};
pub use state::{OutlineHudState, OutlineNode};
pub type OutlineHeading = OutlineNode;
