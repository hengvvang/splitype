//! Explorer file-tree state and model.
//!
//! Pure data-driven state: worktree snapshots, the flat visible-row model,
//! selection, expansion, drag-and-drop state, and the inline filename
//! editor. Each explorer panel instance owns one [`ExplorerState`] entity
//! (one per `ExplorerPanelView`), so split and multi-window panels never
//! share tree state. The VIEW (interactions, rendering) lives in the
//! crate's sibling modules and depends on this state one-way.
//!
//! The editor family never imports this module, and vice versa.
//!
//! The module owns:
//! - [`WorktreeId`] & [`ExplorerEntryId`] — strongly-typed stable identifiers.
//! - [`SelectedEntry`] — the composite selection key `(worktree_id, entry_id)`.
//! - [`VisibleExplorerEntry`] — flat view row derived directly from `WorktreeSnapshot`.
//! - [`ExplorerState`] — file-tree interaction and view-model state.

pub mod undo;
pub mod utils;
pub mod worktree;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use gpui::{
    AnyWindowHandle, AppContext, Bounds, Entity, FocusHandle, Pixels, Task,
    UniformListScrollHandle, WeakEntity,
};

use crate::state::undo::ExplorerUndoHistory;
use crate::state::worktree::{Worktree, WorktreeEntryKind, WorktreeSnapshot};

pub use crate::state::worktree::{ExplorerEntryId, WorktreeEvent, WorktreeId};

/// Explorer row right-click menu: a window-level overlay rendered by the
/// Shell (it must float over every area at window coordinates).
#[derive(Clone)]
pub struct ExplorerFileMenuState {
    pub position: gpui::Point<Pixels>,
    pub path: PathBuf,
    pub is_dir: bool,
}

// ── Icons & constants ───────────────────────────────────────────────────

pub const FOLDER_ICON: &str = "icons/explorer/worktree/folder.svg";
pub const MARKDOWN_ICON: &str = "icons/explorer/worktree/markdown.svg";
pub const FILE_ICON: &str = "icons/explorer/worktree/file_type_default.svg";
pub const PDF_ICON: &str = "icons/explorer/worktree/file_type_pdf.svg";
pub const CODE_ICON: &str = "icons/explorer/worktree/file_type_code.svg";
pub const MUSIC_ICON: &str = "icons/explorer/worktree/file_type_music.svg";
pub const IMAGE_ICON: &str = "icons/explorer/worktree/file_type_image.svg";
pub const TXT_ICON: &str = "icons/explorer/worktree/file_type_txt.svg";
pub const EXPLORER_NODE_HEIGHT: f32 = 28.0;
pub const EXPLORER_NODE_INDENT: f32 = 14.0;

/// Map a lower-cased file extension to its explorer type icon.
pub fn file_type_icon(ext: &str) -> &'static str {
    match ext {
        "md" | "markdown" => MARKDOWN_ICON,
        "pdf" => PDF_ICON,
        "rs" | "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "py" | "js" | "ts" | "tsx" | "jsx"
        | "java" | "go" | "rb" | "php" | "swift" | "kt" | "cs" | "sh" | "bash" | "toml"
        | "json" | "yaml" | "yml" | "xml" | "html" | "css" | "sql" | "lua" | "r" | "scala"
        | "zig" | "dart" | "m" | "mm" => CODE_ICON,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" | "opus" | "mid" | "midi" => {
            MUSIC_ICON
        }
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" | "tif"
        | "avif" | "heic" => IMAGE_ICON,
        "txt" | "text" | "log" | "ini" | "conf" | "cfg" => TXT_ICON,
        _ => FILE_ICON,
    }
}

// ── File-tree node types ────────────────────────────────────────────────

/// What kind of filesystem entry a file-tree row represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerEntryKind {
    Directory,
    MarkdownFile,
    File,
}

impl ExplorerEntryKind {
    #[inline]
    pub fn is_dir(&self) -> bool {
        matches!(self, Self::Directory)
    }

    #[inline]
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }
}

