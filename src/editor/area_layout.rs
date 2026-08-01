//! Tiled window layout manager, area splitting, interactive drag resizers, and border context menus.

use gpui::*;

/// Available panel area types in tiled split layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaType {
    Settings,
    Explorer,
    Outline,
    Source,
    Block,
    Wysiwyg,
}

impl AreaType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Explorer => "Explorer",
            Self::Outline => "Outline",
            Self::Source => "Source",
            Self::Block => "Block",
            Self::Wysiwyg => "WYSIWYG",
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Settings => "Application settings & document info",
            Self::Explorer => "Workspace file directory tree",
            Self::Outline => "Document section headings outline",
            Self::Source => "Raw Markdown text editor",
            Self::Block => "Visual block editor (Rendered)",
            Self::Wysiwyg => "WYSIWYG visual editor (Rendered)",
        }
    }

    pub fn all() -> &'static [AreaType] {
        &[
            Self::Block,
            Self::Wysiwyg,
            Self::Source,
            Self::Explorer,
            Self::Outline,
            Self::Settings,
        ]
    }
}

/// Split orientation between adjacent areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, // Splits left and right
    Vertical,   // Splits top and bottom
}

/// Recursive binary layout tree representing tiled areas and splitters.
#[derive(Clone, Debug)]
pub enum LayoutNode {
    Leaf {
        id: usize,
        area_type: AreaType,
    },
    Split {
        id: usize,
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl PartialEq for LayoutNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Leaf { id: id1, area_type: t1 },
                Self::Leaf { id: id2, area_type: t2 },
            ) => id1 == id2 && t1 == t2,
            (
                Self::Split { id: id1, direction: d1, ratio: r1, first: f1, second: s1 },
                Self::Split { id: id2, direction: d2, ratio: r2, first: f2, second: s2 },
            ) => id1 == id2 && d1 == d2 && (r1 - r2).abs() < 1e-4 && f1 == f2 && s1 == s2,
            _ => false,
        }
    }
}

impl LayoutNode {
    pub fn count_leaves(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { first, second, .. } => first.count_leaves() + second.count_leaves(),
        }
    }

    pub fn find_leaf_area(&self, leaf_id: usize) -> Option<AreaType> {
        match self {
            Self::Leaf { id, area_type } => {
                if *id == leaf_id {
                    Some(*area_type)
                } else {
                    None
                }
            }
            Self::Split { first, second, .. } => {
                first.find_leaf_area(leaf_id).or_else(|| second.find_leaf_area(leaf_id))
            }
        }
    }

    pub fn set_leaf_area(&mut self, leaf_id: usize, new_type: AreaType) -> bool {
        match self {
            Self::Leaf { id, area_type } => {
                if *id == leaf_id {
                    *area_type = new_type;
                    true
                } else {
                    false
                }
            }
            Self::Split { first, second, .. } => {
                first.set_leaf_area(leaf_id, new_type) || second.set_leaf_area(leaf_id, new_type)
            }
        }
    }

    pub fn split_leaf(&mut self, target_id: usize, new_id: usize, direction: SplitDirection) -> bool {
        match self {
            Self::Leaf { id, area_type } => {
                if *id == target_id {
                    let old_type = *area_type;
                    let next_type = match old_type {
                        AreaType::Block => AreaType::Source,
                        AreaType::Wysiwyg => AreaType::Source,
                        AreaType::Source => AreaType::Block,
                        AreaType::Explorer => AreaType::Block,
                        AreaType::Outline => AreaType::Block,
                        AreaType::Settings => AreaType::Block,
                    };
                    *self = Self::Split {
                        id: new_id,
                        direction,
                        ratio: 0.5,
                        first: Box::new(Self::Leaf {
                            id: *id,
                            area_type: old_type,
                        }),
                        second: Box::new(Self::Leaf {
                            id: new_id,
                            area_type: next_type,
                        }),
                    };
                    true
                } else {
                    false
                }
            }
            Self::Split { first, second, .. } => {
                first.split_leaf(target_id, new_id, direction)
                    || second.split_leaf(target_id, new_id, direction)
            }
        }
    }

    pub fn remove_leaf(&mut self, target_id: usize) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split { first, second, .. } => {
                if let Self::Leaf { id, .. } = **first {
                    if id == target_id {
                        *self = (**second).clone();
                        return true;
                    }
                }
                if let Self::Leaf { id, .. } = **second {
                    if id == target_id {
                        *self = (**first).clone();
                        return true;
                    }
                }
                first.remove_leaf(target_id) || second.remove_leaf(target_id)
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_split_ratio(&mut self, split_id: usize, new_ratio: f32) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split { id, ratio, first, second, .. } => {
                if *id == split_id {
                    *ratio = new_ratio.clamp(0.08, 0.92);
                    true
                } else {
                    first.set_split_ratio(split_id, new_ratio)
                        || second.set_split_ratio(split_id, new_ratio)
                }
            }
        }
    }

    pub fn swap_sibling_leaves(&mut self, split_id: usize) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split { id, first, second, .. } => {
                if *id == split_id {
                    std::mem::swap(first, second);
                    true
                } else {
                    first.swap_sibling_leaves(split_id) || second.swap_sibling_leaves(split_id)
                }
            }
        }
    }
}

