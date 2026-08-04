//! Editor window chrome state — menus, dialogs, context menus, status bar.
//!
//! These are pure state records; their rendering lives in `ui::window`.

use std::path::PathBuf;

use gpui::{EntityId, Pixels, Point, Task};

use crate::editor::controller::{InfoDialogKind, TableAxisSelection};

/// Mutable render state tracked across frames for hover effects.
#[derive(Default)]
pub struct StatusBarState {
    pub sidebar_hovered: bool,
    pub mode_hovered: bool,
    pub custom_button_hovered: Option<String>,
}

/// Target block position for inserting a native table.
#[derive(Clone, Copy)]
pub enum TableInsertTarget {
    /// Insert the table immediately after the referenced block.
    After(EntityId),
    /// Append the table to the end of the current root list.
    Append,
}

/// Rendered-mode context menu currently open in the editor.
#[derive(Clone)]
pub enum ContextMenuState {
    /// General block context menu with an insert submenu.
    Insert {
        position: Point<Pixels>,
        target: TableInsertTarget,
        insert_hovered: bool,
        submenu_hovered: bool,
        submenu_open: bool,
    },
    /// Table row or column context menu for an existing native table.
    TableAxis {
        position: Point<Pixels>,
        selection: TableAxisSelection,
    },
    /// Workspace file or folder context menu.
    WorkspaceFile {
        position: Point<Pixels>,
        path: PathBuf,
        is_dir: bool,
    },
}

/// State for the table insertion dialog opened from the context menu.
pub struct TableInsertDialogState {
    pub target: TableInsertTarget,
    pub body_rows: usize,
    pub columns: usize,
}

/// Editor window chrome: menus, dialogs, status bar, and informational
/// overlays. Rendering lives in `ui::window`.
#[derive(Default)]
pub struct WindowChrome {
    pub(crate) status_bar: StatusBarState,
    pub(crate) context_menu: Option<ContextMenuState>,
    pub(crate) table_insert_dialog: Option<TableInsertDialogState>,
    pub(crate) context_menu_submenu_close_task: Option<Task<()>>,
    /// Open top-level menu in the in-window fallback menu bar.
    pub(crate) menu_bar_open: Option<usize>,
    pub(crate) menu_bar_expanded: bool,
    /// Open child submenu inside the in-window fallback menu panel.
    pub(crate) menu_submenu_open: Option<usize>,
    pub(crate) menu_bar_hovered: bool,
    pub(crate) menu_panel_hovered: bool,
    pub(crate) menu_submenu_panel_hovered: bool,
    /// Hover state for the invisible bridge spanning the gap between the menu
    /// panel and an open submenu. Tracked separately from
    /// `menu_submenu_panel_hovered` so the handoff between the two regions
    /// cannot clobber a single shared flag and tear the menu down.
    pub(crate) menu_submenu_bridge_hovered: bool,
    pub(crate) menu_close_task: Option<Task<()>>,
    /// Optional informational dialog shown from the Help menu.
    pub(crate) info_dialog: Option<InfoDialogKind>,
    /// True while an online update check is running in the background.
    pub(crate) update_check_in_progress: bool,
}
