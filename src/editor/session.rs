//! Editor session types — the per-panel tab list, the pane split root,
//! and the pane-kind vocabulary of an Editor area.
//!
//! The pane layout is a [`SplitterRoot`] — the same generic split root
//! the window-level panel layout uses — so both levels share one split
//! model and one set of interactions (see `splitype-splitter`).

use splitype_splitter::root::SplitterRoot;

/// The document tabs owned by one Editor area.
///
/// Every Editor area keeps its own ordered tab list; tabs are deep-copied
/// when an Editor area is split (normal drag) and start empty for fresh
/// editors (Shift-drag).
/// Tab payload type is owned by the host (editor); the container only
/// stores and reorders tabs, so it stays generic over the payload.
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

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Safely gets a reference to the active tab.
    #[inline]
    pub fn active_tab(&self) -> Option<&T> {
        self.tabs.get(self.active_tab)
    }
}

/// The complete per-area editor state: the document tabs plus the inner
/// panel split container.
///
/// Aggregating both under one key guarantees they can never drift apart —
/// an area always has exactly one tab list and one panel layout. Sessions
/// are created lazily and survive a switch away from Editor (background
/// editing) so the tabs are restored when the area becomes Editor again.
/// A retained session is a pure cache: it never participates in explorer
/// or activation logic until its area is back in the foreground.
pub struct EditorSession {
    pub(crate) tab_list: EditorTabList<crate::editor::controller::DocumentTab>,
    /// The pane layout's split root: the pane tree, its
    /// operations, and the active drag sessions.
    pub(crate) root: SplitterRoot<EditorPaneKind>,
}

impl EditorSession {
    /// A fresh session: no tabs and a single default panel. The inner root
    /// is fully self-contained — it numbers its own nodes from 1, so
    /// nested roots never share state with the outer layout.
    pub(crate) fn welcome() -> Self {
        Self {
            tab_list: EditorTabList::empty(),
            root: SplitterRoot::single_leaf(1, EditorPaneKind::SourceCode),
        }
    }
}



/// The pane kinds an Editor panel can host: the document views
/// inside its split tree. The tree holds only real views — the welcome
/// state is the area's mode (`EditorPanelMode`), not a panel kind — so the
/// split structure survives tab open/close cycles unchanged and the
/// remembered panel layout needs no migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPaneKind {
    /// Raw Markdown source code editor.
    SourceCode,
    /// Visual block editor (WYSIWYG rendered view).
    Wysiwyg,
    /// Read-only rendered Markdown preview.
    Preview,
    /// Document section headings outline.
    Outline,
}

impl EditorPaneKind {
    #[inline]
    pub fn is_wysiwyg(&self) -> bool {
        matches!(self, Self::Wysiwyg)
    }

    #[inline]
    pub fn is_source_code(&self) -> bool {
        matches!(self, Self::SourceCode)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::SourceCode => "Source Code",
            Self::Wysiwyg => "Wysiwyg",
            Self::Preview => "Preview",
            Self::Outline => "Outline",
        }
    }

    /// All editor pane types (status-bar dropdown options).
    pub fn all() -> &'static [EditorPaneKind] {
        &[
            Self::Wysiwyg,
            Self::Preview,
            Self::SourceCode,
            Self::Outline,
        ]
    }
}
