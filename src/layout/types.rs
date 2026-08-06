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

/// Editing 模式下的面板类型：Editor 有标签时，一个面板渲染的视图。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditingPanelKind {
    /// Raw Markdown source code editor.
    SourceCode,
    /// Visual block editor (WYSIWYG rendered view).
    Wysiwyg,
    /// Read-only rendered Markdown preview.
    Preview,
    /// Document section headings outline.
    Outline,
}

impl EditingPanelKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SourceCode => "Source Code",
            Self::Wysiwyg => "Wysiwyg",
            Self::Preview => "Preview",
            Self::Outline => "Outline",
        }
    }

    /// All editing-mode panel types (status-bar dropdown options).
    pub fn all() -> &'static [EditingPanelKind] {
        &[
            Self::Wysiwyg,
            Self::Preview,
            Self::SourceCode,
            Self::Outline,
        ]
    }
}

/// Welcome 模式下的面板类型。目前唯一的欢迎面板携带它退出编辑前的
/// 编辑面板类型（`None` = 从未编辑过），重新进入编辑时按它恢复；
/// 将来 welcome 模式可扩展更多面板（如"最近文件"）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WelcomePanelKind {
    Welcome(Option<EditingPanelKind>),
}

/// Sub-panel types available inside a `WindowAreaKind::Editor` container:
/// the outer variant is the mode, the inner variant is the panel type
/// within that mode. The tree always tells the truth — a welcome-mode
/// area holds `Welcome` panels, an editing-mode area holds `Editing`
/// panels — so rendering matches on the panel kind directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorInnerPanelKind {
    /// Welcome 模式统一面板：渲染欢迎提示（双击开始编辑）。
    Welcome(WelcomePanelKind),
    /// Editing 模式面板：渲染对应的编辑视图。
    Editing(EditingPanelKind),
}

impl EditorInnerPanelKind {
    /// The editing panel this panel becomes when the area enters editing:
    /// restores the remembered type, defaulting to `SourceCode` for a
    /// welcome panel that was never edited before.
    pub fn editing_kind(self) -> EditingPanelKind {
        match self {
            Self::Welcome(WelcomePanelKind::Welcome(Some(kind))) => kind,
            Self::Welcome(WelcomePanelKind::Welcome(None)) => EditingPanelKind::SourceCode,
            Self::Editing(kind) => kind,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Welcome(_) => "Welcome",
            Self::Editing(kind) => kind.name(),
        }
    }
}

/// Id of a top-level area in the outer layout tree (`window_area_tree`).
///
/// Also used as the key into `WindowLayout::editor_sessions`: each Editor
/// area owns its own editor session (tab list + inner panel tree).
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
