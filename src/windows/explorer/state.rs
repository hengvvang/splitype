//! ExplorerState sidebar model — file-tree scanning, flat visible entry
//! list, and outline state.
//!
//! This module owns the pure explorer model:
//! - [`ExplorerEntryId`] — stable ids for file-tree nodes (hashed from the
//!   absolute path, so rescanning yields the same ids).
//! - [`ExplorerFileNode`] — the full scanned tree (background-thread
//!   product, never mutated after scan).
//! - [`VisibleExplorerEntry`] — one flat row per visible entry, the data
//!   source for the virtualized list (mirrors Zed's `visible_entries`).
//! - [`ExplorerNode`] — the outline tree (headings only).
//! - [`ExplorerState`] — the combined sidebar state.
//!
//! Outline parsing lives in `crate::editor::panels::outline`; rendering and
//! editor interactions stay in `super::mod`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, Bounds, Entity, FocusHandle, Pixels, Task, UniformListScrollHandle};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use super::undo::ExplorerUndoHistory;
use super::worktree::{Worktree, WorktreeSnapshot};

pub use super::worktree::WorktreeEvent;

// ── Icons & constants ───────────────────────────────────────────────────

pub const FOLDER_ICON: &str = "icon/explorer/folder.svg";
pub const MARKDOWN_ICON: &str = "icon/explorer/markdown.svg";
pub const FILE_ICON: &str = "icon/explorer/file.svg";
pub const EXPLORER_NODE_HEIGHT: f32 = 28.0;
pub const EXPLORER_NODE_INDENT: f32 = 14.0;

// ── Stable entry ids ────────────────────────────────────────────────────

/// Stable id for a file-tree entry, allocated from a shared counter (Zed's
/// `ProjectEntryId`): ids never change across renames or moves, so expansion
/// and selection state survives them. The counter is shared by all
/// worktrees, making ids globally unique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExplorerEntryId(pub u64);

impl ExplorerEntryId {
    /// Derive the stable id for an absolute path.
    pub fn for_path(path: &Path) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        Self(hasher.finish())
    }
}

// ── File-tree node types ────────────────────────────────────────────────

/// What kind of filesystem entry a file-tree node represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerEntryKind {
    Directory,
    MarkdownFile,
    File,
}

/// A node in the scanned file tree. Produced entirely on a background
/// thread by [`scan_explorer_dir`]; never mutated afterwards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerFileNode {
    pub id: ExplorerEntryId,
    pub path: PathBuf,
    pub label: String,
    pub kind: ExplorerEntryKind,
    pub children: Vec<ExplorerFileNode>,
}

/// One visible row in the file-tree list. The flat list is derived from the
/// scanned tree plus the expansion set (mirrors Zed's `visible_entries`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleExplorerEntry {
    /// Index of the worktree this entry belongs to.
    pub root: usize,
    pub id: ExplorerEntryId,
    pub parent_id: Option<ExplorerEntryId>,
    pub path: PathBuf,
    pub label: String,
    pub depth: usize,
    pub kind: ExplorerEntryKind,
    pub is_expanded: bool,
    pub has_children: bool,
}

// ── Outline node types ──────────────────────────────────────────────────

/// A node in the outline tree (headings only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerNode {
    pub id: String,
    pub label: String,
    pub kind: ExplorerNodeKind,
    pub children: Vec<ExplorerNode>,
}

/// Outline node kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerNodeKind {
    Heading { line: usize, level: u8 },
}

/// Which item is currently selected in the explorer sidebar.
///
/// Files are keyed by the Zed-style double key `(root, entry)`: the
/// worktree index plus the worktree-allocated stable entry id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerSelection {
    File {
        root: usize,
        entry: ExplorerEntryId,
    },
    Outline(String),
}

/// Validation feedback for the inline filename editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerValidation {
    Warning(String),
    Error(String),
}

