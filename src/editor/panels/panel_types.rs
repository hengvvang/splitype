//! Editor panel types — the inner-panel vocabulary of an Editor area.
//!
//! Split out of `splitype-layout` when the inner panel layout moved back to
//! the editor layer: the layout crate now owns only the outer window tree,
//! while the editor owns its own panel kinds, tab list, and panel split
//! tree. `SplitTree` itself stays generic and is reused from the layout
//! crate.

use splitype_layout::tree::SplitTree;

/// The document tabs owned by one Editor area.
///
/// Every Editor area keeps its own ordered tab list; tabs are deep-copied
/// when an Editor area is split (normal drag) and start empty for fresh
/// editors (Shift-drag).
/// Tab payload type is owned by the host (editor); layout only stores and
/// reorders tabs, so it stays generic over the payload.
#[derive(Debug, Default, Clone)]
pub struct EditorTabList<T> {
    pub tabs: Vec<T>,
    pub active_tab: usize,
}

impl<T> EditorTabList<T> {
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
        }
    }
}

/// The complete per-area editor state: the document tabs plus the inner
/// panel split tree.
///
/// Aggregating both under one key guarantees they can never drift apart —
/// an area always has exactly one tab list and one panel layout. Sessions
/// are created lazily and survive a switch away from Editor (background
/// editing) so the tabs are restored when the area becomes Editor again.
/// A retained session is a pure cache: it never participates in explorer
/// or activation logic until its area is back in the foreground.
pub struct EditorSession {
    pub(crate) tab_list: EditorTabList<crate::editor::controller::DocumentTab>,
    pub(crate) inner_panel_tree: SplitTree<EditorInnerPanelKind>,
}

/// Welcome 模式下的面板类型。目前唯一的欢迎面板携带它退出编辑前的
/// 编辑面板类型（`None` = 从未编辑过），重新进入编辑时按它恢复；
/// 将来 welcome 模式可扩展更多面板（如"最近文件"）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WelcomePanelKind {
    Welcome(Option<EditingPanelKind>),
}

/// Sub-panel types available inside an Editor container: the outer variant
/// is the mode, the inner variant is the panel type within that mode. The
/// tree always tells the truth — a welcome-mode area holds `Welcome`
/// panels, an editing-mode area holds `Editing` panels — so rendering
/// matches on the panel kind directly.
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

/// Editing-mode panel types (WYSIWYG / source code / preview / outline).
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

/// Locates an editor inner panel: which outer area, and which panel in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnerPanelLocation {
    pub area_id: splitype_layout::types::AreaId,
    pub panel_id: splitype_layout::types::PanelId,
}

/// The action produced by an editor inner-panel corner-drag gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorInnerPanelDragAction {
    /// Split the dragged panel; the new panel inherits its kind.
    Split {
        panel_id: splitype_layout::types::PanelId,
        direction: splitype_layout::tree::Axis,
        ratio: f32,
    },
    /// Join the dragged panel into a neighbor (removes the dragged panel).
    Join {
        from_panel: splitype_layout::types::PanelId,
        into_panel: splitype_layout::types::PanelId,
    },
    /// Swap the kinds of two panels.
    Swap {
        from: splitype_layout::types::PanelId,
        to: splitype_layout::types::PanelId,
    },
    /// Shift + drag: currently a no-op in the host.
    Duplicate {
        panel_id: splitype_layout::types::PanelId,
    },
    /// Gesture was cancelled (e.g. dragged to an invalid target).
    Cancel,
}
