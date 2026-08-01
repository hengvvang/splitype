//! Tiled window layout manager, area splitting, interactive drag resizers,
//! corner-drag gestures (split / join / swap / duplicate), and border context menus.
//!
//! Design is inspired by Blender's screen area action-zone system:
//! each area exposes four corner hot-zones that, when dragged, produce either a
//! split (same area), a join (neighbour area), a swap (Ctrl), or a duplicate
//! (Shift) – with differentiated gesture thresholds and directional cursors.

use gpui::*;

// ---------------------------------------------------------------------------
// Area type
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Split direction & screen direction
// ---------------------------------------------------------------------------

/// Split orientation between adjacent areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, // Splits left and right
    Vertical,   // Splits top and bottom
}

/// Cardinal direction used for corner-drag gesture routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenDirection {
    North, // Up
    South, // Down
    East,  // Right
    West,  // Left
}

impl ScreenDirection {
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::North | Self::South)
    }
}

/// Modifier key held during a corner drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerDragModifier {
    None,
    Swap,      // Ctrl  – swap area contents
    Duplicate, // Shift – duplicate area into a new window
}

/// Live preview state during a corner drag gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerDragPreview {
    /// Still near the corner, not enough movement yet.
    Dragging,
    /// Showing a split preview line at the given ratio.
    SplitPreview {
        direction: SplitDirection,
        ratio: f32,
    },
    /// Showing a join target highlight.
    JoinPreview {
        target_leaf_id: usize,
        direction: ScreenDirection,
    },
}

