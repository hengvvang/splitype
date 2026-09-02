//! Split-layout visuals: divider bars, corner drag handles, border context
//! menus and corner-drag gesture previews.
//!
//! Everything here is pure presentation over [`splitter`] facts: the
//! functions receive the engine's state plus injected styles and callbacks
//! and return elements — they never mutate layout topology. The engine
//! ([`splitter`]) renders nothing; hosts (the window shell, the editor)
//! drive the gestures and apply their results.
//!
//! The module is named `split` (not `splitter`) so it doesn't shadow the
//! `splitter` crate in `use` paths inside this crate.

pub mod chrome;
pub mod drag_preview;

pub use chrome::{OverlayStyle, border_menu_style, overlay_container};
pub use drag_preview::render_corner_drag_preview;
