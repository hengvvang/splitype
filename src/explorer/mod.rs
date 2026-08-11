//! Explorer — the file-tree sidebar view, a Zed `project_panel` port.
//!
//! # Architecture (mirroring Zed's project panel)
//!
//! - **State & model** live in `crate::editor::explorer_state` (owned by
//!   the Editor entity): the [`worktree`] scan entities, the [`state`]
//!   (file-tree nodes, visible rows, selection), the [`undo`] manager, and
//!   the background [`utils`] helpers. This view module depends on them
//!   one-way — the editor never imports `crate::explorer`.
//! - **Tree state** ([`tree`]) caches one [`ExplorerFileNode`] per worktree
//!   (indexed identically to `worktrees`, so expansion sets and selections
//!   keyed by index never drift), holds the per-worktree expansion sets, and
//!   derives the flat list of [`VisibleExplorerEntry`] rows
//!   (`build_explorer_rows`) that the virtualized `uniform_list` renders.
//! - **Selection** ([`selection`]) is keyed by the Zed-style double key
//!   `(worktree index, stable entry id)`; ids survive renames and moves, so
//!   selection and expansion state survive rescans. Multi-select marks,
//!   range selection, and keyboard navigation live here.
//! - **File operations** ([`file_ops`], [`undo`]) cover delete / trash /
//!   cut / copy / paste / duplicate with a Zed-style undo manager; the
//!   filesystem helpers run on a background thread ([`utils`]).
//! - **Drag & drop** ([`drag_and_drop`]) mirrors Zed's panel: edge
//!   auto-scroll with proximity-based speed, move-vs-copy cursor, hover
//!   expansion of collapsed directories, highlight of the drop target and
//!   its descendants, external file drops with collision prompts, and
//!   dragging worktree roots to reorder them.
//! - **Rendering** ([`render`]) virtualizes the file tree and draws the
//!   root rows with their title buttons and the inline edit row.
//!
//! The outline panel (headings) keeps its own state in
//! `crate::editor::outline`; the sidebar state and file-tree model
//! live in `crate::editor::explorer_state`.

pub(crate) mod bottombar;
pub(crate) mod drag_and_drop;
pub(crate) mod file_ops;
pub(crate) mod filename_editor;
pub(crate) mod open;
pub(crate) mod panel;
pub(crate) mod render;
pub(crate) mod selection;
pub(crate) mod topbar;
pub(crate) mod tree;

#[cfg(test)]
mod interaction_probe;
