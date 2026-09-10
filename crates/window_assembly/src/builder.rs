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

    pub fn with_single_panel(mut self, panel_id: PanelId, kind: PanelKind) -> Self {
        self.layout = Some(SplitterRoot::single_leaf(panel_id, kind));
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
        let split_id = splitter::SplitId::new(right_id.as_u32() + 1);
        let max_id = left_id
            .as_u32()
            .max(right_id.as_u32())
            .max(split_id.as_u32());
        self.layout = Some(SplitterRoot {
            tree: SplitTree::Split {
                id: split_id,
                axis: SplitAxis::Horizontal,
                ratio,
                first: Box::new(SplitTree::Leaf(SplitterContainer::new(left_id, left_kind))),
                second: Box::new(SplitTree::Leaf(SplitterContainer::new(
                    right_id, right_kind,
                ))),
            },
            allocator: splitter::NodeIdAllocator::with_start(max_id + 1),
            interaction: Default::default(),
            active_leaf: Some(active_id.into()),
            activation_history: vec![active_id.into()],
        });
        self
    }

    pub fn take_layout(&mut self) -> Option<WindowLayout> {
        self.layout.take()
    }
}
