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
//!   editor panes (each owns its rendering and behavior).
//! - `session.rs` / `session_ops.rs` / `pane_layout.rs` — the per-panel
//!   `EditorSession` aggregate (tab list + pane split tree), its
//!   operations, and its rendering.
//! - `view/` — the editor's document-view render flow and its content-level
//!   overlays (context menu, table-insert dialog); `render/` and
//!   `geometry/` are content pipelines and math.
//! - `topbar/` / `bottombar/` — the editor panel's chrome.

pub mod actions;
pub mod block_protocol;
pub(crate) mod bottombar;
pub mod commands;
pub mod controller;
pub(crate) mod corner_drag_preview;
pub mod editing;
pub mod file;
pub mod geometry;
pub mod keybindings;
pub(crate) mod menu_actions;
pub(crate) mod outline;
pub(crate) mod pane_layout;
pub mod preview;
pub mod render;
pub(crate) mod session;
pub(crate) mod session_ops;
pub(crate) mod source_code;
pub(crate) mod topbar;
pub mod tree;
pub(crate) mod view;
pub mod wysiwyg;

pub(crate) use preview::PreviewState;
pub(crate) use source_code::SourceCodePanelRuntime;

#[cfg(test)]
mod tests;
