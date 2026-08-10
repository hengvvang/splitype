//! Layout area type tags — the labels a `SplitTree` leaf carries.
//!
//! The engine itself is generic over the leaf type; these two enums are the
//! concrete labels used by the hosts: `WindowAreaKind` for the window-level
//! area layout, `EditorInnerPanelKind` for the editor's inner panel layout.
//! The hosts match on them when rendering.
//!
//! The id types (`AreaId` / `PanelId` / `SplitId`) and [`InnerPanelLocation`]
//! give the layout engine's `usize` ids a name, so a signature reads as
//! "split which panel inside which area" without digging into callers.

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

/// Id of a top-level area in the outer layout tree (`window_area_tree`).
pub type AreaId = usize;

/// Id of a sub-panel inside an Editor area's inner panel tree.
pub type PanelId = usize;

/// Id of a split node in either layout tree.
///
/// Split nodes share the same id pool as leaves (a split's id equals its
/// second child leaf's id).
pub type SplitId = usize;

/// Locates an editor inner panel: which outer area, and which panel in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnerPanelLocation {
    pub area_id: AreaId,
    pub panel_id: PanelId,
}

/// How the new sibling area of a split is seeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaSplitMode {
    /// The sibling inherits the source area's kind; Editor areas clone
    /// their inner panel layout (and the host deep-copies the tab list).
    Copy,
    /// The sibling is a blank initial-state area of the same kind: the
    /// default inner panel layout and an empty tab list.
    Fresh,
}