// ---------------------------------------------------------------------------
// Layout tree node
// ---------------------------------------------------------------------------

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
                Self::Leaf {
                    id: id1,
                    area_type: t1,
                },
                Self::Leaf {
                    id: id2,
                    area_type: t2,
                },
            ) => id1 == id2 && t1 == t2,
            (
                Self::Split {
                    id: id1,
                    direction: d1,
                    ratio: r1,
                    first: f1,
                    second: s1,
                },
                Self::Split {
                    id: id2,
                    direction: d2,
                    ratio: r2,
                    first: f2,
                    second: s2,
                },
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
            Self::Split { first, second, .. } => first
                .find_leaf_area(leaf_id)
                .or_else(|| second.find_leaf_area(leaf_id)),
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

    /// Does the subtree contain the leaf with `leaf_id`?
    pub fn contains_leaf(&self, leaf_id: usize) -> bool {
        match self {
            Self::Leaf { id, .. } => *id == leaf_id,
            Self::Split { first, second, .. } => {
                first.contains_leaf(leaf_id) || second.contains_leaf(leaf_id)
            }
        }
    }



    /// Collect all leaf rectangles. Each entry: (leaf_id, x, y, width, height).
    /// Coordinates are in layout-space (normalized 0..1).
    pub fn collect_leaf_rects(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        out: &mut Vec<(usize, f32, f32, f32, f32)>,
    ) {
        match self {
            Self::Leaf { id, .. } => {
                out.push((*id, x, y, w, h));
            }
            Self::Split {
                direction,
                ratio,
                first,
                second,
                ..
            } => {
                let r = ratio.clamp(0.0, 1.0);
                match direction {
                    SplitDirection::Horizontal => {
                        first.collect_leaf_rects(x, y, w * r, h, out);
                        second.collect_leaf_rects(x + w * r, y, w * (1.0 - r), h, out);
                    }
                    SplitDirection::Vertical => {
                        first.collect_leaf_rects(x, y, w, h * r, out);
                        second.collect_leaf_rects(x, y + h * r, w, h * (1.0 - r), out);
                    }
                }
            }
        }
    }

    /// Find layout-space span (0..1 width or height) for a target split node.
    pub fn find_split_span(
        &self,
        target_split_id: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Option<(SplitDirection, f32)> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                if *id == target_split_id {
                    let span = match direction {
                        SplitDirection::Horizontal => w,
                        SplitDirection::Vertical => h,
                    };
                    return Some((*direction, span));
                }
                let r = ratio.clamp(0.0, 1.0);
                match direction {
                    SplitDirection::Horizontal => first
                        .find_split_span(target_split_id, x, y, w * r, h)
                        .or_else(|| {
                            second.find_split_span(target_split_id, x + w * r, y, w * (1.0 - r), h)
                        }),
                    SplitDirection::Vertical => first
                        .find_split_span(target_split_id, x, y, w, h * r)
                        .or_else(|| {
                            second.find_split_span(target_split_id, x, y + h * r, w, h * (1.0 - r))
                        }),
                }
            }
        }
    }

    pub fn split_leaf(
        &mut self,
        target_id: usize,
        new_id: usize,
        direction: SplitDirection,
    ) -> bool {
        self.split_leaf_with_ratio(target_id, new_id, direction, 0.5)
    }

    /// Split a leaf at a specific ratio (clamped to [0.15, 0.85]).
    pub fn split_leaf_with_ratio(
        &mut self,
        target_id: usize,
        new_id: usize,
        direction: SplitDirection,
        ratio: f32,
    ) -> bool {
        let ratio = ratio.clamp(0.15, 0.85);
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
                        ratio,
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
                first.split_leaf_with_ratio(target_id, new_id, direction, ratio)
                    || second.split_leaf_with_ratio(target_id, new_id, direction, ratio)
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

    /// Join `target_id` into `into_id`. The `target_id` leaf is removed and
    /// `into_id` expands to fill the space. Both leaves must share an immediate
    /// split parent (be adjacent siblings).  Returns true on success.
    pub fn join_leaf(&mut self, into_id: usize, target_id: usize) -> bool {
        if into_id == target_id {
            return false;
        }
        match self {
            Self::Leaf { .. } => false,
            Self::Split { first, second, .. } => {
                let into_in_first = first.contains_leaf(into_id);
                let target_in_first = first.contains_leaf(target_id);

                if into_in_first && target_in_first {
                    first.join_leaf(into_id, target_id)
                } else if !into_in_first && !target_in_first {
                    second.join_leaf(into_id, target_id)
                } else {
                    // The two leaves are in different children → this split is their
                    // lowest common ancestor.  Remove the target leaf from its side;
                    // the remaining child (with target removed) stays in place so the
                    // split direction and ratio are preserved.
                    if target_in_first {
                        if !first.remove_leaf(target_id) {
                            return false;
                        }
                    } else {
                        if !second.remove_leaf(target_id) {
                            return false;
                        }
                    }
                    true
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_split_ratio(&mut self, split_id: usize, new_ratio: f32) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split {
                id,
                ratio,
                first,
                second,
                ..
            } => {
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
            Self::Split {
                id, first, second, ..
            } => {
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

// ---------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Drag / menu sessions
// ---------------------------------------------------------------------------

/// Active drag session for resizing a split bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterDragSession {
    pub split_id: usize,
    pub direction: SplitDirection,
    pub start_pointer_pos: f32,
    pub start_ratio: f32,
    pub total_span: f32,
}

/// Corner-drag gesture session.
///
/// Analogous to Blender's `sActionzoneData` – tracks which area corner was
/// grabbed, the gesture direction, and the modifier key held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerDragSession {
    /// The leaf whose corner was grabbed.
    pub leaf_id: usize,
    /// Where the drag started (in window coords).
    pub start_pos: Point<Pixels>,
    /// Cardinal direction deduced from the mouse delta so far.
    pub gesture_dir: Option<ScreenDirection>,
    /// Modifier key held during the drag.
    pub modifier: CornerDragModifier,
    /// Live preview state for the corner drag overlay.
    pub preview: CornerDragPreview,
}

/// Context menu state for right-clicking a border divider bar between areas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderMenuState {
    pub split_id: usize,
    pub direction: SplitDirection,
    pub position: Point<Pixels>,
}

// ---------------------------------------------------------------------------
// Corner drag thresholds (in pixels)
// ---------------------------------------------------------------------------

/// Minimum drag distance before swap / duplicate gesture.
pub const MODIFIER_THRESHOLD_PX: f32 = 4.0;

// ---------------------------------------------------------------------------
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Preferences UI tab & state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreferencesTab {
    Interface, // Interface, Theme & Status Bar
    Editing,   // Editing, Typography & Startup
    Keymap,    // Keymap / Keyboard Shortcuts
}

impl PreferencesTab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::Editing => "Editing",
            Self::Keymap => "Keymap",
        }
    }

    pub fn all() -> &'static [PreferencesTab] {
        &[
            Self::Interface,
            Self::Editing,
            Self::Keymap,
        ]
    }
}

