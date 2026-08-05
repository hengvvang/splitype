//! Layout area type tags — the labels a `SplitTree` leaf carries.
//!
//! The engine itself is generic over the leaf type; these two enums are the
//! concrete labels used by the hosts: `WindowAreaKind` for the window-level area
//! layout, `EditorInnerPanelKind` for the editor's inner panel layout. The hosts
//! match on them when rendering.

/// Top-level area types in the tiled split layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAreaKind {
    /// File explorer / file tree.
    Explorer,
    /// Application settings panel.
    Settings,
    /// Editor container – hosts sub-panels (Source, Block, Preview, Outline).
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

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer file directory tree",
            Self::Settings => "Application settings & document info",
            Self::Editor => "Editor container with sub-panels",
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

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Self::SourceCode => "Raw Markdown source code editor",
            Self::Wysiwyg => "Visual block editor (Rendered)",
            Self::Preview => "Read-only rendered Markdown preview",
            Self::Outline => "Document section headings outline",
        }
    }

    /// All inner Edit sub-panel types.
    pub fn all() -> &'static [EditorInnerPanelKind] {
        &[
            Self::Wysiwyg,
            Self::Preview,
            Self::SourceCode,
            Self::Outline,
        ]
    }
}
