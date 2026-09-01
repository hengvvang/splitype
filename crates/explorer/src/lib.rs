//! Explorer — the file-tree panel plugin.
//!
//! # Architecture
//!
//! - **State & model** live in [`state`]: the [`state::worktree`] scan
//!   entities, the file-tree nodes / visible rows / selection, the
//!   [`state::undo`] manager, and the background helpers. Every panel
//!   instance owns its own [`ExplorerState`] entity (one per
//!   `ExplorerPanelView`), so split and multi-window panels never share
//!   tree state; the panel never touches the window shell, and shell
//!   interactions go through the [`platform_contracts`] host seams.
//! - **Selection** is keyed by the stable double key
//!   `(worktree index, entry id)`; ids survive renames and moves, so
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

pub use bottombar::render_explorer_bottombar;
pub use filename_editor::buffer::{utf8_range_to_utf16_in, utf16_range_to_utf8_in};
pub use filename_editor::element::shape_filename_line;
pub use plugin::*;
pub use render::{render_explorer_body, render_explorer_file_context_menu};
pub use settings::ExplorerSettings;
pub use state::{ExplorerFileMenuState, ExplorerState};
pub use topbar::render_explorer_topbar;
