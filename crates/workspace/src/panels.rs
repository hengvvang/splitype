//! Window panel identity and kind vocabulary.
//!
//! - [`WindowPanelKind`] — the top-level panel types of the tiled split
//!   layout (Explorer / Settings / Editor). The editor's inner panes use
//!   their own kind (`editor::EditorPaneKind`) on the same split engine.
//! - [`WindowLayout`] / [`default_layout`] — the window-level split root.
//!
//! The per-panel *state* aggregates (explorer state, settings state) live
//! in their own crates; the shell owns the instances.

use splitter::container::SplitterContainer;
use splitter::root::SplitterRoot;
use splitter::tree::NodeId;

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
    /// Preview).
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

    pub fn to_kind_id(&self) -> crate::plugin::PanelKindId {
        match self {
            Self::Editor => crate::plugin::PanelKindId::EDITOR,
            Self::Explorer => crate::plugin::PanelKindId::EXPLORER,
            Self::Settings => crate::plugin::PanelKindId::SETTINGS,
        }
    }
}

impl From<WindowPanelKind> for crate::plugin::PanelKindId {
    fn from(kind: WindowPanelKind) -> Self {
        kind.to_kind_id()
    }
}

impl TryFrom<crate::plugin::PanelKindId> for WindowPanelKind {
    type Error = ();
    fn try_from(kind: crate::plugin::PanelKindId) -> Result<Self, Self::Error> {
        match kind {
            crate::plugin::PanelKindId::EDITOR => Ok(WindowPanelKind::Editor),
            crate::plugin::PanelKindId::EXPLORER => Ok(WindowPanelKind::Explorer),
            crate::plugin::PanelKindId::SETTINGS => Ok(WindowPanelKind::Settings),
            _ => Err(()),
        }
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
    /// (Wysiwyg / Source Code / Preview).
    Editing,
}

impl EditorPanelMode {
    /// True when the panel's session holds tabs.
    pub fn is_editing(self) -> bool {
        matches!(self, Self::Editing)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Welcome => "Welcome",
            Self::Editing => "Editing",
        }
    }
}

/// The strongly-typed identifier representing a top-level window panel (Explorer, Settings, Editor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PanelId(pub NodeId);

impl From<NodeId> for PanelId {
    #[inline]
    fn from(id: NodeId) -> Self {
        Self(id)
    }
}

impl From<PanelId> for NodeId {
    #[inline]
    fn from(id: PanelId) -> Self {
        id.0
    }
}

impl From<PanelId> for gpui::ElementId {
    #[inline]
    fn from(id: PanelId) -> Self {
        id.0.into()
    }
}

impl std::fmt::Display for PanelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
        tree: splitter::tree::SplitTree::Split {
            id: DEFAULT_EDITOR_PANEL_ID,
            axis: splitter::tree::SplitAxis::Horizontal,
            ratio: 0.3,
            first: Box::new(splitter::tree::SplitTree::Leaf(
                SplitterContainer::new(ROOT_PANEL_ID, WindowPanelKind::Explorer),
            )),
            second: Box::new(splitter::tree::SplitTree::Leaf(
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
