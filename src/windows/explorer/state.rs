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

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::Result;
use gpui::{Bounds, FocusHandle, Pixels, Task, UniformListScrollHandle};

// ── Icons & constants ───────────────────────────────────────────────────

pub const FOLDER_ICON: &str = "icon/explorer/folder.svg";
pub const MARKDOWN_ICON: &str = "icon/explorer/markdown.svg";
pub const FILE_ICON: &str = "icon/explorer/file.svg";
pub const EXPLORER_NODE_HEIGHT: f32 = 28.0;
pub const EXPLORER_NODE_INDENT: f32 = 14.0;

// ── Stable entry ids ────────────────────────────────────────────────────

/// Stable id for a file-tree entry, derived from the entry's absolute path.
///
/// Re-scanning the same directory yields the same ids, so expansion and
/// selection state survives refreshes. Collisions are astronomically
/// unlikely (64-bit hash of the path).
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerSelection {
    File(ExplorerEntryId),
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
/// inline edit row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerRow {
    Entry(VisibleExplorerEntry),
    Edit,
}

/// One reversible file-tree operation recorded in the undo history.
///
/// Mirrors Zed's `Change`/`Operation` pair with a simpler scheme: the
/// history stores the forward operation; undoing executes its inverse and
/// pushes the same record onto the redo stack (so redo simply re-executes
/// it). Only reversible operations are recorded — permanent deletes are not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerChange {
    Created(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
    Moved { from: PathBuf, to: PathBuf },
    Copied { source: PathBuf, dest: PathBuf },
}

/// Undo/redo stacks for explorer file operations.
#[derive(Clone, Debug, Default)]
pub struct ExplorerUndoHistory {
    pub undo_stack: Vec<ExplorerChange>,
    pub redo_stack: Vec<ExplorerChange>,
}

impl ExplorerUndoHistory {
    pub fn record(&mut self, change: ExplorerChange) {
        self.undo_stack.push(change);
        // A fresh edit invalidates any forward history.
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

/// In-panel clipboard for cut/copy/paste of file-tree entries (mirrors
/// Zed's `ClipboardEntry`). The system clipboard additionally receives the
/// absolute paths as text for use outside the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerClipboard {
    Copied(Vec<ExplorerSelection>),
    Cut(Vec<ExplorerSelection>),
}
    /// Payload for dragging file-tree entries within the panel (Zed's
/// `DraggedSelection`).
#[derive(Clone, Debug)]
pub struct DraggedExplorerSelection {
    pub selections: Vec<ExplorerSelection>,
}

/// The current drag-and-drop target of the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragExplorerTarget {
    /// Highlight this entry (and its descendants).
    Entry(ExplorerEntryId),
    /// Dropping on the empty area targets the explorer root.
    Background,
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
    pub root: Option<PathBuf>,
    /// The root that the current `file_tree` was scanned from.
    pub scanned_root: Option<PathBuf>,
    /// Full scanned tree (background-thread product).
    pub file_tree: Option<ExplorerFileNode>,
    pub file_error: Option<String>,
    pub outline_tree: Vec<ExplorerNode>,
    pub outline_source: Option<String>,
    /// Expanded directory ids, kept sorted for binary search (mirrors Zed's
    /// `expanded_dir_ids`).
    pub expanded: BTreeSet<ExplorerEntryId>,
    /// Expanded outline node ids (kept separate: outline ids are strings).
    pub expanded_outline: HashSet<String>,
    /// Flat visible rows — the virtualized list's data source (includes the
    /// inline edit row while an edit is active).
    pub entries: Vec<ExplorerRow>,
    /// Selection, keyed by stable id.
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
    /// Set when the on-disk tree may have changed and must be re-scanned.
    pub needs_rescan: bool,
    /// Path to select once the next scan completes (used by inline create).
    pub pending_select: Option<PathBuf>,
    /// Active inline create/rename state.
    pub edit: Option<ExplorerEditState>,
    /// Scroll handle bound to the virtualized file-tree list.
    pub scroll_handle: UniformListScrollHandle,
    /// Number of rows rendered in the last frame (used for page scrolling).
    pub rendered_rows: usize,
    /// Handle of the in-flight background scan; replacing it cancels the
    /// previous scan.
    pub scan_task: Option<Task<()>>,
    /// Root currently watched for filesystem changes.
    pub watched_root: Option<PathBuf>,
    /// The filesystem watcher task (alive while the explorer has a root).
    pub fs_watch_task: Option<Task<()>>,
    /// Debounced refresh task triggered by filesystem events.
    pub fs_refresh_task: Option<Task<()>>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self {
            is_open: false,
            root: None,
            scanned_root: None,
            file_tree: None,
            file_error: None,
            outline_tree: Vec::new(),
            outline_source: None,
            expanded: BTreeSet::new(),
            expanded_outline: HashSet::new(),
            entries: Vec::new(),
            selected: None,
            marked: Vec::new(),
            clipboard: None,
            undo_history: ExplorerUndoHistory::default(),
            drag_target: None,
            hover_expand_task: None,
            needs_rescan: false,
            pending_select: None,
            edit: None,
            scroll_handle: UniformListScrollHandle::new(),
            rendered_rows: 0,
            scan_task: None,
            watched_root: None,
            fs_watch_task: None,
            fs_refresh_task: None,
        }
    }
}

