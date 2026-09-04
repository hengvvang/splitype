//! Editor session view models — tabs, pane view state, and the inner pane
//! split topology. The authoritative document state lives in
//! [`crate::document::DocumentBuffer`].

pub mod lifecycle;
pub mod ops;
pub mod pane_state;
pub mod tab;

use editor_contracts::{PaneKind, TabKind};
pub use pane_state::{PaneState, ScrollState};
pub use tab::{DocumentTab, PersistedTab, TabPendingState};

use gpui::App;
use splitter::root::SplitterRoot;

use crate::document::DocumentStore;

/// The document tabs owned by one Editor area.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::Deserialize<'de>"
))]
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

/// The complete per-area editor state: the document tabs plus the inner panel split container.
///
/// Not serializable: tabs hold live buffer entities. Durable persistence goes
/// through [`PersistedEditorSession`], which stores buffer identities only.
pub struct EditorSession {
    pub tab_list: EditorTabList<DocumentTab>,
    pub root: SplitterRoot<PaneKind>,
}

impl EditorSession {
    /// A fresh session: no tabs and a single default panel.
    pub fn empty() -> Self {
        let default_kind = editor_contracts::PaneRegistry::registered_default_kind()
            .ok()
            .flatten()
            .unwrap_or_default();
        Self {
            tab_list: EditorTabList::new(),
            root: SplitterRoot::single_leaf(1, default_kind),
        }
    }

    /// Durable projection referencing the same buffers by identity.
    pub fn to_persisted(&self, cx: &App) -> PersistedEditorSession {
        let mut tab_list = EditorTabList::new();
        for tab in self.tabs() {
            tab_list.push(PersistedTab {
                buffer: tab.buffer.read(cx).id,
                kind: tab.kind,
            });
        }
        tab_list.set_active_tab(self.active_tab_index());
        let mut next_id = self.root.next_node_id;
        let tree = self.root.tree.clone_with_new_ids(&mut next_id);
        PersistedEditorSession {
            tab_list,
            tree,
            next_node_id: next_id,
        }
    }

    /// Rebuilds a live session from a persisted projection, resolving each
    /// buffer from the store. Tabs whose buffer is missing are skipped.
    pub fn from_persisted(persisted: PersistedEditorSession, cx: &App) -> Self {
        let store = cx.global::<DocumentStore>();
        let active_index = persisted.tab_list.active_index();
        let mut tab_list = EditorTabList::new();
        for tab in persisted.tab_list.into_iter() {
            let Some(buffer) = store.get(tab.buffer) else {
                tracing::warn!("persisted tab references a missing buffer; skipping");
                continue;
            };
            tab_list.push(DocumentTab::new(buffer, tab.kind));
        }
        tab_list.set_active_tab(active_index);
        let root = SplitterRoot {
            tree: persisted.tree,
            next_node_id: persisted.next_node_id,
            active_splitter_drag: None,
            active_border_menu: None,
            active_leaf: None,
            activation_history: Vec::new(),
        };
        Self { tab_list, root }
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

    /// Whether this session has a dirty buffer whose every view lives in
    /// this session — i.e. closing this panel would lose unsaved content.
    pub fn has_unsaved_buffers(&self, cx: &App) -> bool {
        let store = cx.global::<DocumentStore>();
        self.tabs().any(|tab| {
            let buffer = tab.buffer.read(cx);
            if !buffer.dirty {
                return false;
            }
            let own_views = self
                .tabs()
                .filter(|other| other.buffer == tab.buffer)
                .count();
            store.view_count(buffer.id) == own_views
        })
    }
}

/// Durable projection of an [`EditorSession`]: buffer identities, tab kinds,
/// and the inner pane split topology.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PersistedEditorSession {
    pub tab_list: EditorTabList<PersistedTab>,
    pub tree: splitter::tree::SplitTree<PaneKind>,
    pub next_node_id: splitter::tree::NodeId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_tab_list_close_reindexes_active_tab() {
        let mut list = EditorTabList::<u32>::new();
        list.push(10);
        list.push(20);
        list.push(30);
        list.set_active_tab(2);
        assert_eq!(list.close_tab(0), Some(10));
        assert_eq!(list.active_index(), 1);
        assert_eq!(list.close_tab(1), Some(30));
        assert_eq!(list.active_index(), 0);
    }

    #[test]
    fn persisted_editor_session_round_trips_through_json() {
        let kind = PaneKind::from_static("splitype.pane.wysiwyg");
        let root = SplitterRoot::single_leaf(1, kind);
        let mut tab_list = EditorTabList::new();
        tab_list.push(PersistedTab {
            buffer: editor_contracts::DocumentId::new(),
            kind: TabKind::Persistent,
        });
        tab_list.set_active_tab(0);
        let persisted = PersistedEditorSession {
            tab_list,
            tree: root.tree,
            next_node_id: root.next_node_id,
        };

        let json = serde_json::to_value(&persisted).expect("serialize");
        let restored: PersistedEditorSession = serde_json::from_value(json).expect("deserialize");

        assert_eq!(restored.tab_list.len(), 1);
        assert_eq!(restored.tab_list.active_index(), 0);
        assert_eq!(restored.tree.count_leaves(), 1);
        assert_eq!(restored.next_node_id, 2);
    }
}
