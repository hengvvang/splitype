//! Reusable UI components — small, business-free building blocks.
//!
//! Components here must not depend on `editor` or `model`; they only consume
//! `infra::theme`, `platform`, and gpui so any view can reuse them.

pub mod bottombar;
pub mod button;
pub mod corner_drag_preview;
pub mod custom_titlebar;
pub mod dialog;
pub mod empty_state;
pub mod menu_bar;
pub mod menu_item;
pub mod popover;
pub mod section;
pub mod select;
pub mod stepper;
pub mod switch;
pub mod tab;
pub mod topbar;

pub use corner_drag_preview::render_corner_drag_preview;