// ── Filesystem helpers ──────────────────────────────────────────────────

/// Returns `true` when `path` has a `.md` extension (case-insensitive).
pub fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("md"))
}

/// Returns `true` for directory names that the explorer scanner skips.
pub fn is_ignored_explorer_entry(name: &str) -> bool {
    name == "node_modules"
        || name == "target"
        || name == "dist"
        || name == "build"
        || name == ".git"
}

/// Explorer tree sort mode used by the scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerSortMode {
    DirectoriesFirst,
    FilesFirst,
    Mixed,
}

/// Explorer tree sort order used by the scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerSortOrder {
    Ascending,
    Descending,
}

/// Scan options derived from the persisted explorer settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExplorerScanOptions {
    pub hide_hidden: bool,
    pub sort_mode: ExplorerSortMode,
    pub sort_order: ExplorerSortOrder,
}

impl Default for ExplorerScanOptions {
    fn default() -> Self {
        Self {
            hide_hidden: false,
            sort_mode: ExplorerSortMode::DirectoriesFirst,
            sort_order: ExplorerSortOrder::Ascending,
        }
    }
}

/// Recursively scan a directory into an [`ExplorerFileNode`] tree.
///
/// Directories sort before files (unless `sort_mode` says otherwise); within
/// each group entries are sorted case-insensitively by label. Designed to
/// run on a background thread: it performs blocking filesystem I/O.
pub fn scan_explorer_dir(
    path: &Path,
    options: &ExplorerScanOptions,
) -> Result<ExplorerFileNode> {
    let mut children = Vec::new();
    let read_dir = fs::read_dir(path)
        .map_err(|err| anyhow::anyhow!("failed to read '{}': {err}", path.display()))?;

    for entry in read_dir {
        let Ok(entry) = entry else {
            continue;
        };
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        if is_ignored_explorer_entry(&name) {
            continue;
        }
        if options.hide_hidden && name.starts_with('.') {
            continue;
        }

        if entry_path.is_dir() {
            let dir_children = scan_explorer_dir(&entry_path, options)
                .map(|node| node.children)
                .unwrap_or_default();
            children.push(ExplorerFileNode {
                id: ExplorerEntryId::for_path(&entry_path),
                path: entry_path,
                label: name,
                kind: ExplorerEntryKind::Directory,
                children: dir_children,
            });
        } else if entry_path.is_file() {
            let kind = if is_markdown_file(&entry_path) {
                ExplorerEntryKind::MarkdownFile
            } else {
                ExplorerEntryKind::File
            };
            children.push(ExplorerFileNode {
                id: ExplorerEntryId::for_path(&entry_path),
                path: entry_path,
                label: name,
                kind,
                children: Vec::new(),
            });
        }
    }

    children.sort_by(|left, right| {
        let left_dir = left.kind == ExplorerEntryKind::Directory;
        let right_dir = right.kind == ExplorerEntryKind::Directory;
        let dir_cmp = match options.sort_mode {
            ExplorerSortMode::DirectoriesFirst => right_dir.cmp(&left_dir),
            ExplorerSortMode::FilesFirst => left_dir.cmp(&right_dir),
            ExplorerSortMode::Mixed => std::cmp::Ordering::Equal,
        };
        let name_cmp = left.label.to_lowercase().cmp(&right.label.to_lowercase());
        let cmp = dir_cmp.then(name_cmp);
        if options.sort_order == ExplorerSortOrder::Descending {
            cmp.reverse()
        } else {
            cmp
        }
    });

    Ok(ExplorerFileNode {
        id: ExplorerEntryId::for_path(path),
        path: path.to_path_buf(),
        label: file_label(path),
        kind: ExplorerEntryKind::Directory,
        children,
    })
}