/// Presets for workspace layout arrangements.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutPreset {
    DefaultEditor,
    DualView,
    WorkspaceEditor,
    TripleView,
    OutlineView,
}

impl LayoutPreset {
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            Self::DefaultEditor => "Single Editor",
            Self::DualView => "Dual (Block | Source)",
            Self::WorkspaceEditor => "Explorer (Explorer | Block)",
            Self::TripleView => "Triple (Explorer | Block | Source)",
            Self::OutlineView => "Outline (Outline | Block | Explorer)",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> &'static [LayoutPreset] {
        &[
            Self::DefaultEditor,
            Self::DualView,
            Self::WorkspaceEditor,
            Self::TripleView,
            Self::OutlineView,
        ]
    }
}

/// Active drag session for resizing a split bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterDragSession {
    pub split_id: usize,
    pub direction: SplitDirection,
    pub start_pointer_pos: f32,
    pub start_ratio: f32,
    pub total_span: f32,
}

/// Active corner gesture session (corner drag to split area).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerDragSession {
    pub leaf_id: usize,
    pub start_pos: Point<Pixels>,
}

/// Context menu state for right-clicking a border divider bar between areas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderMenuState {
    pub split_id: usize,
    pub direction: SplitDirection,
    pub position: Point<Pixels>,
}

/// Full state for the tiled area layout manager.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaLayoutState {
    pub root: LayoutNode,
    pub next_id: usize,
    pub active_dropdown_leaf: Option<usize>,
    pub maximized_leaf: Option<usize>,
    pub active_splitter_drag: Option<SplitterDragSession>,
    pub active_corner_drag: Option<CornerDragSession>,
    pub active_border_menu: Option<BorderMenuState>,
}

impl Default for AreaLayoutState {
    fn default() -> Self {
        Self {
            root: LayoutNode::Leaf {
                id: 1,
                area_type: AreaType::Block,
            },
            next_id: 2,
            active_dropdown_leaf: None,
            maximized_leaf: None,
            active_splitter_drag: None,
            active_corner_drag: None,
            active_border_menu: None,
        }
    }
}

impl AreaLayoutState {
    #[allow(dead_code)]
    pub fn apply_preset(&mut self, preset: LayoutPreset) {
        self.maximized_leaf = None;
        self.active_dropdown_leaf = None;
        self.active_border_menu = None;
        match preset {
            LayoutPreset::DefaultEditor => {
                self.root = LayoutNode::Leaf {
                    id: 1,
                    area_type: AreaType::Block,
                };
                self.next_id = 2;
            }
            LayoutPreset::DualView => {
                self.root = LayoutNode::Split {
                    id: 1,
                    direction: SplitDirection::Horizontal,
                    ratio: 0.5,
                    first: Box::new(LayoutNode::Leaf {
                        id: 2,
                        area_type: AreaType::Block,
                    }),
                    second: Box::new(LayoutNode::Leaf {
                        id: 3,
                        area_type: AreaType::Source,
                    }),
                };
                self.next_id = 4;
            }
            LayoutPreset::WorkspaceEditor => {
                self.root = LayoutNode::Split {
                    id: 1,
                    direction: SplitDirection::Horizontal,
                    ratio: 0.22,
                    first: Box::new(LayoutNode::Leaf {
                        id: 2,
                        area_type: AreaType::Explorer,
                    }),
                    second: Box::new(LayoutNode::Leaf {
                        id: 3,
                        area_type: AreaType::Block,
                    }),
                };
                self.next_id = 4;
            }
            LayoutPreset::TripleView => {
                self.root = LayoutNode::Split {
                    id: 1,
                    direction: SplitDirection::Horizontal,
                    ratio: 0.2,
                    first: Box::new(LayoutNode::Leaf {
                        id: 2,
                        area_type: AreaType::Explorer,
                    }),
                    second: Box::new(LayoutNode::Split {
                        id: 3,
                        direction: SplitDirection::Horizontal,
                        ratio: 0.5,
                        first: Box::new(LayoutNode::Leaf {
                            id: 4,
                            area_type: AreaType::Block,
                        }),
                        second: Box::new(LayoutNode::Leaf {
                            id: 5,
                            area_type: AreaType::Source,
                        }),
                    }),
                };
                self.next_id = 6;
            }
            LayoutPreset::OutlineView => {
                self.root = LayoutNode::Split {
                    id: 1,
                    direction: SplitDirection::Horizontal,
                    ratio: 0.2,
                    first: Box::new(LayoutNode::Leaf {
                        id: 2,
                        area_type: AreaType::Outline,
                    }),
                    second: Box::new(LayoutNode::Split {
                        id: 3,
                        direction: SplitDirection::Horizontal,
                        ratio: 0.7,
                        first: Box::new(LayoutNode::Leaf {
                            id: 4,
                            area_type: AreaType::Block,
                        }),
                        second: Box::new(LayoutNode::Leaf {
                            id: 5,
                            area_type: AreaType::Explorer,
                        }),
                    }),
                };
                self.next_id = 6;
            }
        }
    }

