//! Document editing runtime — controller, document tree, block entities,
//! undo/selection/layout-geometry logic, and block-event protocol.
//!
//! # Layout
//!
//! - `controller.rs` — the `Editor` entity and its state aggregation
//!   (facade for the rest of the crate).
//! - `tree/` — the document tree: `block.rs` (entity + core behavior) with
//!   the edit-mode enum, tree-metadata flags, inline-projection engine, and
//!   code-language input split into sibling files.
//! - `editing/` — editing behaviors: `input/` (keyboard, focus, typing,
//!   paste, IME, block-event routing), history, projection, selection,
//!   source mapping, and the table runtime.
//! - `panels/` — the inner panels (WYSIWYG / source / preview / outline)
//!   plus the sidebar and settings panel state.
//! - `topbar/` / `bottombar/` — per-area chrome of an Editor area.
//! - `window/` — the window-level render flow and floating overlays
//!   (context menu, dialogs, export).
//! - `window_layout.rs` — outer tiled-area rendering and the `WindowPanels`
//!   aggregate; `render/` and `geometry/` are content pipelines and math.

pub mod actions;
pub mod block_protocol;
pub(crate) mod bottombar;
pub mod commands;
pub mod controller;
pub mod editing;
pub mod file;
pub mod geometry;
pub mod keybindings;
pub(crate) mod menu_bar;
pub mod panels;
pub mod render;
pub(crate) mod topbar;
pub mod tree;
pub(crate) mod window;
pub(crate) mod window_layout;

#[cfg(test)]
mod tests;
