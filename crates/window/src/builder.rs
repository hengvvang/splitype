//! Window builder for configuring and spawning window instances.

use gpui::{Bounds, Pixels, Point, px, size};
use splitter::container::SplitterContainer;
use splitter::root::SplitterRoot;
use splitter::tree::{SplitAxis, SplitTree};
use crate::layout::{PanelId, WindowLayout};
use crate::panel::PanelKind;

/// Fluent builder for constructing and configuring window layouts.
pub struct WindowBuilder {
    title: String,
    bounds: Option<Bounds<Pixels>>,
    layout: Option<WindowLayout>,
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            title: "Splitype".to_string(),
            bounds: None,
            layout: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn with_dimensions(mut self, width: f32, height: f32) -> Self {
        self.bounds = Some(Bounds {
            origin: Point::default(),
            size: size(px(width), px(height)),
        });
        self
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
                first: Box::new(SplitTree::Leaf(SplitterContainer::new(left_id.0, left_kind))),
                second: Box::new(SplitTree::Leaf(SplitterContainer::new(right_id.0, right_kind))),
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

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn bounds(&self) -> Option<Bounds<Pixels>> {
        self.bounds
    }

    pub fn take_layout(&mut self) -> Option<WindowLayout> {
        self.layout.take()
    }
}
