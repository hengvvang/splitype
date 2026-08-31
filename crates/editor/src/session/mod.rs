//! Editor session domain models — tabs, buffer/source-of-truth, and file state.

pub mod file;
pub mod ops;
pub mod tab;

pub use core_contracts::{OpenFileMode, PaneKind, TabKind};
pub use file::{FileState, PendingOpenLink};
pub use tab::{DocumentTab, PaneState, ScrollState, ScrollbarDragSession};

use splitter::root::SplitterRoot;

/// The document tabs owned by one Editor area.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorTabList<T> {
    tabs: Vec<T>,
    active_tab: usize,
}

impl<T> EditorTabList<T> {
    #[inline]
    pub fn new() -> Self {
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

    #[inline]
    pub fn push(&mut self, tab: T) -> usize {
        let index = self.tabs.len();
        self.tabs.push(tab);
        index
    }

    pub fn replace(&mut self, index: usize, new_tab: T) -> Option<T> {
        if index < self.tabs.len() {
            Some(std::mem::replace(&mut self.tabs[index], new_tab))
        } else {
            None
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.tabs.get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.tabs.get_mut(index)
    }

    #[inline]
    pub fn active_tab(&self) -> Option<&T> {
        self.tabs.get(self.active_tab)
    }

    #[inline]
    pub fn active_tab_mut(&mut self) -> Option<&mut T> {
        let index = self.active_tab;
        self.tabs.get_mut(index)
    }

    pub fn set_active_tab(&mut self, index: usize) {
        if self.tabs.is_empty() {
            self.active_tab = 0;
        } else {
            self.active_tab = index.min(self.tabs.len() - 1);
        }
    }

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

    #[inline]
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

impl<T> std::ops::Index<usize> for EditorTabList<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.tabs[index]
    }
}

impl<T> std::ops::IndexMut<usize> for EditorTabList<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.tabs[index]
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

/// The complete per-area editor state: the document tabs plus the inner panel split container.
pub struct EditorSession {
    pub tab_list: EditorTabList<DocumentTab>,
    pub root: SplitterRoot<PaneKind>,
    pub empty_panes: std::collections::HashMap<core_contracts::PaneId, PaneState>,
}

impl EditorSession {
    /// A fresh session: no tabs and a single default panel.
    pub fn empty() -> Self {
        let default_kind = core_contracts::PaneRegistry::global()
            .lock()
            .unwrap()
            .default_kind()
            .unwrap_or_default();
        Self {
            tab_list: EditorTabList::new(),
            root: SplitterRoot::single_leaf(1, default_kind),
            empty_panes: std::collections::HashMap::new(),
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
    pub fn active_tab(&self) -> Option<&DocumentTab> {
        self.tab_list.active_tab()
    }

    #[inline]
    pub fn active_tab_mut(&mut self) -> Option<&mut DocumentTab> {
        self.tab_list.active_tab_mut()
    }

    #[inline]
    pub fn tab(&self, index: usize) -> Option<&DocumentTab> {
        self.tab_list.get(index)
    }

    #[inline]
    pub fn tab_mut(&mut self, index: usize) -> Option<&mut DocumentTab> {
        self.tab_list.get_mut(index)
    }

    #[inline]
    pub fn set_active_tab(&mut self, index: usize) {
        self.tab_list.set_active_tab(index);
    }

    #[inline]
    pub fn close_tab(&mut self, index: usize) -> Option<DocumentTab> {
        self.tab_list.close_tab(index)
    }

    #[inline]
    pub fn tabs(&self) -> impl Iterator<Item = &DocumentTab> {
        self.tab_list.iter()
    }

    #[inline]
    pub fn tabs_mut(&mut self) -> impl Iterator<Item = &mut DocumentTab> {
        self.tab_list.iter_mut()
    }

    #[inline]
    pub fn push_tab(&mut self, tab: DocumentTab) -> usize {
        self.tab_list.push(tab)
    }

    #[inline]
    pub fn clear_tabs(&mut self) {
        self.tab_list.clear();
    }

    #[inline]
    pub fn has_dirty_tabs(&self) -> bool {
        self.tabs().any(|tab| tab.file.dirty)
    }
}


