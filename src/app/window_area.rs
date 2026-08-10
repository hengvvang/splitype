//! Window-area kinds and the window-level splitter container.
//!
//! The splitter engine stays generic: it only knows [`ContainerKind`]
//! contracts. The application registers its own area kinds here (by
//! implementing the trait), defines the window layout alias, and seeds the
//! default layout (Explorer + Editor).

use splitype_splitter::state::{ContainerKind, ShiftBehavior, SplitterContainer};
use splitype_splitter::types::NodeId;

/// Top-level area types in the tiled split layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAreaKind {
    /// File explorer / file tree.
    Explorer,
    /// Application settings panel.
    Settings,
    /// Editor container – hosts sub-panels (Source, Wysiwyg, Preview, Outline).
    Editor,
}

impl WindowAreaKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Settings => "Settings",
            Self::Editor => "Editor",
        }
    }

    /// Outer layout area types (displayed in top-level dropdown).
    pub fn all() -> &'static [WindowAreaKind] {
        &[Self::Editor, Self::Explorer, Self::Settings]
    }
}

/// The two outer states an Editor area can be in.
///
/// Derived from whether the area's tab list is empty; switching happens
/// automatically when the first tab is created or the last one is closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorAreaMode {
    /// No document tabs: the area shows the welcome prompt (double-click to
    /// start editing) instead of any panel.
    Welcome,
    /// At least one document tab: the area renders its inner panel layout
    /// (Wysiwyg / Source Code / Preview / Outline).
    Editing,
}

impl EditorAreaMode {
    /// True when the area's session holds tabs.
    pub fn is_editing(self) -> bool {
        matches!(self, Self::Editing)
    }
}

/// Registration of the window-area kinds with the splitter engine.
impl ContainerKind for WindowAreaKind {
    fn shift_behavior(&self) -> ShiftBehavior {
        match self {
            WindowAreaKind::Settings => ShiftBehavior::Cancel,
            WindowAreaKind::Editor => ShiftBehavior::Preview { fresh: true },
            WindowAreaKind::Explorer => ShiftBehavior::Preview { fresh: false },
        }
    }

    fn is_editor(&self) -> bool {
        matches!(self, Self::Editor)
    }
}

/// The window-level split container: the outer area layout.
pub type WindowLayout = SplitterContainer<WindowAreaKind>;

/// The id of the root area created by the default layout.
pub const ROOT_AREA_ID: NodeId = 1;

/// The Editor area id of the default layout: the initial split is
/// Explorer (left) + Editor (right), and the split node shares the Editor
/// leaf's id by the tree's split-id convention.
pub const DEFAULT_EDITOR_AREA_ID: NodeId = 2;

/// The default window layout: Explorer (left, 30%) + Editor (right, 70%).
///
/// A constructor instead of a `Default` impl because the orphan rule
/// forbids implementing a foreign trait (std `Default`) for the foreign
/// `SplitterContainer<WindowAreaKind>` even with the local kind.
pub fn default_layout() -> WindowLayout {
    SplitterContainer {
        tree: splitype_splitter::tree::SplitTree::Split {
            id: DEFAULT_EDITOR_AREA_ID,
            direction: splitype_splitter::tree::Axis::Horizontal,
            ratio: 0.3,
            first: Box::new(splitype_splitter::tree::SplitTree::Leaf {
                id: ROOT_AREA_ID,
                kind: WindowAreaKind::Explorer,
            }),
            second: Box::new(splitype_splitter::tree::SplitTree::Leaf {
                id: DEFAULT_EDITOR_AREA_ID,
                kind: WindowAreaKind::Editor,
            }),
        },
        next_node_id: 3,
        open_dropdown: None,
        maximized_area: None,
        active_splitter_drag: None,
        active_corner_drag: None,
        active_border_menu: None,
        active_area: None,
        activation_history: Vec::new(),
        focused_area: None,
    }
}
