//! Window panel module — the panel-kind vocabulary, the window-level split
//! root, and the window panel state aggregate.
//!
//! - [`WindowPanelKind`] — the top-level panel types of the tiled split
//!   layout (Explorer / Settings / Editor). The editor's inner panes use
//!   their own kind (`crate::editor::session::EditorPaneKind`) on the same
//!   split engine.
//! - [`WindowLayout`] / [`default_layout`] — the window-level split root.
//! - [`WindowPanels`] — the sidebar and tiled-layout state owned by the
//!   Shell (pure state records; rendering lives in `crate::explorer`,
//!   `crate::settings`, and the editor's own render flow).

use splitype_splitter::container::SplitterContainer;
use splitype_splitter::policy::DragPolicy;
use splitype_splitter::root::SplitterRoot;
use splitype_splitter::tree::NodeId;

use crate::explorer::state::state::ExplorerState;
use crate::settings::state::SettingsUiState;

/// Top-level panel types in the tiled split layout: the window-level
/// panels (each a split-tree leaf). The editor's inner panes are a
/// separate kind vocabulary on the same engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowPanelKind {
    /// File explorer / file tree.
    Explorer,
    /// Application settings panel.
    Settings,
    /// Editor container – hosts its own pane tree (SourceCode, Wysiwyg,
    /// Preview, Outline).
    Editor,
}

impl WindowPanelKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Settings => "Settings",
            Self::Editor => "Editor",
        }
    }

    /// Window-level panel types (displayed in top-level dropdown).
    pub fn all() -> &'static [WindowPanelKind] {
        &[Self::Editor, Self::Explorer, Self::Settings]
    }
}

/// The two outer states an Editor panel can be in.
///
/// Derived from whether the panel's tab list is empty; switching happens
/// automatically when the first tab is created or the last one is closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPanelMode {
    /// No document tabs: the panel shows the welcome prompt (double-click
    /// to start editing) instead of any pane.
    Welcome,
    /// At least one document tab: the panel renders its pane tree
    /// (Wysiwyg / Source Code / Preview / Outline).
    Editing,
}

impl EditorPanelMode {
    /// True when the panel's session holds tabs.
    pub fn is_editing(self) -> bool {
        matches!(self, Self::Editing)
    }
}

/// Window-level panel containers use the engine's default drag policy:
/// plain drags split with a content seed, Shift drags open the dragged
/// panel in a new window, Ctrl swaps, Alt does nothing.
impl DragPolicy<WindowPanelKind> for SplitterContainer<WindowPanelKind> {}

/// The window-level split root: the outer panel layout.
pub type WindowLayout = SplitterRoot<WindowPanelKind>;

/// The id of the root panel created by the default layout.
pub const ROOT_PANEL_ID: NodeId = 1;

/// The Editor panel id of the default layout: the initial split is
/// Explorer (left) + Editor (right), and the split node shares the Editor
/// leaf's id by the tree's split-id convention.
pub const DEFAULT_EDITOR_PANEL_ID: NodeId = 2;

/// The default window layout: Explorer (left, 30%) + Editor (right, 70%).
///
/// A constructor instead of a `Default` impl because the orphan rule
/// forbids implementing a foreign trait (std `Default`) for the foreign
/// `SplitterRoot<WindowPanelKind>` even with the local kind.
pub fn default_layout() -> WindowLayout {
    SplitterRoot {
        tree: splitype_splitter::tree::SplitTree::Split {
            id: DEFAULT_EDITOR_PANEL_ID,
            axis: splitype_splitter::tree::SplitAxis::Horizontal,
            ratio: 0.3,
            first: Box::new(splitype_splitter::tree::SplitTree::Leaf(
                SplitterContainer::new(ROOT_PANEL_ID, WindowPanelKind::Explorer),
            )),
            second: Box::new(splitype_splitter::tree::SplitTree::Leaf(
                SplitterContainer::new(DEFAULT_EDITOR_PANEL_ID, WindowPanelKind::Editor),
            )),
        },
        next_node_id: 3,
        active_splitter_drag: None,
        active_border_menu: None,
        active_leaf: Some(DEFAULT_EDITOR_PANEL_ID),
        activation_history: vec![DEFAULT_EDITOR_PANEL_ID],
        focused_leaf: None,
    }
}

/// Sidebar and tiled-layout state of the window.
pub struct WindowPanels {
    pub(crate) explorer: ExplorerState,
    pub(crate) layout: WindowLayout,
    pub(crate) settings: SettingsUiState,
}

impl Default for WindowPanels {
    fn default() -> Self {
        Self {
            explorer: ExplorerState::default(),
            layout: default_layout(),
            settings: SettingsUiState::default(),
        }
    }
}