/// Human-readable label for a path (its final component).
pub fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Stable numeric hash of an id, for use as a DOM element id suffix.
pub fn stable_node_hash(id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

// ── Flat visible list derivation ────────────────────────────────────────

/// Flatten the scanned tree into the visible row list. The root itself is
/// the first row (depth 0, foldable — mirroring Zed's default `hide_root:
/// false` rendering); its children are visible only while the root is
/// expanded. A directory's children are included only when its id is in
/// `expanded` (traversal pruning, mirroring Zed's `advance_to_sibling`).
pub fn flatten_file_tree(
    root: &ExplorerFileNode,
    expanded: &BTreeSet<ExplorerEntryId>,
) -> Vec<VisibleExplorerEntry> {
    let mut out = Vec::new();
    out.push(VisibleExplorerEntry {
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
        flatten_children(&root.children, Some(root.id), 1, expanded, &mut out);
    }
    out
}

/// Derive the flat row list from the scanned tree plus expansion set, then
/// splice the inline edit row into its position (create: after its parent
/// row, or at index 0 for a root-level parent; rename: replacing the target
/// row).
pub fn build_explorer_rows(
    root: &ExplorerFileNode,
    expanded: &BTreeSet<ExplorerEntryId>,
    edit: Option<&ExplorerEditState>,
) -> Vec<ExplorerRow> {
    let flat = flatten_file_tree(root, expanded);
    let mut rows = Vec::with_capacity(flat.len() + 1);
    match edit {
        Some(edit_state) if edit_state.target_id.is_none() => {
            // New entry: insert the edit row right after its parent row.
            let parent_index = flat
                .iter()
                .position(|entry| Some(entry.id) == edit_state.parent_id);
            let mut inserted = false;
            for (index, entry) in flat.into_iter().enumerate() {
                if Some(index) == parent_index {
                    rows.push(ExplorerRow::Edit);
                    inserted = true;
                }
                rows.push(ExplorerRow::Entry(entry));
            }
            if !inserted {
                // Fallback: never in front of the root row (index 0).
                rows.insert(1, ExplorerRow::Edit);
            }
        }
        Some(edit_state) => {
            // Rename: replace the target row.
            for entry in flat {
                if Some(entry.id) == edit_state.target_id {
                    rows.push(ExplorerRow::Edit);
                } else {
                    rows.push(ExplorerRow::Entry(entry));
                }
            }
        }
        None => rows = flat.into_iter().map(ExplorerRow::Entry).collect(),
    }
    rows
}

fn flatten_children(
    nodes: &[ExplorerFileNode],
    parent_id: Option<ExplorerEntryId>,
    depth: usize,
    expanded: &BTreeSet<ExplorerEntryId>,
    out: &mut Vec<VisibleExplorerEntry>,
) {
    for node in nodes {
        let is_expanded = expanded.contains(&node.id);
        out.push(VisibleExplorerEntry {
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
            flatten_children(&node.children, Some(node.id), depth + 1, expanded, out);
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

/// Collect every directory id in the tree (used to prune stale expansion ids
/// after a rescan).
pub fn collect_directory_ids(node: &ExplorerFileNode, out: &mut HashSet<ExplorerEntryId>) {
    if node.kind == ExplorerEntryKind::Directory {
        out.insert(node.id);
    }
    for child in &node.children {
        collect_directory_ids(child, out);
    }
}
