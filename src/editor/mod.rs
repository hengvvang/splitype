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
//! - `wysiwyg/` / `source_code/` / `preview/` / `outline/` — the four
//!   editor display modes (each owns its rendering and behavior).
//! - `session.rs` / `session_ops.rs` — the per-area `EditorSession`
//!   aggregate (tab list + panel split tree) and its operations;
//!   `panel_layout.rs` renders that split tree.
//! - `explorer/` / `settings.rs` — sidebar state owned by the window
//!   (`WindowPanels`); the views live in the top-level `src/explorer` /
//!   `src/settings` and depend on these one-way.
//! - `topbar/` / `bottombar/` — per-area chrome of an Editor area.
//! - `window/` — the window-level render flow and floating overlays
//!   (context menu, dialogs, export); `window_layout.rs` — outer tiled-area
//!   rendering and the `WindowPanels` aggregate; `render/` and `geometry/`
//!   are content pipelines and math.

pub mod actions;
pub mod block_protocol;
pub(crate) mod bottombar;
pub mod commands;
pub mod controller;
pub(crate) mod corner_drag_preview;
pub(crate) mod drag_policy;
pub mod editing;
pub(crate) mod explorer;
pub mod file;
pub mod geometry;
pub mod keybindings;
pub(crate) mod menu_bar;
pub(crate) mod outline;
pub(crate) mod panel_layout;
pub mod preview;
pub mod render;
pub(crate) mod session;
pub(crate) mod session_ops;
pub mod settings;
pub(crate) mod source_code;
pub(crate) mod topbar;
pub mod tree;
pub(crate) mod window;
pub(crate) mod window_layout;
pub mod wysiwyg;

pub(crate) use preview::PreviewState;
pub(crate) use source_code::SourceCodePanelRuntime;

#[cfg(test)]
mod tests;