// ---------------------------------------------------------------------------
// Area layout state
// ---------------------------------------------------------------------------

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
    /// Measured pixel size of the tiled-layout container.
    pub container_size: Option<Size<Pixels>>,
    // --- Settings / Preferences Panel State ---
    pub preferences_tab: PreferencesTab,
    pub preferences_expanded_sections: HashSet<String>,
    pub preferences_expanded_cards: HashSet<String>,
    pub pref_show_status_bar: bool,
    pub pref_show_word_count: bool,
    pub pref_show_cursor_pos: bool,
    pub pref_show_mode_switch: bool,
    pub pref_show_table_headers: bool,
}

impl Default for AreaLayoutState {
    fn default() -> Self {
        let mut sections = HashSet::new();
        sections.insert("theme".to_string());
        sections.insert("status_bar".to_string());
        sections.insert("typography".to_string());
        sections.insert("markdown".to_string());
        sections.insert("startup".to_string());
        sections.insert("doc_actions".to_string());
        sections.insert("view_controls".to_string());

        let mut cards = HashSet::new();
        cards.insert("status_bar".to_string());
        cards.insert("markdown_options".to_string());

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
            container_size: None,
            preferences_tab: PreferencesTab::Interface,
            preferences_expanded_sections: sections,
            preferences_expanded_cards: cards,
            pref_show_status_bar: true,
            pref_show_word_count: true,
            pref_show_cursor_pos: true,
            pref_show_mode_switch: true,
            pref_show_table_headers: true,
        }
    }
}

impl AreaLayoutState {
    // ------------------------------------------------------------------
    // Preset
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // Split / close / type
    // ------------------------------------------------------------------

    pub fn split_area(&mut self, target_leaf_id: usize, direction: SplitDirection) {
        let new_id = self.next_id;
        self.next_id += 1;
        self.root.split_leaf(target_leaf_id, new_id, direction);
        self.active_dropdown_leaf = None;
        self.active_border_menu = None;
    }