/// Text buffer of the inline filename editor. Offsets are UTF-8 byte
/// offsets; GPUI's IME layer speaks UTF-16 and is bridged in
/// `filename_editor.rs`.
#[derive(Clone, Debug, Default)]
pub struct ExplorerFilenameEditor {
    pub text: String,
    pub selection: Range<usize>,
    pub reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub focus_handle: Option<FocusHandle>,
    /// Bounds of the input element from the last frame (IME hit-testing).
    pub last_bounds: Option<Bounds<Pixels>>,
}

/// Inline create/rename state (mirrors Zed's `EditState`).
///
/// `target_id == None` means a new entry is being created inside the
/// directory `parent_id`; `Some(id)` means the entry `id` is being renamed.
#[derive(Clone, Debug)]
pub struct ExplorerEditState {
    /// Index of the worktree the edit happens in.
    pub root: usize,
    /// Parent directory id for a new entry; `None` for a root-level create.
    pub parent_id: Option<ExplorerEntryId>,
    /// `None` = creating a new entry, `Some` = renaming this entry.
    pub target_id: Option<ExplorerEntryId>,
    /// Whether the entry being created is a directory.
    pub is_dir: bool,
    /// Row depth of the edit row in the flat list.
    pub depth: usize,
    /// Parent directory path (create) or the entry's own path (rename).
    pub path: PathBuf,
    pub validation: Option<ExplorerValidation>,
    pub filename: ExplorerFilenameEditor,
    /// Selection to restore when the edit is cancelled.
    pub previously_selected: Option<ExplorerSelection>,
    /// True while a confirm operation is in flight; blocks re-confirms.
    pub processing: bool,
}

/// One row of the virtualized file-tree list: either a visible entry or the
/// inline edit row. `root` is the worktree index the row belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerRow {
    Entry(VisibleExplorerEntry),
    Edit { root: usize },
}

/// In-panel clipboard for cut/copy/paste of file-tree entries (mirrors
/// Zed's `ClipboardEntry`). The system clipboard additionally receives the
/// absolute paths as text for use outside the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerClipboard {
    Copied(Vec<ExplorerSelection>),
    Cut(Vec<ExplorerSelection>),
}
    /// Payload for dragging file-tree entries within the panel (mirrors
    /// Zed's `DraggedSelection`). The first selection is the row where the
    /// drag started (`active_selection`); the rest are the marked entries it
    /// carries along.
#[derive(Clone, Debug)]
pub struct DraggedExplorerSelection {
    pub selections: Vec<ExplorerSelection>,
}

impl DraggedExplorerSelection {
    /// The entry the drag started on (mirrors Zed's `active_selection`).
    pub fn active(&self) -> Option<&ExplorerSelection> {
        self.selections.first()
    }
}

/// The current drag-and-drop target of the panel (mirrors Zed's `DragTarget`).
///
/// `entry_id` is the entry under the pointer (the drop and hover-expand
/// target); `highlight_entry_id` is the entry whose highlight should extend
/// to all of its descendants. A directory highlights itself, a file
/// highlights its parent directory — mirroring Zed's
/// `highlight_entry_for_external_drag`/`highlight_entry_for_selection_drag`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragExplorerTarget {
    /// The entry under the pointer plus the entry to highlight (a directory
    /// highlights itself, a file highlights its parent directory — and the
    /// highlight extends to all descendants, mirroring Zed).
    Entry {
        entry_id: ExplorerEntryId,
        highlight_entry_id: ExplorerEntryId,
    },
    /// Dropping on the empty area targets the explorer root.
    Background,
}

impl DragExplorerTarget {
    /// The entry currently under the pointer, if any.
    pub fn entry_id(&self) -> Option<ExplorerEntryId> {
        match self {
            Self::Entry { entry_id, .. } => Some(*entry_id),
            Self::Background => None,
        }
    }

    /// The entry whose highlight should be shown, if any.
    pub fn highlight_entry_id(&self) -> Option<ExplorerEntryId> {
        match self {
            Self::Entry {
                highlight_entry_id, ..
            } => Some(*highlight_entry_id),
            Self::Background => None,
        }
    }
}

