//! Versioned persistence projection of a window: the panel layout tree plus
//! per-panel plugin state.
//!
//! Only durable facts are captured — transient interaction sessions (splitter
//! drags, dropdowns) are not serialized. Panels opt into state persistence
//! through [`core_contracts::PanelDescriptor::serialize_state`].

use core_contracts::{PanelId, PanelKind};
use serde::{Deserialize, Serialize};
use splitter::root::SplitterRoot;
use splitter::tree::{NodeId, SplitTree};

/// Current schema version of [`PersistedWindowState`]. Bump on breaking
/// changes; loaders must reject versions they do not understand.
pub const WINDOW_STATE_VERSION: u32 = 1;

/// Persisted state of one window panel tile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedPanel {
    pub id: PanelId,
    pub kind: PanelKind,
    /// Descriptor-owned state JSON; `null` when the plugin did not opt in.
    #[serde(default)]
    pub state: serde_json::Value,
}

/// Versioned snapshot of one window: layout topology and panel states.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedWindowState {
    pub version: u32,
    pub tree: SplitTree<PanelKind>,
    pub next_node_id: NodeId,
    pub active_leaf: Option<NodeId>,
    pub activation_history: Vec<NodeId>,
    pub panels: Vec<PersistedPanel>,
}

impl PersistedWindowState {
    /// Rebuilds the live layout root from the persisted topology.
    pub fn into_layout(self) -> SplitterRoot<PanelKind> {
        SplitterRoot {
            tree: self.tree,
            next_node_id: self.next_node_id,
            active_splitter_drag: None,
            active_border_menu: None,
            active_leaf: self.active_leaf,
            activation_history: self.activation_history,
            focused_leaf: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use splitter::container::SplitterContainer;
    use splitter::tree::SplitAxis;

    fn leaf(id: NodeId, kind: &'static str) -> SplitTree<PanelKind> {
        SplitTree::Leaf(SplitterContainer::new(id, PanelKind::from_static(kind)))
    }

    #[test]
    fn persisted_window_state_round_trips() {
        let state = PersistedWindowState {
            version: WINDOW_STATE_VERSION,
            tree: SplitTree::Split {
                id: 3,
                axis: SplitAxis::Horizontal,
                ratio: 0.3,
                first: Box::new(leaf(1, "splitype.panel.explorer")),
                second: Box::new(leaf(2, "splitype.panel.editor")),
            },
            next_node_id: 4,
            active_leaf: Some(2),
            activation_history: vec![2],
            panels: vec![PersistedPanel {
                id: PanelId(2),
                kind: PanelKind::from_static("splitype.panel.editor"),
                state: serde_json::json!({ "text": "# hello" }),
            }],
        };

        let json = serde_json::to_string(&state).expect("serialize");
        let restored: PersistedWindowState = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.version, WINDOW_STATE_VERSION);
        assert_eq!(restored.next_node_id, 4);
        assert_eq!(restored.activation_history, vec![2]);
        assert_eq!(restored.panels.len(), 1);
        assert_eq!(restored.panels[0].state["text"], "# hello");

        let layout = restored.into_layout();
        assert_eq!(layout.tree.count_leaves(), 2);
        assert_eq!(layout.active_leaf, Some(2));
    }
}