impl From<WorktreeEntryKind> for ExplorerEntryKind {
    fn from(kind: WorktreeEntryKind) -> Self {
        match kind {
            WorktreeEntryKind::Directory => Self::Directory,
            WorktreeEntryKind::File => Self::File,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldedAncestors {
    pub current_ancestor_depth: usize,
    pub ancestors: Vec<ExplorerEntryId>,
}

impl FoldedAncestors {
    pub fn max_ancestor_depth(&self) -> usize {
        self.ancestors.len()
    }

    pub fn active_ancestor(&self) -> Option<ExplorerEntryId> {
        if self.current_ancestor_depth == 0 {
            return None;
        }
        self.ancestors.get(self.current_ancestor_depth).copied()
    }

    pub fn active_index(&self) -> usize {
        self.max_ancestor_depth()
            .saturating_sub(1)
            .saturating_sub(self.current_ancestor_depth)
    }

    pub fn set_active_index(&mut self, index: usize) -> bool {
        let new_depth = self
            .max_ancestor_depth()
            .saturating_sub(1)
            .saturating_sub(index);
        if self.current_ancestor_depth != new_depth {
            self.current_ancestor_depth = new_depth;
            true
        } else {
            false
        }
    }
}

/// One visible row in the virtualized file-tree list. Derived directly
/// from [`WorktreeSnapshot`] in linear time (mirrors Zed's `visible_entries`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleExplorerEntry {
    pub worktree_id: WorktreeId,
    pub id: ExplorerEntryId,
    pub parent_id: Option<ExplorerEntryId>,
    pub path: PathBuf,
    pub label: String,
    pub depth: usize,
    pub kind: ExplorerEntryKind,
    pub is_expanded: bool,
    pub has_children: bool,
    pub ancestors: Option<FoldedAncestors>,
}

// ── Selection ───────────────────────────────────────────────────────────

/// Strongly-typed composite selection key (mirrors Zed's `SelectedEntry`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SelectedEntry {
    pub worktree_id: WorktreeId,
    pub entry_id: ExplorerEntryId,
}

/// Validation feedback for the inline filename editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerValidation {
    Warning(String),
    Error(String),
}

/// Text buffer of the inline filename editor.
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
#[derive(Clone, Debug)]
pub struct ExplorerEditState {
    pub worktree_id: WorktreeId,
    pub parent_id: Option<ExplorerEntryId>,
    pub target_id: Option<ExplorerEntryId>,
    pub is_dir: bool,
    pub depth: usize,
    pub path: PathBuf,
    pub validation: Option<ExplorerValidation>,
    pub filename: ExplorerFilenameEditor,
    pub previously_selected: Option<SelectedEntry>,
    pub processing: bool,
    /// IME host entity registered as the filename input's window handler.
    pub ime_host: Option<Entity<crate::filename_editor::ExplorerFilenameImeHost>>,
}

impl ExplorerEditState {
    #[inline]
    pub fn is_new_entry(&self) -> bool {
        self.target_id.is_none()
    }
}

/// One row of the virtualized file-tree list: either a visible entry or the
/// inline edit row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerRow {
    Entry(VisibleExplorerEntry),
    Edit { worktree_id: WorktreeId },
}

/// In-panel clipboard for cut/copy/paste of file-tree entries (mirrors
/// Zed's `ClipboardEntry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerClipboard {
    Copied(BTreeSet<SelectedEntry>),
    Cut(BTreeSet<SelectedEntry>),
}

impl ExplorerClipboard {
    #[inline]
    pub fn is_cut(&self) -> bool {
        matches!(self, Self::Cut(_))
    }

    #[inline]
    pub fn items(&self) -> &BTreeSet<SelectedEntry> {
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

/// Payload for dragging file-tree entries within the panel (mirrors
/// Zed's `DraggedSelection`).
#[derive(Clone, Debug)]
pub struct DraggedExplorerSelection {
    pub selections: Vec<SelectedEntry>,
}

impl DraggedExplorerSelection {
    #[inline]
    pub fn active(&self) -> Option<&SelectedEntry> {
        self.selections.first()
    }
}

/// The current drag-and-drop target of the panel (mirrors Zed's `DragTarget`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragExplorerTarget {
    Entry {
        entry_id: ExplorerEntryId,
        highlight_entry_id: ExplorerEntryId,
    },
    Background,
}

impl DragExplorerTarget {
    #[inline]
    pub fn entry_id(&self) -> Option<ExplorerEntryId> {
        match self {
            Self::Entry { entry_id, .. } => Some(*entry_id),
            Self::Background => None,
        }
    }

