//! Outline contract: heading data ([`OutlineNode`]), the HUD cache state
//! ([`OutlineHudState`]), and the navigation seam ([`OutlineHost`]).
//!
//! Presentation lives in the `ui` crate (`ui::render_floating_outline_hud`),
//! which consumes these data types without pulling in any plugin crate.

pub mod host;
mod state;

pub use host::OutlineHost;
pub use state::{OutlineHudState, OutlineNode};