    pub fn split_area(&mut self, target_leaf_id: usize, direction: SplitDirection) {
        let new_id = self.next_id;
        self.next_id += 1;
        self.root.split_leaf(target_leaf_id, new_id, direction);
        self.active_dropdown_leaf = None;
        self.active_border_menu = None;
    }

    pub fn close_area(&mut self, target_leaf_id: usize) {
        if self.root.count_leaves() > 1 {
            self.root.remove_leaf(target_leaf_id);
            if self.maximized_leaf == Some(target_leaf_id) {
                self.maximized_leaf = None;
            }
        }
        self.active_dropdown_leaf = None;
        self.active_border_menu = None;
    }

    pub fn change_area_type(&mut self, leaf_id: usize, new_type: AreaType) {
        self.root.set_leaf_area(leaf_id, new_type);
        self.active_dropdown_leaf = None;
    }

    pub fn toggle_maximize(&mut self, leaf_id: usize) {
        if self.maximized_leaf == Some(leaf_id) {
            self.maximized_leaf = None;
        } else {
            self.maximized_leaf = Some(leaf_id);
        }
    }

    pub fn toggle_dropdown(&mut self, leaf_id: usize) {
        if self.active_dropdown_leaf == Some(leaf_id) {
            self.active_dropdown_leaf = None;
        } else {
            self.active_dropdown_leaf = Some(leaf_id);
        }
    }

    pub fn update_splitter_drag(&mut self, current_pointer_pos: f32) {
        if let Some(session) = self.active_splitter_drag {
            if session.total_span > 1.0 {
                let delta = current_pointer_pos - session.start_pointer_pos;
                let ratio_delta = delta / session.total_span;
                let new_ratio = session.start_ratio + ratio_delta;
                self.root.set_split_ratio(session.split_id, new_ratio);
            }
        }
    }

    pub fn end_splitter_drag(&mut self) {
        self.active_splitter_drag = None;
    }

    pub fn swap_split_children(&mut self, split_id: usize) {
        self.root.swap_sibling_leaves(split_id);
        self.active_border_menu = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_layout_suite() {
        let mut layout = AreaLayoutState::default();
        assert_eq!(layout.root.count_leaves(), 1);

        layout.split_area(1, SplitDirection::Horizontal);
        assert_eq!(layout.root.count_leaves(), 2);

        layout.split_area(2, SplitDirection::Vertical);
        assert_eq!(layout.root.count_leaves(), 3);

        layout.close_area(2);
        assert_eq!(layout.root.count_leaves(), 2);

        layout.change_area_type(1, AreaType::Source);
        assert_eq!(layout.root.find_leaf_area(1), Some(AreaType::Source));

        layout.toggle_maximize(1);
        assert_eq!(layout.maximized_leaf, Some(1));
        layout.toggle_maximize(1);
        assert_eq!(layout.maximized_leaf, None);

        layout.active_splitter_drag = Some(SplitterDragSession {
            split_id: 1,
            direction: SplitDirection::Horizontal,
            start_pointer_pos: 100.0,
            start_ratio: 0.5,
            total_span: 1000.0,
        });
        layout.update_splitter_drag(200.0);
        layout.end_splitter_drag();
        assert_eq!(layout.active_splitter_drag, None);
    }
}
