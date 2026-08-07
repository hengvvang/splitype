//! Explorer — the file-tree sidebar, a Zed `project_panel` port.
//!
//! # Architecture (mirroring Zed's project panel)
//!
//! - **Scanning** happens on a background thread inside the [`worktree`]
//!   entity, which owns an immutable entry snapshot, a stable id allocator
//!   shared across worktrees, a recursive fs watcher and debounced rescans.
//!   The panel never mutates a worktree; it consumes `snapshot()` clones and
//!   rebuilds its visible list when `WorktreeEvent::UpdatedEntries` fires.
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
//! - **Rendering** ([`render`]) virtualizes the file tree, draws the root
//!   rows with their title buttons, the inline edit row and the outline.
//!
//! The outline tree (headings) shares the sidebar state but keeps its own
//! string-keyed expansion set; its parsing lives in
//! `crate::editor::panels::outline`.

pub(crate) mod drag_and_drop;
pub(crate) mod file_ops;
pub(crate) mod filename_editor;
pub(crate) mod open;
pub(crate) mod panel;
pub(crate) mod render;
pub(crate) mod selection;
pub(crate) mod state;
pub(crate) mod tree;
pub(crate) mod undo;
pub(crate) mod utils;
pub(crate) mod worktree;
