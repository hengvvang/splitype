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

/// Sub-panel types available inside a `WindowAreaKind::Editor` container.
///
/// Each Editor area hosts one active file. The file is sent as a single input
/// source to each sub-panel channel (Source / Wysiwyg / Preview / Outline),
/// which process it independently and render in their own tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorInnerPanelKind {
    /// Raw Markdown source code editor.
    SourceCode,
    /// Visual block editor (WYSIWYG rendered view).
    Wysiwyg,
    /// Read-only rendered Markdown preview.
    Preview,
    /// Document section headings outline.
    Outline,
}

impl EditorInnerPanelKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SourceCode => "Source Code",
            Self::Wysiwyg => "Wysiwyg",
            Self::Preview => "Preview",
            Self::Outline => "Outline",
        }
    }

    /// All inner Editor sub-panel types.
    pub fn all() -> &'static [EditorInnerPanelKind] {
        &[
            Self::Wysiwyg,
            Self::Preview,
            Self::SourceCode,
            Self::Outline,
        ]
    }
}

/// Id of a top-level area in the outer layout tree (`window_area_tree`).
///
/// Also used as the key into `WindowLayout::editor_inner_panel_trees`: each
/// Editor area owns its own inner panel tree.
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
