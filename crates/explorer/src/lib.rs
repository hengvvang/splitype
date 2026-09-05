//! Explorer — the file-tree panel plugin.
//!
//! # Architecture
//!
//! - **State & model** live in [`state`]: the [`state::worktree`] scan
//!   entities, the file-tree nodes / visible rows / selection, the
//!   [`state::undo`] manager, and the background helpers. Scanned trees are
//!   process-level shared resources registered in [`state::store::WorktreeStore`]
//!   (one `Worktree` per folder root, reference-counted by views); each
//!   panel instance owns its own [`ExplorerState`] view state (visible
//!   roots, expansion, selection, drag state), so split and multi-window
//!   panels share one scanned tree per folder while keeping independent
//!   view state. The panel never touches the window shell, and shell
//!   interactions go through the [`platform_contracts`] host seams.
//! - **Selection** is keyed by the stable double key
//!   `(worktree id, entry id)`; ids survive renames and moves, so
//!   selection and expansion state survive rescans. Multi-select marks,
//!   range selection, and keyboard navigation live in [`ops::selection`].
//! - **File operations** ([`ops::file_ops`], [`state::undo`]) cover delete /
//!   trash / cut / copy / paste / duplicate with an undo manager; the
//!   filesystem helpers run on a background thread through [`fs`].
//! - **Drag & drop** ([`ops::drag_and_drop`]) provides edge auto-scroll with
//!   proximity-based speed, move-vs-copy cursor, hover expansion of collapsed
//!   directories, highlight of the drop target and its descendants, external
//!   file drops with collision prompts, and dragging worktree roots to
//!   reorder them.
//! - **Rendering** ([`render`]) virtualizes the file tree and draws the
//!   root rows with their title buttons and the inline edit row.
//! - **Shell hooks** ([`plugin`]) export the functions the composition root
//!   registers to push document context into this panel and to dispatch its
//!   commands (toggle tree, close folder scope) by kind.
//!
//! The panel never imports the editor family, and vice versa.

pub mod assets;
pub mod bottombar;
pub mod filename_editor;
pub mod fs;
pub mod lifecycle;
pub mod ops;
pub mod persist;
pub mod plugin;
pub mod render;
pub mod settings;
pub mod state;
pub mod topbar;

pub use plugin::*;