    /// Split a leaf area at a specific ratio.
    pub fn split_area_with_ratio(
        &mut self,
        target_leaf_id: usize,
        direction: SplitDirection,
        ratio: f32,
    ) {
        let new_id = self.next_id;
        self.next_id += 1;
        self.root
            .split_leaf_with_ratio(target_leaf_id, new_id, direction, ratio);
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

    pub fn toggle_preferences_section(&mut self, section_key: &str) {
        if self.preferences_expanded_sections.contains(section_key) {
            self.preferences_expanded_sections.remove(section_key);
        } else {
            self.preferences_expanded_sections
                .insert(section_key.to_string());
        }
    }

    pub fn toggle_preferences_card(&mut self, card_key: &str) {
        if self.preferences_expanded_cards.contains(card_key) {
            self.preferences_expanded_cards.remove(card_key);
        } else {
            self.preferences_expanded_cards
                .insert(card_key.to_string());
        }
    }

    // ------------------------------------------------------------------
    // Join two adjacent areas
    // ------------------------------------------------------------------

    /// Join `target_id` into `into_id`. The target leaf is removed and its
    /// space is absorbed by the `into_id` leaf. The two must be adjacent
    /// (share an edge) in the layout.
    pub fn join_area(&mut self, into_id: usize, target_id: usize) -> bool {
        if into_id == target_id {
            return false;
        }
        if self.root.count_leaves() <= 1 {
            return false;
        }
        let ok = self.root.join_leaf(into_id, target_id);
        if ok {
            if self.maximized_leaf == Some(target_id) {
                self.maximized_leaf = None;
            }
        }
        self.active_dropdown_leaf = None;
        self.active_border_menu = None;
        ok
    }

    // ------------------------------------------------------------------
    // Swap two area types
    // ------------------------------------------------------------------

    /// Swap the area type of leaf `a` and leaf `b`.
    pub fn swap_area_types(&mut self, a: usize, b: usize) {
        let type_a = self.root.find_leaf_area(a);
        let type_b = self.root.find_leaf_area(b);
        if let (Some(ta), Some(tb)) = (type_a, type_b) {
            self.root.set_leaf_area(a, tb);
            self.root.set_leaf_area(b, ta);
        }
    }

    // ------------------------------------------------------------------
    // Maximise / dropdown
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // Splitter drag
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // Corner drag — split / join / swap / duplicate
    // ------------------------------------------------------------------

    /// Begin a corner-drag gesture from `leaf_id` at `pos` with optional
    /// modifier key.
    pub fn start_corner_drag(
        &mut self,
        leaf_id: usize,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        self.active_corner_drag = Some(CornerDragSession {
            leaf_id,
            start_pos: pos,
            gesture_dir: None,
            modifier,
            preview: CornerDragPreview::Dragging,
        });
    }

    /// Process a mouse-move event during a corner drag.
    ///
    /// `current_pos`   – current mouse position in window coords.
    /// `container_size` – size of the tiled-layout container (used for
    ///                    normalised hit-test rects and split-ratio calc).
    ///
    /// Updates the active session's preview state. Modifier-based actions
    /// (Ctrl / Shift) still fire immediately when they cross their threshold
    /// by returning `Some(action)`; the no-modifier split/join path only
    /// updates the preview and always returns `None` so the caller can paint
    /// the live overlay.
    pub fn update_corner_drag(
        &mut self,
        current_pos: Point<Pixels>,
        container_size: Size<Pixels>,
    ) -> Option<CornerDragAction> {
        let session = match self.active_corner_drag {
            Some(ref s) => *s,
            None => return None,
        };

        let dx = f32::from(current_pos.x - session.start_pos.x);
        let dy = f32::from(current_pos.y - session.start_pos.y);
        let dist = (dx * dx + dy * dy).sqrt();
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        // Determine cardinal direction.
        let dir = if abs_dy > abs_dx {
            if dy > 0.0 {
                ScreenDirection::South
            } else {
                ScreenDirection::North
            }
        } else {
            if dx > 0.0 {
                ScreenDirection::East
            } else {
                ScreenDirection::West
            }
        };

        // Update gesture direction for cursor feedback.
        self.active_corner_drag.as_mut().unwrap().gesture_dir = Some(dir);

        // --- Modifier-based actions (Ctrl / Shift) ---
        // These remain immediate: once the threshold is crossed the action
        // is returned so the caller can execute it straight away.
        if session.modifier != CornerDragModifier::None {
            if dist < MODIFIER_THRESHOLD_PX {
                return None;
            }
            let leaf_rects = self.collect_leaf_rects(container_size);
            let over_id = leaf_id_from_point(&leaf_rects, current_pos);

            return Some(match session.modifier {
                CornerDragModifier::Swap => {
                    if let Some(target) = over_id {
                        if target != session.leaf_id {
                            CornerDragAction::Swap {
                                from: session.leaf_id,
                                to: target,
                            }
                        } else {
                            CornerDragAction::Cancel
                        }
                    } else {
                        CornerDragAction::Cancel
                    }
                }
                CornerDragModifier::Duplicate => CornerDragAction::Duplicate {
                    leaf_id: session.leaf_id,
                },
                CornerDragModifier::None => unreachable!(),
            });
        }

        // --- No modifier: split vs join preview ---
        let leaf_rects = self.collect_leaf_rects(container_size);
        let over_id = leaf_id_from_point(&leaf_rects, current_pos);

        if over_id == Some(session.leaf_id) || over_id.is_none() {
            // Cursor is still in the same area (or outside).  Potential split.
            let split_dir = if dir.is_vertical() {
                SplitDirection::Vertical
            } else {
                SplitDirection::Horizontal
            };
            // Calculate split ratio from cursor position within the leaf.
            if let Some((_id, lx, ly, lw, lh)) =
                self.get_leaf_pixel_rect(session.leaf_id, &leaf_rects)
            {
                if lw > 1.0 && lh > 1.0 {
                    let ratio = match split_dir {
                        SplitDirection::Horizontal => {
                            let r = (f32::from(current_pos.x) - lx) / lw;
                            r.clamp(0.15, 0.85)
                        }
                        SplitDirection::Vertical => {
                            let r = (f32::from(current_pos.y) - ly) / lh;
                            r.clamp(0.15, 0.85)
                        }
                    };
                    self.active_corner_drag.as_mut().unwrap().preview =
                        CornerDragPreview::SplitPreview {
                            direction: split_dir,
                            ratio,
                        };
                }
            }
        } else if let Some(target_id) = over_id {
            // Cursor is over a different area.  Potential join.
            self.active_corner_drag.as_mut().unwrap().preview = CornerDragPreview::JoinPreview {
                target_leaf_id: target_id,
                direction: dir,
            };
        }

        None
    }

    /// Finish the corner-drag gesture on mouse release.
    ///
    /// Reads the active session's preview state and returns the appropriate
    /// action, then clears the session.
    pub fn finish_corner_drag(&mut self) -> Option<CornerDragAction> {
        let session = self.active_corner_drag?;
        let action = match session.preview {
            CornerDragPreview::SplitPreview { direction, ratio } => Some(CornerDragAction::Split {
                leaf_id: session.leaf_id,
                direction,
                ratio,
            }),
            CornerDragPreview::JoinPreview {
                target_leaf_id,
                direction: _,
            } => Some(CornerDragAction::Join {
                from: session.leaf_id,
                into: target_leaf_id,
            }),
            CornerDragPreview::Dragging => Some(CornerDragAction::Cancel),
        };
        self.end_corner_drag();
        action
    }

    /// End the corner-drag session, clearing state.
    pub fn end_corner_drag(&mut self) {
        self.active_corner_drag = None;
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Collect all leaf rectangles in pixel coordinates.
    pub fn collect_leaf_rects(
        &self,
        container_size: Size<Pixels>,
    ) -> Vec<(usize, f32, f32, f32, f32)> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let mut rects = Vec::new();
        if w > 0.0 && h > 0.0 {
            // Use normalised layout coords, then scale to pixels.
            let mut norm = Vec::new();
            self.root.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut norm);
            for (id, nx, ny, nw, nh) in norm {
                rects.push((id, nx * w, ny * h, nw * w, nh * h));
            }
        }
        rects
    }

