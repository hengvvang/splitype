//! Editor session types — the per-panel tab list, the pane split root,
//! and the pane-kind vocabulary of an Editor area.
//!
//! The pane layout is a [`SplitterRoot`] — the same generic split root
//! the window-level panel layout uses — so both levels share one split
//! model and one set of interactions (see `splitype-splitter`).

use gpui::{Pixels, Size};
use splitype_splitter::container::SplitterContainer;
use splitype_splitter::policy::{ClonedContainer, DragPolicy};
use splitype_splitter::root::SplitterRoot;
use splitype_splitter::sessions::CornerDragSession;

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

    /// Safely gets a mutable reference to the active tab.
    #[inline]
    pub fn active_tab_mut(&mut self) -> Option<&mut T> {
        self.tabs.get_mut(self.active_tab)
    }

    /// Safely selects a tab by index, maintaining bounds invariant.
    pub fn select_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab = index;
            true
        } else {
            false
        }
    }

    /// Removes a tab at `index` and clamps `active_tab` within valid bounds.
    pub fn remove_tab(&mut self, index: usize) -> Option<T> {
        if index >= self.tabs.len() {
            return None;
        }
        let removed = self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active_tab = 0;
        } else if self.active_tab >= index && self.active_tab > 0 {
            self.active_tab = self.active_tab.min(self.tabs.len() - 1);
        }
        Some(removed)
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

/// Pane containers override the Shift-drag default (which opens
/// the dragged panel in a new window): dragging a pane.s corner
/// with Shift is a no-op.
/// Plain drags, Ctrl swaps, and Alt keep the shared defaults.
impl DragPolicy<EditorPaneKind> for SplitterContainer<EditorPaneKind> {
    fn on_shift_drag(
        _root: &mut SplitterRoot<EditorPaneKind>,
        _facts: &CornerDragSession,
        _container_size: Size<Pixels>,
    ) -> Option<ClonedContainer<EditorPaneKind>> {
        // Empty override: Shift + drag on a pane does nothing.
        None
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

    #[inline]
    pub fn is_preview(&self) -> bool {
        matches!(self, Self::Preview)
    }

    #[inline]
    pub fn is_outline(&self) -> bool {
        matches!(self, Self::Outline)
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