impl ExplorerClipboard {
    pub fn is_cut(&self) -> bool {
        matches!(self, Self::Cut(_))
    }

    pub fn items(&self) -> &[ExplorerSelection] {
        match self {
            Self::Copied(items) | Self::Cut(items) => items,
        }
    }

    /// After the first paste a cut degrades into a copy (Zed).
    pub fn into_copied(self) -> Self {
        match self {
            Self::Cut(items) => Self::Copied(items),
            copied => copied,
        }
    }
}

/// Top-level explorer sidebar state.
pub struct ExplorerState {
    pub is_open: bool,
    /// Worktree entities in display order (mirrors Zed's `visible_worktrees`).
    pub worktrees: Vec<Entity<Worktree>>,
    /// Shared stable-id allocator across all worktrees (Zed's `WorktreeStore`).
    pub next_entry_id: Arc<AtomicU64>,
    /// Expanded directory ids per worktree index (Zed's `expanded_dir_ids`).
    pub expanded: HashMap<usize, BTreeSet<ExplorerEntryId>>,
    /// Rebuilt tree per worktree, refreshed whenever a worktree scan
    /// completes (the panel's working copy of each snapshot).
    pub trees_cache: Vec<ExplorerFileNode>,
    pub file_error: Option<String>,
    pub outline_tree: Vec<ExplorerNode>,
    pub outline_source: Option<String>,
    /// Expanded outline node ids (kept separate: outline ids are strings).
    pub expanded_outline: HashSet<String>,
    /// Flat visible rows — the virtualized list's data source (includes the
    /// inline edit row while an edit is active).
    pub entries: Vec<ExplorerRow>,
    /// Selection, keyed by the double key `(root, entry)`.
    pub selected: Option<ExplorerSelection>,
    /// Multi-select marks (Zed's `marked_entries`).
    pub marked: Vec<ExplorerSelection>,
    /// In-panel cut/copy clipboard.
    pub clipboard: Option<ExplorerClipboard>,
    /// Undo/redo stacks for file operations.
    pub undo_history: ExplorerUndoHistory,
    /// Current drag target while a drag is in flight.
    pub drag_target: Option<DragExplorerTarget>,
    /// Delayed task that expands a hovered directory during a drag.
    pub hover_expand_task: Option<Task<()>>,
    /// Continuous scroll task while a drag hovers the list edges (Zed's
    /// `hover_scroll_task`).
    pub hover_scroll_task: Option<Task<()>>,
    /// Bumped whenever a drag move replaces the scroll task or a drop
    /// clears it — stale tasks detect the mismatch and stop themselves.
    pub hover_scroll_generation: u64,
    /// Last drag-move position — drags only refresh the cursor style and
    /// scroll when the pointer actually moved.
    pub previous_drag_position: Option<gpui::Point<Pixels>>,
    /// Path (and worktree index) to select once the next scan completes.
    pub pending_select: Option<(usize, PathBuf)>,
    /// Copy-collision rename to start once the next scan makes the entry
    /// visible (the inline rename editor needs the scanned tree to resolve
    /// the entry).
    pub pending_rename: Option<(AnyWindowHandle, PathBuf)>,
    /// Active inline create/rename state.
    pub edit: Option<ExplorerEditState>,
    /// Scroll handle bound to the virtualized file-tree list.
    pub scroll_handle: UniformListScrollHandle,
    /// Number of rows rendered in the last frame (used for page scrolling).
    pub rendered_rows: usize,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self {
            is_open: false,
            worktrees: Vec::new(),
            next_entry_id: Arc::new(AtomicU64::new(1)),
            expanded: HashMap::new(),
            trees_cache: Vec::new(),
            file_error: None,
            outline_tree: Vec::new(),
            outline_source: None,
            expanded_outline: HashSet::new(),
            entries: Vec::new(),
            selected: None,
            marked: Vec::new(),
            clipboard: None,
            undo_history: ExplorerUndoHistory::default(),
            drag_target: None,
            hover_expand_task: None,
            hover_scroll_task: None,
            hover_scroll_generation: 0,
            previous_drag_position: None,
            pending_select: None,
            pending_rename: None,
            edit: None,
            scroll_handle: UniformListScrollHandle::new(),
            rendered_rows: 0,
        }
    }
}