    #[inline]
    pub fn highlight_entry_id(&self) -> Option<ExplorerEntryId> {
        match self {
            Self::Entry {
                highlight_entry_id, ..
            } => Some(*highlight_entry_id),
            Self::Background => None,
        }
    }
}

// ── Explorer State ─────────────────────────────────────────────────────

/// Top-level explorer file-tree state.
pub struct ExplorerState {
    pub tree_visible: bool,
    /// Worktree entities in display order (mirrors Zed's `visible_worktrees`).
    pub worktrees: Vec<Entity<Worktree>>,
    /// Immutable worktree snapshots kept in sync on scan events.
    pub snapshots: Vec<Arc<WorktreeSnapshot>>,
    /// Shared stable-id allocator across all worktrees.
    pub next_entry_id: Arc<AtomicU64>,
    /// Expanded directory ids per worktree (Zed's `expanded_dir_ids`).
    pub expanded: HashMap<WorktreeId, BTreeSet<ExplorerEntryId>>,
    /// Unfolded directory ids (explicitly unfolded compact directories, mirrors Zed).
    pub unfolded_dir_ids: HashSet<ExplorerEntryId>,
    /// Maps from leaf entry id to its compact folded ancestors (Zed's `ancestors`).
    pub ancestors: HashMap<ExplorerEntryId, FoldedAncestors>,
    pub file_error: Option<String>,
    /// Flat visible rows — the virtualized list's data source.
    pub entries: Vec<ExplorerRow>,
    /// Active selection (Zed's `selection`).
    pub selected: Option<SelectedEntry>,
    /// Multi-select marks (Zed's `marked_entries`).
    pub marked: BTreeSet<SelectedEntry>,
    /// In-panel cut/copy clipboard.
    pub clipboard: Option<ExplorerClipboard>,
    /// Undo/redo stacks for file operations.
    pub undo_history: ExplorerUndoHistory,
    /// Current drag target while a drag is in flight.
    pub drag_target: Option<DragExplorerTarget>,
    /// Delayed task that expands a hovered directory during a drag.
    pub hover_expand_task: Option<Task<()>>,
    /// Continuous scroll task while a drag hovers the list edges.
    pub hover_scroll_task: Option<Task<()>>,
    pub hover_scroll_generation: u64,
    pub previous_drag_position: Option<gpui::Point<Pixels>>,
    /// Path (and worktree id) to select once the next scan completes.
    pub pending_select: Option<(WorktreeId, PathBuf)>,
    /// Copy-collision rename to start once the next scan makes the entry visible.
    pub pending_rename: Option<(AnyWindowHandle, PathBuf)>,
    /// Active inline create/rename state.
    pub edit: Option<ExplorerEditState>,
    /// Scroll handle bound to the virtualized file-tree list.
    pub scroll_handle: UniformListScrollHandle,
    pub rendered_rows: usize,
    pub recent_folders_cache: Vec<PathBuf>,
    pub recent_files_cache: Vec<PathBuf>,
    /// Open row right-click menu (window-level overlay state).
    pub file_menu: Option<ExplorerFileMenuState>,
    /// Bottom bar three-dots action menu open state.
    pub bottombar_menu_open: bool,
    /// Path of the file open in the active editor tab (pushed by the shell
    /// every frame; used to keep the tree selection in sync).
    pub active_file: Option<PathBuf>,
    /// Weak handle to this state's own entity, captured at construction so
    /// event handlers and background tasks can re-enter the panel state.
    pub self_weak: WeakEntity<Self>,
}

impl ExplorerState {
    /// Construct a per-panel explorer state entity. Each panel instance
    /// owns its own state, so split/multi-window panels never interfere.
    pub fn entity(cx: &mut gpui::App) -> Entity<Self> {
        cx.new(|cx| Self {
            self_weak: cx.weak_entity(),
            ..Default::default()
        })
    }
}

impl Default for ExplorerState {
    fn default() -> Self {
        let mut state = Self {
            tree_visible: false,
            worktrees: Vec::new(),
            snapshots: Vec::new(),
            next_entry_id: Arc::new(AtomicU64::new(1)),
            expanded: HashMap::new(),
            unfolded_dir_ids: HashSet::new(),
            ancestors: HashMap::new(),
            file_error: None,
            entries: Vec::new(),
            selected: None,
            marked: BTreeSet::new(),
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
            recent_folders_cache: Vec::new(),
            recent_files_cache: Vec::new(),
            file_menu: None,
            bottombar_menu_open: false,
            active_file: None,
            self_weak: WeakEntity::new_invalid(),
        };
        state.refresh_recent_cache();
        state
    }
}

impl ExplorerState {
    /// Refreshes the cached recent folders and files for empty state rendering.
    pub fn refresh_recent_cache(&mut self) {
        self.recent_folders_cache = config::recent::read_recent_folders()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_dir())
            .take(5)
            .collect();
        self.recent_files_cache = config::recent::read_recent_files()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_file())
            .take(5)
            .collect();
    }
}

