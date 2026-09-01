//! Window layout factory — fluent constructors for the window-level split
//! layout root.

use crate::layout::WindowLayout;
use platform_contracts::{PanelId, PanelKind};
use splitter::container::SplitterContainer;
use splitter::root::SplitterRoot;
use splitter::tree::{SplitAxis, SplitTree};

/// Fluent builder producing a [`WindowLayout`] from panel kinds and ids.
pub struct WindowLayoutBuilder {
    layout: Option<WindowLayout>,
}

impl Default for WindowLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowLayoutBuilder {
    pub fn new() -> Self {
        Self { layout: None }
    }

    pub fn with_layout(mut self, layout: WindowLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn with_single_panel(mut self, panel_id: PanelId, kind: PanelKind) -> Self {
        self.layout = Some(SplitterRoot::single_leaf(panel_id.0, kind));
        self
    }

    pub fn with_split(
        mut self,
        left_id: PanelId,
        left_kind: PanelKind,
        right_id: PanelId,
        right_kind: PanelKind,
        ratio: f32,
        active_id: PanelId,
    ) -> Self {
        let split_id = right_id.0;
        self.layout = Some(SplitterRoot {
            tree: SplitTree::Split {
                id: split_id,
                axis: SplitAxis::Horizontal,
                ratio,
                first: Box::new(SplitTree::Leaf(SplitterContainer::new(
                    left_id.0, left_kind,
                ))),
                second: Box::new(SplitTree::Leaf(SplitterContainer::new(
                    right_id.0, right_kind,
                ))),
            },
            next_node_id: split_id + 1,
            active_splitter_drag: None,
            active_border_menu: None,
            active_leaf: Some(active_id.0),
            activation_history: vec![active_id.0],
            focused_leaf: None,
        });
        self
    }

    pub fn take_layout(&mut self) -> Option<WindowLayout> {
        self.layout.take()
    }
}