    /// Get the pixel-space rectangle for a specific leaf, given pre-computed
    /// leaf rects from `collect_leaf_rects`.
    pub fn get_leaf_pixel_rect(
        &self,
        leaf_id: usize,
        rects: &[(usize, f32, f32, f32, f32)],
    ) -> Option<(usize, f32, f32, f32, f32)> {
        rects.iter().find(|&&(id, ..)| id == leaf_id).copied()
    }

    /// Calculate the pixel span (width or height) of a split container.
    pub fn get_split_pixel_span(
        &self,
        split_id: usize,
        container_size: Size<Pixels>,
    ) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        if w > 0.0 && h > 0.0 {
            if let Some((dir, span_norm)) =
                self.root.find_split_span(split_id, 0.0, 0.0, 1.0, 1.0)
            {
                let pixel_span = match dir {
                    SplitDirection::Horizontal => span_norm * w,
                    SplitDirection::Vertical => span_norm * h,
                };
                return Some(pixel_span);
            }
        }
        None
    }

    // ------------------------------------------------------------------
    // Border context menu
    // ------------------------------------------------------------------

    pub fn swap_split_children(&mut self, split_id: usize) {
        self.root.swap_sibling_leaves(split_id);
        self.active_border_menu = None;
    }
}

// ---------------------------------------------------------------------------
// Corner-drag action result
// ---------------------------------------------------------------------------

