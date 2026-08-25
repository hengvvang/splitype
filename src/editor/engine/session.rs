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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorTabList<T> {
    tabs: Vec<T>,
    active_tab: usize,
}

impl<T> EditorTabList<T> {
    pub fn new() -> Self {
        Self::empty()
    }

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

    #[inline]
    pub fn active_index(&self) -> usize {
        self.active_tab
    }

    /// Pushes a new tab to the list and returns its index.
    pub fn push(&mut self, tab: T) -> usize {
        let index = self.tabs.len();
        self.tabs.push(tab);
        index
    }

    /// Safely gets a reference to the tab at `index`.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.tabs.get(index)
    }

    /// Safely gets a mutable reference to the tab at `index`.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.tabs.get_mut(index)
    }

    /// Safely gets a reference to the active tab.
    #[inline]
    pub fn active_tab(&self) -> Option<&T> {
        self.tabs.get(self.active_tab)
    }

    /// Safely gets a mutable reference to the active tab.
    #[inline]
    pub fn active_tab_mut(&mut self) -> Option<&mut T> {
        let index = self.active_tab;
        self.tabs.get_mut(index)
    }

    /// Safely sets the active tab index with bounds clamping.
    pub fn set_active_tab(&mut self, index: usize) {
        if self.tabs.is_empty() {
            self.active_tab = 0;
        } else {
            self.active_tab = index.min(self.tabs.len() - 1);
        }
    }

    /// Safely closes the tab at `index`, adjusting `active_tab` automatically.
    pub fn close_tab(&mut self, index: usize) -> Option<T> {
        if index >= self.tabs.len() {
            return None;
        }
        let was_active = index == self.active_tab;
        let removed = self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active_tab = 0;
        } else if was_active {
            self.active_tab = index.min(self.tabs.len() - 1);
        } else if index < self.active_tab {
            self.active_tab -= 1;
        }
        Some(removed)
    }

    pub fn clear(&mut self) {
        self.tabs.clear();
        self.active_tab = 0;
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.tabs
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a < self.tabs.len() && b < self.tabs.len() {
            self.tabs.swap(a, b);
            if self.active_tab == a {
                self.active_tab = b;
            } else if self.active_tab == b {
                self.active_tab = a;
            }
        }
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.tabs.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.tabs.iter_mut()
    }
}

impl<'a, T> IntoIterator for &'a EditorTabList<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut EditorTabList<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
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
    pub(crate) tab_list: EditorTabList<crate::editor::engine::controller::DocumentTab>,
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

    #[inline]
    pub fn has_tabs(&self) -> bool {
        !self.tab_list.is_empty()
    }

    #[inline]
    pub fn tab_count(&self) -> usize {
        self.tab_list.len()
    }

    #[inline]
    pub fn active_tab_index(&self) -> usize {
        self.tab_list.active_index()
    }

    #[inline]
    pub(crate) fn active_tab(&self) -> Option<&crate::editor::engine::controller::DocumentTab> {
        self.tab_list.active_tab()
    }

    #[inline]
    pub(crate) fn active_tab_mut(&mut self) -> Option<&mut crate::editor::engine::controller::DocumentTab> {
        self.tab_list.active_tab_mut()
    }

    #[inline]
    pub(crate) fn tab(&self, index: usize) -> Option<&crate::editor::engine::controller::DocumentTab> {
        self.tab_list.get(index)
    }

    #[inline]
    pub(crate) fn tab_mut(&mut self, index: usize) -> Option<&mut crate::editor::engine::controller::DocumentTab> {
        self.tab_list.get_mut(index)
    }

    #[inline]
    pub fn set_active_tab(&mut self, index: usize) {
        self.tab_list.set_active_tab(index);
    }

    #[inline]
    pub(crate) fn close_tab(&mut self, index: usize) -> Option<crate::editor::engine::controller::DocumentTab> {
        self.tab_list.close_tab(index)
    }

    #[inline]
    pub(crate) fn tabs(&self) -> impl Iterator<Item = &crate::editor::engine::controller::DocumentTab> {
        self.tab_list.iter()
    }

    #[inline]
    pub(crate) fn tabs_mut(&mut self) -> impl Iterator<Item = &mut crate::editor::engine::controller::DocumentTab> {
        self.tab_list.iter_mut()
    }

    #[inline]
    pub(crate) fn push_tab(&mut self, tab: crate::editor::engine::controller::DocumentTab) -> usize {
        self.tab_list.push(tab)
    }

    #[inline]
    pub(crate) fn clear_tabs(&mut self) {
        self.tab_list.clear();
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