// ── Filesystem helpers ──────────────────────────────────────────────────

/// Stable numeric hash of an id, for use as a DOM element id suffix.
pub fn stable_node_hash(id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

// ── Flat visible list derivation ────────────────────────────────────────

/// Rebuild the tree model for one worktree from its flat snapshot. The
/// snapshot's `entries_by_path` is ordered, so a single pass with a depth
/// stack recovers the parent/child structure in O(n). Children are sorted
/// directories-first, then case-insensitively by label (the explorer's
/// default ordering).
pub fn build_tree_from_snapshot(snapshot: &WorktreeSnapshot) -> Option<ExplorerFileNode> {
    let root_entry = snapshot.entries_by_path.values().next()?;
    let mut arena = vec![make_tree_node(root_entry)];
    let mut stack: Vec<usize> = vec![0]; // arena index of the last node at each depth
    for entry in snapshot.entries_by_path.values().skip(1) {
        let rel = entry.path.strip_prefix(&arena[0].path).ok()?;
        let depth = rel.components().count();
        while stack.len() > depth {
            stack.pop();
        }
        let parent = *stack.last()?;
        let idx = arena.len();
        let node = make_tree_node(entry);
        arena[parent].children.push(node.clone());
        arena.push(node);
        stack.push(idx);
    }
    for node in &mut arena {
        node.children.sort_by(|left, right| {
            let left_dir = left.kind == ExplorerEntryKind::Directory;
            let right_dir = right.kind == ExplorerEntryKind::Directory;
            right_dir
                .cmp(&left_dir)
                .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
        });
    }
    arena.into_iter().next()
}

fn make_tree_node(entry: &super::worktree::WorktreeEntry) -> ExplorerFileNode {
    ExplorerFileNode {
        id: ExplorerEntryId(entry.id),
        path: entry.path.clone(),
        label: entry
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry.path.to_string_lossy().into_owned()),
        kind: match entry.kind {
            super::worktree::WorktreeEntryKind::Directory => ExplorerEntryKind::Directory,
            super::worktree::WorktreeEntryKind::File => {
                if entry.path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
                    ExplorerEntryKind::MarkdownFile
                } else {
                    ExplorerEntryKind::File
                }
            }
        },
        children: Vec::new(),
    }
}

/// Flatten one worktree's tree into the visible row list. The root itself
/// is the first row (depth 0, foldable — mirroring Zed's default `hide_root:
/// false` rendering); its children are visible only while the root is
/// expanded. A directory's children are included only when its id is in
/// `expanded` (traversal pruning, mirroring Zed's `advance_to_sibling`).
pub fn flatten_file_tree(
    root_index: usize,
    root: &ExplorerFileNode,
    expanded: &BTreeSet<ExplorerEntryId>,
) -> Vec<VisibleExplorerEntry> {
    let mut out = Vec::new();
    out.push(VisibleExplorerEntry {
        root: root_index,
        id: root.id,
        parent_id: None,
        path: root.path.clone(),
        label: root.label.clone(),
        depth: 0,
        kind: ExplorerEntryKind::Directory,
        is_expanded: expanded.contains(&root.id),
        has_children: !root.children.is_empty(),
    });
    if out[0].is_expanded {
        flatten_children(root_index, &root.children, Some(root.id), 1, expanded, &mut out);
    }
    out
}

