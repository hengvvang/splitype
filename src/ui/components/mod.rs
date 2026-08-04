//! Reusable UI components — small, business-free building blocks.
//!
//! Components here must not depend on `editor`, `model`, or `windows`;
//! they only consume `theme` and gpui so any view can reuse them.

pub mod button;
pub mod dialog;
pub mod menu_item;
pub mod popover;
pub mod select;
pub mod switch;