/// The action that should be performed once a corner-drag gesture crosses its
/// threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerDragAction {
    /// Split the dragged leaf into two.
    Split {
        leaf_id: usize,
        direction: SplitDirection,
        ratio: f32,
    },
    /// Join the dragged leaf into a neighbor (remove the dragged leaf).
    Join { from: usize, into: usize },
    /// Swap area types between two leaves.
    Swap { from: usize, to: usize },
    /// Duplicate the dragged leaf (open in a new window).
    Duplicate { leaf_id: usize },
    /// Gesture was cancelled (e.g. dragged to invalid target).
    Cancel,
}

// ---------------------------------------------------------------------------
// Hit-test helper
// ---------------------------------------------------------------------------

/// Return the leaf id that contains `pos`, given pixel-space rects.
fn leaf_id_from_point(rects: &[(usize, f32, f32, f32, f32)], pos: Point<Pixels>) -> Option<usize> {
    let px = f32::from(pos.x);
    let py = f32::from(pos.y);
    for &(id, rx, ry, rw, rh) in rects {
        if px >= rx && px <= rx + rw && py >= ry && py <= ry + rh {
            return Some(id);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    #[test]
    fn test_join_sibling_leaves() {
        let mut layout = AreaLayoutState::default();
        // Create a simple horizontal split: [1, 3]
        layout.split_area(1, SplitDirection::Horizontal);
        // Now we have leaf 1 (Block) and leaf 3 (Source) as siblings.
        assert_eq!(layout.root.count_leaves(), 2);

        // Join leaf 3 into leaf 1: remove 3, expand 1.
        let ok = layout.join_area(1, 3);
        assert!(ok);
        assert_eq!(layout.root.count_leaves(), 1);
        assert_eq!(layout.root.find_leaf_area(1), Some(AreaType::Block));
    }

    #[test]
    fn test_join_nested_leaves() {
        let mut layout = AreaLayoutState::default();
        // Build: Split(H) { Leaf(1, Block), Split(V) { Leaf(3, Source), Leaf(4, Outline) } }
        layout.split_area(1, SplitDirection::Horizontal); // ids: 1, 3
        layout.split_area(3, SplitDirection::Vertical); // ids: 1, 4, 5 (new leaf from 3)
        assert_eq!(layout.root.count_leaves(), 3);

        // Join leaf 1 (Block, left) with leaf 4 (Source, was from first 3 split).
        // After join we should have 2 leaves: 1 (Block, expanded) and 5 (Outline from second).
        let ok = layout.join_area(1, 4);
        assert!(ok);
        assert_eq!(layout.root.count_leaves(), 2);
    }

    #[test]
    fn test_collect_leaf_rects() {
        let mut layout = AreaLayoutState::default();
        layout.split_area(1, SplitDirection::Horizontal);
        let rects = layout.collect_leaf_rects(size(px(1000.0), px(800.0)));
        assert_eq!(rects.len(), 2);
        // First leaf: left half, second leaf: right half.
        let (id1, x1, _y1, w1, h1) = rects[0];
        let (id2, x2, _y2, w2, h2) = rects[1];
        assert!((w1 - 500.0).abs() < 1.0);
        assert!((w2 - 500.0).abs() < 1.0);
        assert!((h1 - 800.0).abs() < 1.0);
        assert!((h2 - 800.0).abs() < 1.0);
        assert!((x2 - 500.0).abs() < 1.0);
    }
}
