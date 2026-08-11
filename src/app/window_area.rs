//! Window-panel kinds and the window-level split root.
//!
//! The splitter engine stays kind-agnostic: the application defines its
//! own panel kinds here, implements its own taxonomy (which kinds are
//! editors, how each kind seeds a split), registers the default drag
//! policy for its panel type, and seeds the default layout
//! (Explorer + Editor). The editor's inner panes use their own kind
//! (`crate::editor::session::EditorPaneKind`) on the same split engine.

use splitype_splitter::container::SplitterContainer;
use splitype_splitter::policy::DragPolicy;
use splitype_splitter::root::SplitterRoot;
use splitype_splitter::tree::NodeId;

/// Top-level panel types in the tiled split layout: the window-level
/// panels (each a split-tree leaf). The editor's inner panes are a
/// separate kind vocabulary on the same engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowPanelKind {
    /// File explorer / file tree.
    Explorer,
    /// Application settings panel.
    Settings,
    /// Editor container – hosts its own pane tree (Source, Wysiwyg,
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
    /// No document tabs: the area shows the welcome prompt (double-click to
    /// start editing) instead of any panel.
    Welcome,
    /// At least one document tab: the area renders its pane layout
    /// (Wysiwyg / Source Code / Preview / Outline).
    Editing,
}

impl EditorPanelMode {
    /// True when the area's session holds tabs.
    pub fn is_editing(self) -> bool {
        matches!(self, Self::Editing)
    }
}

/// Window-level panel containers use the engine's default drag policy:
/// plain drags split with a content seed, Shift drags open the dragged
/// area in a new window, Ctrl swaps, Alt does nothing.
impl DragPolicy<WindowPanelKind> for SplitterContainer<WindowPanelKind> {}

/// The window-level split root: the outer area layout.
pub type WindowLayout = SplitterRoot<WindowPanelKind>;

/// The id of the root area created by the default layout.
pub const ROOT_PANEL_ID: NodeId = 1;

/// The Editor area id of the default layout: the initial split is
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
            direction: splitype_splitter::tree::Axis::Horizontal,
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
        active_leaf: None,
        activation_history: Vec::new(),
        focused_area: None,
    }
}
