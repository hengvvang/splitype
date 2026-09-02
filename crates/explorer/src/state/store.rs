//! Process-level registry of shared file trees.
//!
//! Mirrors the editor's `DocumentStore`: one scanned [`Worktree`] per folder
//! root, shared by every explorer panel that shows it, with per-view
//! reference counting. Split explorers and cloned windows therefore observe
//! the same scanned tree — a rename in one panel is visible everywhere the
//! moment the single scan finishes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, App, Entity};

use crate::state::worktree::{Worktree, WorktreeId};

/// The process-global worktree registry: which folder roots are scanned and
/// how many panel views reference each tree.
///
/// Lifetime rules:
/// - Every panel view acquires a reference; every removed view releases one.
/// - A tree is dropped (watcher and scans stop) when its last view releases.
#[derive(Default)]
pub struct WorktreeStore {
    trees: HashMap<PathBuf, Entity<Worktree>>,
    refs: HashMap<PathBuf, usize>,
    next_worktree_id: u64,
}

impl gpui::Global for WorktreeStore {}

impl WorktreeStore {
    /// Installs the store as a process global. Called once at startup.
    pub fn init(cx: &mut App) {
        cx.set_global(Self::default());
    }

    /// Returns the shared tree for `root`, scanning it when this is the
    /// first view of the folder. Existing trees sync the caller's
    /// dotfile-visibility setting instead of rescanning.
    pub fn open(
        root: PathBuf,
        hide_hidden: bool,
        window_handle: Option<AnyWindowHandle>,
        cx: &mut App,
    ) -> Entity<Worktree> {
        let root = canonical_root(root);
        if let Some(tree) = cx.global::<Self>().resolve(&root) {
            tree.update(cx, |tree, cx| tree.set_hide_hidden(hide_hidden, cx));
            return tree;
        }
        let id = cx.global_mut::<Self>().allocate_id();
        let tree = Worktree::new(id, root.clone(), hide_hidden, window_handle, cx);
        cx.global_mut::<Self>().insert(root, tree.clone());
        tree
    }

    /// The shared tree for `root`, if one is registered.
    pub fn resolve(&self, root: &Path) -> Option<Entity<Worktree>> {
        self.trees.get(root).cloned()
    }

    /// Registers one panel view of the tree rooted at `root`.
    pub fn acquire(&mut self, root: &Path) {
        *self.refs.entry(root.to_path_buf()).or_insert(0) += 1;
    }

    /// Releases one panel view. The tree is dropped with its last view.
    pub fn release(&mut self, root: &Path) {
        let Some(count) = self.refs.get_mut(root) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            self.refs.remove(root);
            self.trees.remove(root);
        }
    }

    fn allocate_id(&mut self) -> WorktreeId {
        self.next_worktree_id += 1;
        WorktreeId(self.next_worktree_id)
    }

    fn insert(&mut self, root: PathBuf, tree: Entity<Worktree>) {
        self.trees.insert(root, tree);
    }
}

/// Canonicalizes a folder root into a stable registry key.
fn canonical_root(root: PathBuf) -> PathBuf {
    std::fs::canonicalize(&root).unwrap_or(root)
}
