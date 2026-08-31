//! Window panel identity and layout types.
//!
//! - [`PanelId`] — the strongly-typed identifier of a top-level window panel.
//! - [`WindowLayout`] / [`default_layout`] — the window-level split root.

use splitter::container::SplitterContainer;
use splitter::root::SplitterRoot;
use splitter::tree::NodeId;
use crate::plugin::PanelKindId;

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
pub type WindowLayout = SplitterRoot<PanelKindId>;

/// The id of the root panel created by the default layout.
pub const ROOT_PANEL_ID: NodeId = 1;

/// The Editor panel id of the default layout: the initial split is
/// Explorer (left) + Editor (right), and the split node shares the Editor
/// leaf's id by the tree's split-id convention.
pub const DEFAULT_EDITOR_PANEL_ID: NodeId = 2;

/// The default window layout: Explorer (left, 30%) + Editor (right, 70%).
pub fn default_layout() -> WindowLayout {
    SplitterRoot {
        tree: splitter::tree::SplitTree::Split {
            id: DEFAULT_EDITOR_PANEL_ID,
            axis: splitter::tree::SplitAxis::Horizontal,
            ratio: 0.3,
            first: Box::new(splitter::tree::SplitTree::Leaf(
                SplitterContainer::new(ROOT_PANEL_ID, PanelKindId::EXPLORER),
            )),
            second: Box::new(splitter::tree::SplitTree::Leaf(
                SplitterContainer::new(DEFAULT_EDITOR_PANEL_ID, PanelKindId::EDITOR),
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