// ── Direct Snapshot $\rightarrow$ Visible Rows Derivation ──────────────────────

/// Derive the flat visible row list directly from each [`WorktreeSnapshot`]
/// in linear time $O(N)$ with zero intermediate recursive tree allocations.
pub fn build_explorer_rows(
    snapshots: &[Arc<WorktreeSnapshot>],
    expanded: &HashMap<WorktreeId, BTreeSet<ExplorerEntryId>>,
    edit: Option<&ExplorerEditState>,
) -> Vec<ExplorerRow> {
    let mut rows = Vec::new();

    for snapshot in snapshots {
        let worktree_id = snapshot.id();
        let expanded_set = expanded.get(&worktree_id);
        let Some(root_entry) = snapshot.root_entry() else {
            continue;
        };
        let root_path = &root_entry.path;
        let root_is_expanded = expanded_set.is_none_or(|set| set.contains(&root_entry.id));

        let mut flat_entries: Vec<VisibleExplorerEntry> =
            Vec::with_capacity(snapshot.entries_by_path.len());
        let mut collapsed_prefix: Option<PathBuf> = None;

        for entry in snapshot.entries_by_path.values() {
            let path = &entry.path;

            // If we are currently skipping a collapsed directory subtree:
            if let Some(prefix) = &collapsed_prefix {
                if path.starts_with(prefix) && path != prefix {
                    continue;
                } else {
                    collapsed_prefix = None;
                }
            }

            // Calculate depth relative to worktree root
            let (depth, label) = if path == root_path {
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                (0, label)
            } else if let Ok(rel) = path.strip_prefix(root_path) {
                let depth = rel.components().count();
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                (depth, label)
            } else {
                continue;
            };

            let is_dir = entry.kind == WorktreeEntryKind::Directory;
            let is_expanded = if path == root_path {
                root_is_expanded
            } else if is_dir {
                expanded_set.is_some_and(|set| set.contains(&entry.id))
            } else {
                false
            };

            // Check if this directory has children in the snapshot
            let has_children = is_dir && snapshot.child_entries(path).next().is_some();

            // Determine entry kind
            let kind = if is_dir {
                ExplorerEntryKind::Directory
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                ExplorerEntryKind::MarkdownFile
            } else {
                ExplorerEntryKind::File
            };

            let parent_id = path
                .parent()
                .and_then(|p| snapshot.id_for_path.get(p).copied());

            flat_entries.push(VisibleExplorerEntry {
                worktree_id,
                id: entry.id,
                parent_id,
                path: path.clone(),
                label,
                depth,
                kind,
                is_expanded,
                has_children,
                ancestors: None,
            });

            // If this directory is collapsed, skip all its children
            if is_dir && !is_expanded {
                collapsed_prefix = Some(path.clone());
            }
        }

        // Splice inline edit row if active for this worktree
        let mut segment = Vec::with_capacity(flat_entries.len() + 1);
        match edit {
            Some(edit_state)
                if edit_state.is_new_entry() && edit_state.worktree_id == worktree_id =>
            {
                let parent_index = flat_entries
                    .iter()
                    .position(|entry| Some(entry.id) == edit_state.parent_id);
                let mut inserted = false;
                for (index, entry) in flat_entries.into_iter().enumerate() {
                    segment.push(ExplorerRow::Entry(entry));
                    if Some(index) == parent_index {
                        segment.push(ExplorerRow::Edit { worktree_id });
                        inserted = true;
                    }
                }
                if !inserted {
                    let insert_pos = 1.min(segment.len());
                    segment.insert(insert_pos, ExplorerRow::Edit { worktree_id });
                }
            }
            Some(edit_state) if edit_state.worktree_id == worktree_id => {
                for entry in flat_entries {
                    if Some(entry.id) == edit_state.target_id {
                        segment.push(ExplorerRow::Edit { worktree_id });
                    } else {
                        segment.push(ExplorerRow::Entry(entry));
                    }
                }
            }
            _ => segment.extend(flat_entries.into_iter().map(ExplorerRow::Entry)),
        }
        rows.extend(segment);
    }

    rows
}