/// Derive the flat row list from every worktree's tree plus the per-worktree
/// expansion sets, then splice the inline edit row into its position
/// (create: after its parent row; rename: replacing the target row).
/// Worktree segments are concatenated in order (Zed's
/// `VisibleEntriesForWorktree`).
pub fn build_explorer_rows(
    trees: &[(usize, &ExplorerFileNode)],
    expanded: &HashMap<usize, BTreeSet<ExplorerEntryId>>,
    edit: Option<&ExplorerEditState>,
) -> Vec<ExplorerRow> {
    let mut rows = Vec::new();
    for (root_index, tree) in trees {
        let expanded_set = expanded.get(root_index).cloned().unwrap_or_default();
        let flat = flatten_file_tree(*root_index, tree, &expanded_set);
        let mut segment = Vec::with_capacity(flat.len() + 1);
        match edit {
            Some(edit_state)
                if edit_state.target_id.is_none() && edit_state.root == *root_index =>
            {
                // New entry: insert the edit row right AFTER its parent row
                // (the first child position). Inserting before the parent
                // would place the edit row above the root when the parent is
                // the root.
                let parent_index = flat
                    .iter()
                    .position(|entry| Some(entry.id) == edit_state.parent_id);
                let mut inserted = false;
                for (index, entry) in flat.into_iter().enumerate() {
                    segment.push(ExplorerRow::Entry(entry));
                    if Some(index) == parent_index {
                        segment.push(ExplorerRow::Edit { root: *root_index });
                        inserted = true;
                    }
                }
                if !inserted {
                    // Fallback: never in front of the root row (index 0).
                    segment.insert(1, ExplorerRow::Edit { root: *root_index });
                }
            }
            Some(edit_state) if edit_state.root == *root_index => {
                // Rename: replace the target row.
                for entry in flat {
                    if Some(entry.id) == edit_state.target_id {
                        segment.push(ExplorerRow::Edit { root: *root_index });
                    } else {
                        segment.push(ExplorerRow::Entry(entry));
                    }
                }
            }
            _ => segment.extend(flat.into_iter().map(ExplorerRow::Entry)),
        }
        rows.extend(segment);
    }
    rows
}

fn flatten_children(
    root_index: usize,
    nodes: &[ExplorerFileNode],
    parent_id: Option<ExplorerEntryId>,
    depth: usize,
    expanded: &BTreeSet<ExplorerEntryId>,
    out: &mut Vec<VisibleExplorerEntry>,
) {
    for node in nodes {
        let is_expanded = expanded.contains(&node.id);
        out.push(VisibleExplorerEntry {
            root: root_index,
            id: node.id,
            parent_id,
            path: node.path.clone(),
            label: node.label.clone(),
            depth,
            kind: node.kind,
            is_expanded,
            has_children: !node.children.is_empty(),
        });
        if is_expanded && !node.children.is_empty() {
            flatten_children(root_index, &node.children, Some(node.id), depth + 1, expanded, out);
        }
    }
}

/// Depth-first lookup of a node by absolute path in the scanned tree.
pub fn find_explorer_node<'a>(
    node: &'a ExplorerFileNode,
    path: &Path,
) -> Option<&'a ExplorerFileNode> {
    if node.path == path {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_explorer_node(child, path) {
            return Some(found);
        }
    }
    None
}

/// Whether the scanned tree contains a node with the given stable id.
pub fn explorer_tree_contains_id(node: &ExplorerFileNode, id: ExplorerEntryId) -> bool {
    if node.id == id {
        return true;
    }
    node.children
        .iter()
        .any(|child| explorer_tree_contains_id(child, id))
}

/// Depth-first lookup of a node by stable id in the scanned tree.
pub fn find_explorer_node_by_id<'a>(
    node: &'a ExplorerFileNode,
    id: ExplorerEntryId,
) -> Option<&'a ExplorerFileNode> {
    if node.id == id {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_explorer_node_by_id(child, id))
}

/// Collect `node` and every descendant directory id into `out` (used by
/// expand-all / collapse-all for an entry).
pub fn collect_descendant_dir_ids(
    node: &ExplorerFileNode,
    out: &mut std::collections::BTreeSet<ExplorerEntryId>,
) {
    out.insert(node.id);
    for child in &node.children {
        if child.kind == ExplorerEntryKind::Directory {
            collect_descendant_dir_ids(child, out);
        }
    }
}
