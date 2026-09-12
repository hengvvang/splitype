//! Explorer file-tree state and model.
//!
//! Pure data-driven state: worktree snapshots, the flat visible-row model,
//! selection, expansion, drag-and-drop state, and the inline filename
//! editor. Each explorer panel instance owns one [`ExplorerState`] entity
//! (one per `ExplorerPanelView`) holding the panel's VIEW state, while the
//! scanned trees themselves are process-level shared resources in
//! [`WorktreeStore`] — split and multi-window panels share one tree per
//! folder root and keep independent expansion/selection. The VIEW
//! (interactions, rendering) lives in the crate's sibling modules and
//! depends on this state one-way.
//!
//! The editor family never imports this module, and vice versa.
//!
//! The module owns:
//! - [`WorktreeId`] & [`ExplorerEntryId`] — strongly-typed stable identifiers.
//! - [`SelectedEntry`] — the composite selection key `(worktree_id, entry_id)`.
//! - [`VisibleExplorerEntry`] — flat view row derived directly from `WorktreeSnapshot`.
//! - [`ExplorerState`] — file-tree interaction and view-model state.
//! - [`WorktreeStore`] — the process-global registry of shared scanned trees.

pub mod ignore;
pub mod store;
pub mod undo;
pub mod utils;
pub mod worktree;

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    AnyWindowHandle, AppContext, Entity, FocusHandle, Pixels, Subscription, Task,
    UniformListScrollHandle, WeakEntity,
};

use crate::settings::{ExplorerSettings, ExplorerSortMode, ExplorerSortOrder};
use crate::state::undo::ExplorerUndoHistory;
use crate::state::worktree::{Worktree, WorktreeEntry, WorktreeEntryKind, WorktreeSnapshot};

pub use crate::state::store::WorktreeStore;
pub use crate::state::worktree::{ExplorerEntryId, NEW_ENTRY_ID, WorktreeEvent, WorktreeId};

/// Explorer row right-click menu: a window-level overlay rendered by the
/// Shell (it must float over every area at window coordinates).
#[derive(Clone)]
pub struct ExplorerFileMenuState {
    pub position: gpui::Point<Pixels>,
    pub path: PathBuf,
    pub is_dir: bool,
}

// ── Icons & constants ───────────────────────────────────────────────────

pub const FOLDER_ICON: &str = "plugin://splitype.explorer/worktree/folder.svg";
pub const MARKDOWN_ICON: &str = "plugin://splitype.explorer/worktree/markdown.svg";
pub const FILE_ICON: &str = "plugin://splitype.explorer/worktree/file_type_default.svg";
pub const PDF_ICON: &str = "plugin://splitype.explorer/worktree/file_type_pdf.svg";
pub const CODE_ICON: &str = "plugin://splitype.explorer/worktree/file_type_code.svg";
pub const MUSIC_ICON: &str = "plugin://splitype.explorer/worktree/file_type_music.svg";
pub const IMAGE_ICON: &str = "plugin://splitype.explorer/worktree/file_type_image.svg";
pub const TXT_ICON: &str = "plugin://splitype.explorer/worktree/file_type_txt.svg";
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
}

impl From<WorktreeEntryKind> for ExplorerEntryKind {
    fn from(kind: WorktreeEntryKind) -> Self {
        match kind {
            WorktreeEntryKind::Directory => Self::Directory,
            WorktreeEntryKind::File => Self::File,
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
    pub is_ignored: bool,
    pub folded_ancestors: Vec<ExplorerEntryId>,
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

pub type ExplorerFilenameEditor = crate::filename_editor::FilenameEditor;

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
    pub previously_selected: Option<SelectedEntry>,
    pub processing: bool,
    pub processing_filename: Option<String>,
    /// Folder that was temporarily unfolded for this edit and should be re-folded on cancel.
    pub temporarily_unfolded: Option<(WorktreeId, ExplorerEntryId)>,
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
    /// Sorting mode (directories first, files first, mixed).
    pub sort_mode: ExplorerSortMode,
    /// Sorting order (ascending, descending).
    pub sort_order: ExplorerSortOrder,
    /// Auto-collapse single-child directory chains into a single row.
    pub auto_fold_dirs: bool,
    /// Hide entries matching .gitignore rules.
    pub hide_gitignore: bool,
    /// Automatically reveal and select the active editor file in the file tree.
    pub auto_reveal: bool,
    /// Selection anchor index in the visible row list for range selection (Shift+Click / Shift+Up/Down).
    pub selection_anchor: Option<usize>,
    /// Shared worktree entities in this panel's display order. The trees
    /// themselves are process-global (see [`WorktreeStore`]); this list is
    /// the panel's view of which roots are visible.
    pub worktrees: Vec<Entity<Worktree>>,
    /// Immutable worktree snapshots kept in sync on scan events (render cache).
    pub snapshots: Vec<Arc<WorktreeSnapshot>>,
    /// Expanded directory ids per worktree.
    pub expanded: HashMap<WorktreeId, BTreeSet<ExplorerEntryId>>,
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
    /// Worktree scan-event subscriptions, keyed by tree id. Dropping a
    /// subscription (or the panel) unsubscribes; the store drops the shared
    /// tree when the last view releases it.
    pub subscriptions: HashMap<WorktreeId, Subscription>,
    /// Keyboard focus handle for the explorer panel.
    pub focus_handle: Option<FocusHandle>,
    /// Persistent single-line filename editor entity (mirrors Zed's `filename_editor: Entity<Editor>`).
    pub filename_editor: Option<Entity<crate::filename_editor::FilenameEditor>>,
}

impl ExplorerState {
    /// Construct a per-panel explorer state entity. Each panel instance
    /// owns its own state, so split/multi-window panels never interfere.
    pub fn entity(cx: &mut gpui::App) -> Entity<Self> {
        let settings = config::settings::PluginSettings::<ExplorerSettings>::get(cx);
        let focus_handle = cx.focus_handle();
        let filename_editor = cx.new(|cx| crate::filename_editor::FilenameEditor::new(cx));
        let filename_editor_sub = filename_editor.clone();

        cx.new(|cx| {
            let state_weak = cx.weak_entity();
            cx.subscribe(
                &filename_editor_sub,
                move |state: &mut ExplorerState,
                      _,
                      event: &crate::filename_editor::FilenameEditorEvent,
                      cx| {
                    match event {
                        crate::filename_editor::FilenameEditorEvent::BufferEdited => {
                            state.populate_explorer_validation(cx);
                            state.autoscroll_explorer_edit(None, cx);
                            cx.refresh_windows();
                        }
                        crate::filename_editor::FilenameEditorEvent::SelectionsChanged => {
                            state.autoscroll_explorer_edit(None, cx);
                            cx.refresh_windows();
                        }
                        crate::filename_editor::FilenameEditorEvent::Blurred => {
                            if state.edit.as_ref().is_some_and(|e| e.processing) {
                                return;
                            }
                            if state.edit.is_some() && !state.confirm_explorer_edit(None, cx) {
                                state.discard_explorer_edit(None, cx);
                            }
                        }
                        crate::filename_editor::FilenameEditorEvent::Confirmed => {
                            state.confirm_explorer_edit(None, cx);
                        }
                        crate::filename_editor::FilenameEditorEvent::Cancelled => {
                            state.discard_explorer_edit(None, cx);
                        }
                    }
                },
            )
            .detach();

            Self {
                self_weak: state_weak,
                sort_mode: settings.sort_mode,
                sort_order: settings.sort_order,
                auto_fold_dirs: settings.auto_fold_dirs,
                hide_gitignore: settings.hide_gitignore,
                auto_reveal: settings.auto_reveal,
                focus_handle: Some(focus_handle),
                filename_editor: Some(filename_editor),
                ..Default::default()
            }
        })
    }

    #[inline]
    pub fn filename_editor(&self) -> &Entity<crate::filename_editor::FilenameEditor> {
        self.filename_editor
            .as_ref()
            .expect("filename_editor initialized on panel entity creation")
    }
}

impl Default for ExplorerState {
    fn default() -> Self {
        let mut state = Self {
            tree_visible: false,
            sort_mode: ExplorerSortMode::DirectoriesFirst,
            sort_order: ExplorerSortOrder::Ascending,
            auto_fold_dirs: true,
            hide_gitignore: false,
            auto_reveal: true,
            selection_anchor: None,
            worktrees: Vec::new(),
            snapshots: Vec::new(),
            expanded: HashMap::new(),
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
            subscriptions: HashMap::new(),
            focus_handle: None,
            filename_editor: None,
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

// ── Ergonomic Sorting & Visible Rows Derivation ──────────────────────────

/// Natural alphanumeric comparison (case-insensitive, numeric-aware).
/// Groups of ASCII digits are compared as numbers; non-digits are compared case-insensitively.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_bytes = a.as_bytes();
    let mut b_bytes = b.as_bytes();

    while !a_bytes.is_empty() && !b_bytes.is_empty() {
        if a_bytes[0].is_ascii_digit() && b_bytes[0].is_ascii_digit() {
            let a_len = a_bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            let b_len = b_bytes.iter().take_while(|b| b.is_ascii_digit()).count();

            let a_digits = &a_bytes[..a_len];
            let b_digits = &b_bytes[..b_len];

            // Skip leading zeros
            let a_non_zero = a_digits.iter().position(|&b| b != b'0').unwrap_or(a_len);
            let b_non_zero = b_digits.iter().position(|&b| b != b'0').unwrap_or(b_len);

            let a_trimmed = &a_digits[a_non_zero..];
            let b_trimmed = &b_digits[b_non_zero..];

            let ord = a_trimmed
                .len()
                .cmp(&b_trimmed.len())
                .then_with(|| a_trimmed.cmp(b_trimmed))
                .then_with(|| a_len.cmp(&b_len));

            if ord != std::cmp::Ordering::Equal {
                return ord;
            }

            a_bytes = &a_bytes[a_len..];
            b_bytes = &b_bytes[b_len..];
        } else {
            let ca = a_bytes[0].to_ascii_lowercase();
            let cb = b_bytes[0].to_ascii_lowercase();
            match ca.cmp(&cb) {
                std::cmp::Ordering::Equal => {
                    a_bytes = &a_bytes[1..];
                    b_bytes = &b_bytes[1..];
                }
                diff => return diff,
            }
        }
    }

    a_bytes.len().cmp(&b_bytes.len()).then_with(|| a.cmp(b))
}

/// Compare two entries in the same directory using ergonomic rules:
/// 1. Group by node kind (DirectoriesFirst, FilesFirst, or Mixed).
/// 2. Sort by natural alphanumeric order within the same kind.
/// 3. Invert the name comparison if Descending.
pub fn compare_worktree_entries(
    a: &WorktreeEntry,
    b: &WorktreeEntry,
    sort_mode: ExplorerSortMode,
    sort_order: ExplorerSortOrder,
) -> std::cmp::Ordering {
    let kind_cmp = match sort_mode {
        ExplorerSortMode::DirectoriesFirst => match (a.kind, b.kind) {
            (WorktreeEntryKind::Directory, WorktreeEntryKind::File) => std::cmp::Ordering::Less,
            (WorktreeEntryKind::File, WorktreeEntryKind::Directory) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        },
        ExplorerSortMode::FilesFirst => match (a.kind, b.kind) {
            (WorktreeEntryKind::File, WorktreeEntryKind::Directory) => std::cmp::Ordering::Less,
            (WorktreeEntryKind::Directory, WorktreeEntryKind::File) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        },
        ExplorerSortMode::Mixed => std::cmp::Ordering::Equal,
    };

    if kind_cmp != std::cmp::Ordering::Equal {
        return kind_cmp;
    }

    let name_a = a.path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let name_b = b.path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    let name_cmp = natural_cmp(name_a, name_b);

    match sort_order {
        ExplorerSortOrder::Ascending => name_cmp,
        ExplorerSortOrder::Descending => name_cmp.reverse(),
    }
}

fn visible_children(
    snapshot: &WorktreeSnapshot,
    parent_id: ExplorerEntryId,
    hide_gitignore: bool,
) -> Vec<&WorktreeEntry> {
    snapshot
        .child_ids(parent_id)
        .iter()
        .filter_map(|id| snapshot.entry_for_id(*id))
        .filter(|entry| !hide_gitignore || !entry.is_ignored)
        .collect()
}

fn collect_visible_entries(
    snapshot: &WorktreeSnapshot,
    worktree_id: WorktreeId,
    parent_id: ExplorerEntryId,
    depth: usize,
    expanded_set: Option<&BTreeSet<ExplorerEntryId>>,
    sort_mode: ExplorerSortMode,
    sort_order: ExplorerSortOrder,
    auto_fold_dirs: bool,
    hide_gitignore: bool,
    flat_entries: &mut Vec<VisibleExplorerEntry>,
) {
    let mut children = visible_children(snapshot, parent_id, hide_gitignore);
    if children.is_empty() {
        return;
    }

    children.sort_by(|a, b| compare_worktree_entries(a, b, sort_mode, sort_order));

    for child in children {
        let is_dir = child.kind == WorktreeEntryKind::Directory;
        if is_dir {
            let mut folded_ancestors = Vec::new();
            let mut current = child;
            let mut combined_label = current
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| current.path.to_string_lossy().into_owned());

            if auto_fold_dirs {
                loop {
                    let subchildren = visible_children(snapshot, current.id, hide_gitignore);
                    if subchildren.len() == 1 && subchildren[0].kind == WorktreeEntryKind::Directory
                    {
                        let only_child = subchildren[0];
                        folded_ancestors.push(current.id);
                        let child_name = only_child
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| only_child.path.to_string_lossy().into_owned());
                        combined_label.push_str(" / ");
                        combined_label.push_str(&child_name);
                        current = only_child;
                    } else {
                        break;
                    }
                }
            }

            let is_expanded = expanded_set.is_some_and(|set| set.contains(&current.id));
            let has_children = snapshot.child_count(current.id) > 0;
            let is_ignored = current.is_ignored || child.is_ignored;

            flat_entries.push(VisibleExplorerEntry {
                worktree_id,
                id: current.id,
                parent_id: Some(parent_id),
                path: current.path.clone(),
                label: combined_label,
                depth,
                kind: ExplorerEntryKind::Directory,
                is_expanded,
                has_children,
                is_ignored,
                folded_ancestors,
            });

            if is_expanded {
                collect_visible_entries(
                    snapshot,
                    worktree_id,
                    current.id,
                    depth + 1,
                    expanded_set,
                    sort_mode,
                    sort_order,
                    auto_fold_dirs,
                    hide_gitignore,
                    flat_entries,
                );
            }
        } else {
            let label = child
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| child.path.to_string_lossy().into_owned());

            let kind = if child
                .path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                ExplorerEntryKind::MarkdownFile
            } else {
                ExplorerEntryKind::File
            };

            flat_entries.push(VisibleExplorerEntry {
                worktree_id,
                id: child.id,
                parent_id: Some(parent_id),
                path: child.path.clone(),
                label,
                depth,
                kind,
                is_expanded: false,
                has_children: false,
                is_ignored: child.is_ignored,
                folded_ancestors: Vec::new(),
            });
        }
    }
}

/// Derive the flat visible row list directly from each [`WorktreeSnapshot`]
/// by traversing only expanded directories in $O(V)$ time, with children sorted
/// by the specified sort mode and sort order.
pub fn build_explorer_rows(
    snapshots: &[Arc<WorktreeSnapshot>],
    expanded: &HashMap<WorktreeId, BTreeSet<ExplorerEntryId>>,
    edit: Option<&ExplorerEditState>,
    sort_mode: ExplorerSortMode,
    sort_order: ExplorerSortOrder,
    auto_fold_dirs: bool,
    hide_gitignore: bool,
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
            Vec::with_capacity(snapshot.entries_by_path.len().min(64));

        let root_is_dir = root_entry.kind == WorktreeEntryKind::Directory;
        let root_kind = if root_is_dir {
            ExplorerEntryKind::Directory
        } else if root_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            ExplorerEntryKind::MarkdownFile
        } else {
            ExplorerEntryKind::File
        };
        let root_label = root_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root_path.to_string_lossy().into_owned());
        let root_has_children = root_is_dir && snapshot.child_count(root_entry.id) > 0;

        flat_entries.push(VisibleExplorerEntry {
            worktree_id,
            id: root_entry.id,
            parent_id: None,
            path: root_path.clone(),
            label: root_label,
            depth: 0,
            kind: root_kind,
            is_expanded: root_is_expanded,
            has_children: root_has_children,
            is_ignored: root_entry.is_ignored,
            folded_ancestors: Vec::new(),
        });

        if root_is_expanded && root_is_dir {
            collect_visible_entries(
                snapshot,
                worktree_id,
                root_entry.id,
                1,
                expanded_set,
                sort_mode,
                sort_order,
                auto_fold_dirs,
                hide_gitignore,
                &mut flat_entries,
            );
        }

        // Splice inline edit row if active for this worktree
        let mut segment = Vec::with_capacity(flat_entries.len() + 1);
        match edit {
            Some(edit_state)
                if edit_state.is_new_entry() && edit_state.worktree_id == worktree_id =>
            {
                let parent_index = flat_entries.iter().position(|entry| {
                    Some(entry.id) == edit_state.parent_id
                        || entry
                            .folded_ancestors
                            .iter()
                            .any(|id| Some(*id) == edit_state.parent_id)
                });
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
                    if Some(entry.id) == edit_state.target_id
                        || entry
                            .folded_ancestors
                            .iter()
                            .any(|id| Some(*id) == edit_state.target_id)
                    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::undo::ExplorerChange;
    use std::cmp::Ordering;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_natural_cmp() {
        assert_eq!(
            natural_cmp("file2.txt", "file10.txt"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            natural_cmp("file10.txt", "file2.txt"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            natural_cmp("file1.txt", "file1.txt"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(natural_cmp("a.txt", "B.txt"), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp("B.txt", "a.txt"), std::cmp::Ordering::Greater);
        assert_eq!(natural_cmp("A.txt", "a.txt"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_compare_worktree_entries() {
        let dir = WorktreeEntry {
            id: ExplorerEntryId(1),
            path: PathBuf::from("/root/z_folder"),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        let file = WorktreeEntry {
            id: ExplorerEntryId(2),
            path: PathBuf::from("/root/a_file.txt"),
            kind: WorktreeEntryKind::File,
            inode: None,
            is_ignored: false,
        };

        // Directories first: dir comes before file even though 'z' > 'a'
        assert_eq!(
            compare_worktree_entries(
                &dir,
                &file,
                ExplorerSortMode::DirectoriesFirst,
                ExplorerSortOrder::Ascending
            ),
            std::cmp::Ordering::Less
        );

        // Files first: file comes before dir
        assert_eq!(
            compare_worktree_entries(
                &dir,
                &file,
                ExplorerSortMode::FilesFirst,
                ExplorerSortOrder::Ascending
            ),
            std::cmp::Ordering::Greater
        );

        // Mixed: 'a_file.txt' comes before 'z_folder'
        assert_eq!(
            compare_worktree_entries(
                &dir,
                &file,
                ExplorerSortMode::Mixed,
                ExplorerSortOrder::Ascending
            ),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_build_explorer_rows_directories_first() {
        let mut entries_by_path = std::collections::BTreeMap::new();
        let mut id_for_path = std::collections::HashMap::new();
        let mut path_for_id = std::collections::HashMap::new();
        let mut children_by_parent = std::collections::HashMap::new();

        let root = PathBuf::from("/project");
        let root_entry = WorktreeEntry {
            id: ExplorerEntryId(0),
            path: root.clone(),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        entries_by_path.insert(root.clone(), root_entry.clone());
        id_for_path.insert(root.clone(), ExplorerEntryId(0));
        path_for_id.insert(ExplorerEntryId(0), root.clone());

        // Children of root:
        // 1. Cargo.toml (file)
        // 2. src (dir)
        // 3. assets (dir)
        // 4. README.md (file)
        let items = [
            ("Cargo.toml", WorktreeEntryKind::File, 1),
            ("src", WorktreeEntryKind::Directory, 2),
            ("assets", WorktreeEntryKind::Directory, 3),
            ("README.md", WorktreeEntryKind::File, 4),
        ];

        let mut root_child_ids = Vec::new();
        for (name, kind, id_num) in items {
            let id = ExplorerEntryId(id_num);
            let path = root.join(name);
            let entry = WorktreeEntry {
                id,
                path: path.clone(),
                kind,
                inode: None,
                is_ignored: false,
            };
            entries_by_path.insert(path.clone(), entry);
            id_for_path.insert(path.clone(), id);
            path_for_id.insert(id, path);
            root_child_ids.push(id);
        }
        children_by_parent.insert(ExplorerEntryId(0), root_child_ids);

        let snapshot = Arc::new(WorktreeSnapshot {
            worktree_id: WorktreeId(1),
            entries_by_path,
            id_for_path,
            path_for_id,
            inode_to_id: std::collections::HashMap::new(),
            dir_child_counts: std::collections::HashMap::new(),
            children_by_parent,
        });

        let mut expanded = HashMap::new();
        let mut exp_set = BTreeSet::new();
        exp_set.insert(ExplorerEntryId(0)); // root expanded
        expanded.insert(WorktreeId(1), exp_set);

        let rows = build_explorer_rows(
            &[snapshot],
            &expanded,
            None,
            ExplorerSortMode::DirectoriesFirst,
            ExplorerSortOrder::Ascending,
            false,
            false,
        );

        let labels: Vec<String> = rows
            .into_iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label),
                _ => None,
            })
            .collect();

        // Root is first, then directories ('assets', 'src'), then files ('Cargo.toml', 'README.md')
        assert_eq!(
            labels,
            vec!["project", "assets", "src", "Cargo.toml", "README.md"]
        );
    }

    #[test]
    fn test_build_explorer_rows_nested_expanded() {
        let mut entries_by_path = std::collections::BTreeMap::new();
        let mut id_for_path = std::collections::HashMap::new();
        let mut path_for_id = std::collections::HashMap::new();
        let mut children_by_parent = std::collections::HashMap::new();

        let root = PathBuf::from("/project");
        let root_entry = WorktreeEntry {
            id: ExplorerEntryId(0),
            path: root.clone(),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        entries_by_path.insert(root.clone(), root_entry.clone());
        id_for_path.insert(root.clone(), ExplorerEntryId(0));
        path_for_id.insert(ExplorerEntryId(0), root.clone());

        // Root has child dir "src" (id 1) and file "a_root.txt" (id 2)
        let src_path = root.join("src");
        entries_by_path.insert(
            src_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(1),
                path: src_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(src_path.clone(), ExplorerEntryId(1));
        path_for_id.insert(ExplorerEntryId(1), src_path.clone());

        let root_file_path = root.join("a_root.txt");
        entries_by_path.insert(
            root_file_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(2),
                path: root_file_path.clone(),
                kind: WorktreeEntryKind::File,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(root_file_path.clone(), ExplorerEntryId(2));
        path_for_id.insert(ExplorerEntryId(2), root_file_path);

        children_by_parent.insert(
            ExplorerEntryId(0),
            vec![ExplorerEntryId(1), ExplorerEntryId(2)],
        );

        // "src" has child file "main.rs" (id 3)
        let main_path = src_path.join("main.rs");
        entries_by_path.insert(
            main_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(3),
                path: main_path.clone(),
                kind: WorktreeEntryKind::File,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(main_path.clone(), ExplorerEntryId(3));
        path_for_id.insert(ExplorerEntryId(3), main_path);

        children_by_parent.insert(ExplorerEntryId(1), vec![ExplorerEntryId(3)]);

        let snapshot = Arc::new(WorktreeSnapshot {
            worktree_id: WorktreeId(1),
            entries_by_path,
            id_for_path,
            path_for_id,
            inode_to_id: std::collections::HashMap::new(),
            dir_child_counts: std::collections::HashMap::new(),
            children_by_parent,
        });

        // Expand root and src
        let mut expanded = HashMap::new();
        let mut exp_set = BTreeSet::new();
        exp_set.insert(ExplorerEntryId(0));
        exp_set.insert(ExplorerEntryId(1));
        expanded.insert(WorktreeId(1), exp_set);

        let rows = build_explorer_rows(
            &[snapshot],
            &expanded,
            None,
            ExplorerSortMode::DirectoriesFirst,
            ExplorerSortOrder::Ascending,
            false,
            false,
        );

        let labels: Vec<String> = rows
            .into_iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label),
                _ => None,
            })
            .collect();

        // project -> src -> main.rs -> a_root.txt
        assert_eq!(labels, vec!["project", "src", "main.rs", "a_root.txt"]);
    }

    #[test]
    fn test_trailing_slash_path_parsing() {
        let input_unix = "nested/dir/my_folder/";
        let is_dir_unix = input_unix.ends_with('/') || input_unix.ends_with('\\');
        assert!(is_dir_unix);
        let parts_unix: Vec<&str> = input_unix
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(parts_unix, vec!["nested", "dir", "my_folder"]);

        let input_win = "nested\\dir\\my_folder\\";
        let is_dir_win = input_win.ends_with('/') || input_win.ends_with('\\');
        assert!(is_dir_win);
        let parts_win: Vec<&str> = input_win
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(parts_win, vec!["nested", "dir", "my_folder"]);

        let file_input = "nested/dir/file.txt";
        let is_dir_file = file_input.ends_with('/') || file_input.ends_with('\\');
        assert!(!is_dir_file);
        let parts_file: Vec<&str> = file_input
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(parts_file, vec!["nested", "dir", "file.txt"]);
    }

    #[test]
    fn test_natural_cmp_complex() {
        assert_eq!(natural_cmp("file1.txt", "file2.txt"), Ordering::Less);
        assert_eq!(natural_cmp("file2.txt", "file10.txt"), Ordering::Less);
        assert_eq!(natural_cmp("file10.txt", "file100.txt"), Ordering::Less);
        assert_eq!(natural_cmp("item1", "item01"), Ordering::Less);
        assert_eq!(natural_cmp("item01", "item1"), Ordering::Greater);
        assert_eq!(natural_cmp("item1", "item1"), Ordering::Equal);
        assert_eq!(natural_cmp("apple", "Banana"), Ordering::Less);
        assert_eq!(natural_cmp("Banana", "cherry"), Ordering::Less);
    }

    #[test]
    fn test_filename_editor_mouse_and_selection() {
        let mut editor = ExplorerFilenameEditor::default();
        editor.set_text("test_file.rs".to_string(), Some(0..9));
        assert_eq!(editor.selected_text(), "test_file");

        editor.select_all();
        assert_eq!(editor.selected_text(), "test_file.rs");

        editor.move_to(4);
        assert_eq!(editor.cursor(), 4);
        assert!(editor.selection_range().is_empty());

        editor.select_to(9);
        assert_eq!(editor.selected_text(), "_file");

        editor.select_to(0);
        assert_eq!(editor.selected_text(), "test");
    }

    struct TestDir(std::path::PathBuf);
    impl TestDir {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "splitype_test_{}_{}_{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = std::fs::create_dir_all(&dir);
            Self(dir)
        }
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_execute_entry_ops_disambiguate_policy() {
        let temp = TestDir::new();
        let target_dir = temp.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();

        let src_file = temp.path().join("file.txt");
        std::fs::write(&src_file, "source content").unwrap();

        let existing_dest = target_dir.join("file.txt");
        std::fs::write(&existing_dest, "original content").unwrap();

        // 1. With disambiguate = true (drag-copy with collision): creates "file copy.txt"
        let changes = crate::state::utils::execute_entry_ops(
            std::slice::from_ref(&src_file),
            &target_dir,
            false,
            true,
        );
        assert_eq!(changes.len(), 1);
        if let crate::state::undo::ExplorerChange::Copied { dest, .. } = &changes[0] {
            assert!(dest.file_name().unwrap().to_str().unwrap().contains("copy"));
            assert!(dest.exists());
        } else {
            panic!("expected Copied change");
        }

        // 2. With disambiguate = false (external replace drop): overwrites "file.txt"
        let changes = crate::state::utils::execute_entry_ops(
            std::slice::from_ref(&src_file),
            &target_dir,
            false,
            false,
        );
        assert_eq!(changes.len(), 1);
        if let crate::state::undo::ExplorerChange::Copied { dest, .. } = &changes[0] {
            assert_eq!(dest, &existing_dest);
            assert_eq!(std::fs::read_to_string(dest).unwrap(), "source content");
        } else {
            panic!("expected Copied change");
        }
    }

    #[test]
    fn test_filename_editor_insert_and_typing() {
        let mut editor = ExplorerFilenameEditor::default();
        assert_eq!(editor.text, "");

        // Typing characters and space
        editor.insert_at_selection("my");
        editor.insert_at_selection(" ");
        editor.insert_at_selection("file.rs");
        assert_eq!(editor.text, "my file.rs");
        assert_eq!(editor.cursor(), 10);

        // Backspace
        editor.delete_backward();
        editor.delete_backward();
        editor.delete_backward();
        assert_eq!(editor.text, "my file");

        // Move to start and insert
        editor.move_home(false);
        assert_eq!(editor.cursor(), 0);
        editor.insert_at_selection("new_");
        assert_eq!(editor.text, "new_my file");

        // Selection replacement
        editor.set_text("renamed_file.txt".to_string(), Some(0..7));
        assert_eq!(editor.selected_text(), "renamed");
        editor.insert_at_selection("updated");
        assert_eq!(editor.text, "updated_file.txt");
    }

    #[test]
    fn test_filename_editor_backspace_and_marked_range() {
        let mut editor = ExplorerFilenameEditor::default();

        // 1. Backspace on empty text does not panic or underflow
        editor.delete_backward();
        assert_eq!(editor.text, "");
        assert_eq!(editor.cursor(), 0);

        // 2. Normal deletion
        editor.insert_at_selection("hello");
        assert_eq!(editor.text, "hello");
        editor.delete_backward();
        assert_eq!(editor.text, "hell");
        editor.delete_forward();
        assert_eq!(editor.text, "hell");

        // 3. Selection deletion
        editor.selection = 1..3;
        editor.delete_backward();
        assert_eq!(editor.text, "hl");
        assert_eq!(editor.cursor(), 1);

        // 4. Marked range (IME composition) deletion
        editor.set_text("nihk".to_string(), None);
        editor.marked_range = Some(2..4);
        if let Some(marked) = editor.marked_range.take() {
            editor.replace_range(marked, "");
        } else {
            editor.delete_backward();
        }
        assert_eq!(editor.text, "ni");
        assert_eq!(editor.marked_range, None);

        // 5. Interior mutability verification: setting layout, bounds, and scroll does not require &mut
        editor.last_bounds.set(Some(gpui::Bounds::default()));
        assert!(editor.last_bounds.get().is_some());
        editor.scroll_offset.set(gpui::px(42.0));
        assert_eq!(editor.scroll_offset.get(), gpui::px(42.0));
    }

    #[test]
    fn test_drag_and_drop_move_to_different_folder() {
        let temp = TestDir::new();
        let folder_a = temp.path().join("folder_a");
        let folder_b = temp.path().join("folder_b");
        std::fs::create_dir_all(&folder_a).unwrap();
        std::fs::create_dir_all(&folder_b).unwrap();

        let file1 = folder_a.join("file1.txt");
        std::fs::write(&file1, "file1 content").unwrap();

        let file2 = folder_b.join("file2.txt");
        std::fs::write(&file2, "file2 content").unwrap();

        // 1. Moving file1 from folder_a into folder_b (dragging onto folder_b or file in folder_b)
        let changes = crate::state::utils::execute_entry_ops(
            std::slice::from_ref(&file1),
            &folder_b,
            true, // is_cut (move)
            false,
        );
        assert_eq!(changes.len(), 1);
        let dest = folder_b.join("file1.txt");
        assert!(!file1.exists(), "source file should be moved away");
        assert!(dest.exists(), "dest file should exist in folder_b");
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "file1 content");

        // 2. Moving file1 into folder_b again (same parent directory / sibling file): should be a no-op
        let noop_changes = crate::state::utils::execute_entry_ops(
            std::slice::from_ref(&dest),
            &folder_b,
            true,
            false,
        );
        assert_eq!(
            noop_changes.len(),
            0,
            "moving a file into its own parent directory is a no-op"
        );
        assert!(dest.exists());
    }

    #[test]
    fn test_context_menu_selection_and_highlight_target() {
        let entry1 = SelectedEntry {
            worktree_id: WorktreeId(1),
            entry_id: ExplorerEntryId(10),
        };
        let entry2 = SelectedEntry {
            worktree_id: WorktreeId(1),
            entry_id: ExplorerEntryId(20),
        };
        let path1 = PathBuf::from("/root/file1.txt");
        let path2 = PathBuf::from("/root/file2.txt");

        let mut marked = std::collections::HashSet::new();
        marked.insert(entry1);
        marked.insert(entry2);

        // Case 1: Right-clicking entry1 (which is already inside multi-selection)
        // mirrors Zed: keeps all marked entries intact and sets selected to entry1.
        let target_selection = entry1;
        let mut selected = Some(target_selection);
        if !marked.contains(&target_selection) {
            marked.clear();
        }
        assert_eq!(
            marked.len(),
            2,
            "multi-selection should remain intact when right-clicking a marked item"
        );
        assert_eq!(selected, Some(entry1));

        // Case 2: Right-clicking entry3 (not in multi-selection)
        // mirrors Zed: clears previous multi-selection and sets selected to entry3.
        let entry3 = SelectedEntry {
            worktree_id: WorktreeId(1),
            entry_id: ExplorerEntryId(30),
        };
        let target_selection_unmarked = entry3;
        selected = Some(target_selection_unmarked);
        if !marked.contains(&target_selection_unmarked) {
            marked.clear();
        }
        assert!(
            marked.is_empty(),
            "marked entries should clear when right-clicking an unmarked item"
        );
        assert_eq!(selected, Some(entry3));

        // Case 3: Context menu open state marks target path as active highlight target
        let file_menu = Some(ExplorerFileMenuState {
            position: gpui::Point {
                x: gpui::px(100.0),
                y: gpui::px(200.0),
            },
            path: path1.clone(),
            is_dir: false,
        });

        let is_menu_target_path1 = file_menu.as_ref().is_some_and(|m| m.path == path1);
        let is_menu_target_path2 = file_menu.as_ref().is_some_and(|m| m.path == path2);
        assert!(
            is_menu_target_path1,
            "file1 should be recognized as active context menu target"
        );
        assert!(
            !is_menu_target_path2,
            "file2 should not be recognized as active context menu target"
        );
    }

    #[test]
    fn test_filename_editor_word_motion_and_delete() {
        let mut editor = ExplorerFilenameEditor::default();
        editor.set_text("hello_world.test.txt".to_string(), None);
        assert_eq!(editor.cursor(), 20);

        // Move word left: should stop at "txt" boundary (index 17)
        editor.move_word_left(false);
        assert_eq!(editor.cursor(), 17);

        // Move word left: should stop at delimiter "." (index 16)
        editor.move_word_left(false);
        assert_eq!(editor.cursor(), 16);

        // Move word left: should stop at "test" boundary (index 12)
        editor.move_word_left(false);
        assert_eq!(editor.cursor(), 12);

        // Move word right: should jump over "test" to delimiter (index 16)
        editor.move_word_right(false);
        assert_eq!(editor.cursor(), 16);

        // Move word right: should jump over "." to "txt" (index 17)
        editor.move_word_right(false);
        assert_eq!(editor.cursor(), 17);

        // Move word right: should jump to end (index 20)
        editor.move_word_right(false);
        assert_eq!(editor.cursor(), 20);

        // Delete word backward: deletes "txt"
        editor.delete_word_backward();
        assert_eq!(editor.text, "hello_world.test.");
        assert_eq!(editor.cursor(), 17);

        // Delete word backward: deletes "."
        editor.delete_word_backward();
        assert_eq!(editor.text, "hello_world.test");
        assert_eq!(editor.cursor(), 16);

        // Delete word backward: deletes "test"
        editor.delete_word_backward();
        assert_eq!(editor.text, "hello_world.");
        assert_eq!(editor.cursor(), 12);
    }

    #[test]
    fn test_disjoint_explorer_entries() {
        let wt_id = WorktreeId(1);
        let root_dir = WorktreeEntry {
            id: ExplorerEntryId(1),
            path: PathBuf::from("/workspace/src"),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        let child_dir = WorktreeEntry {
            id: ExplorerEntryId(2),
            path: PathBuf::from("/workspace/src/nested"),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        let child_file = WorktreeEntry {
            id: ExplorerEntryId(3),
            path: PathBuf::from("/workspace/src/nested/main.rs"),
            kind: WorktreeEntryKind::File,
            inode: None,
            is_ignored: false,
        };
        let sibling_file = WorktreeEntry {
            id: ExplorerEntryId(4),
            path: PathBuf::from("/workspace/src/lib.rs"),
            kind: WorktreeEntryKind::File,
            inode: None,
            is_ignored: false,
        };

        let mut entries_by_path = std::collections::BTreeMap::new();
        let mut id_for_path = std::collections::HashMap::new();
        let mut path_for_id = std::collections::HashMap::new();

        for entry in [&root_dir, &child_dir, &child_file, &sibling_file] {
            entries_by_path.insert(entry.path.clone(), (*entry).clone());
            id_for_path.insert(entry.path.clone(), entry.id);
            path_for_id.insert(entry.id, entry.path.clone());
        }

        let snapshot = Arc::new(WorktreeSnapshot {
            worktree_id: wt_id,
            entries_by_path,
            id_for_path,
            path_for_id,
            inode_to_id: std::collections::HashMap::new(),
            dir_child_counts: std::collections::HashMap::new(),
            children_by_parent: std::collections::HashMap::new(),
        });

        let state = ExplorerState {
            snapshots: vec![snapshot],
            ..Default::default()
        };

        // Scenario: Multi-selection has both `nested` (directory) and `main.rs` (child file)
        let selections = vec![
            SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(2), // nested dir
            },
            SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(3), // main.rs (inside nested dir)
            },
            SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(4), // lib.rs (sibling)
            },
        ];

        let disjoint = state.disjoint_explorer_entries(selections);

        // Child file `main.rs` must be pruned because `nested` is selected
        assert_eq!(disjoint.len(), 2);
        assert!(disjoint.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(2),
        }));
        assert!(disjoint.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(4),
        }));
        assert!(!disjoint.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(3),
        }));
    }

    #[test]
    fn test_explorer_paste_target_dir_folder_duplication() {
        let wt_id = WorktreeId(1);
        let folder = WorktreeEntry {
            id: ExplorerEntryId(10),
            path: PathBuf::from("/workspace/my_folder"),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };

        let mut entries_by_path = std::collections::BTreeMap::new();
        let mut id_for_path = std::collections::HashMap::new();
        let mut path_for_id = std::collections::HashMap::new();

        entries_by_path.insert(folder.path.clone(), folder.clone());
        id_for_path.insert(folder.path.clone(), folder.id);
        path_for_id.insert(folder.id, folder.path.clone());

        let snapshot = Arc::new(WorktreeSnapshot {
            worktree_id: wt_id,
            entries_by_path,
            id_for_path,
            path_for_id,
            inode_to_id: std::collections::HashMap::new(),
            dir_child_counts: std::collections::HashMap::new(),
            children_by_parent: std::collections::HashMap::new(),
        });

        let mut state = ExplorerState {
            snapshots: vec![snapshot],
            ..Default::default()
        };

        let selected = SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(10),
        };
        state.selected = Some(selected);

        // When clipboard contains the selected folder (e.g. user duplicated or copied it),
        // paste target directory MUST pop to parent ("/workspace"), NOT stay as "/workspace/my_folder"!
        let mut clipboard_items = std::collections::BTreeSet::new();
        clipboard_items.insert(selected);
        state.clipboard = Some(ExplorerClipboard::Copied(clipboard_items));

        let target_dir = state.explorer_paste_target_dir();
        assert_eq!(
            target_dir,
            Some(PathBuf::from("/workspace")),
            "folder duplicate/paste into itself must target parent folder"
        );

        // When clipboard contains a DIFFERENT item, paste target directory is the folder itself
        let other_item = SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(999),
        };
        let mut other_clipboard = std::collections::BTreeSet::new();
        other_clipboard.insert(other_item);
        state.clipboard = Some(ExplorerClipboard::Copied(other_clipboard));

        let target_dir_other = state.explorer_paste_target_dir();
        assert_eq!(
            target_dir_other,
            Some(PathBuf::from("/workspace/my_folder")),
            "pasting a different item into a directory must target that directory"
        );
    }

    #[test]
    fn test_non_destructive_redo() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_path = std::env::temp_dir().join(format!("splitype_test_redo_{unique_id}.txt"));

        // Write initial content
        std::fs::write(&file_path, "important content").unwrap();

        let mut change = ExplorerChange::Created {
            path: file_path.clone(),
            is_dir: false,
            trashed_backup: None,
        };

        // Redo should NOT overwrite existing file with empty string!
        let redo_result = crate::state::undo::execute_explorer_change(&mut change);
        assert!(redo_result.is_ok());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            content, "important content",
            "redo must never overwrite existing content with empty string"
        );
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_created_undo_redo_preserves_content() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_path =
            std::env::temp_dir().join(format!("splitype_test_created_preserves_{unique_id}.txt"));

        // User created file and wrote content
        std::fs::write(&file_path, "user critical data").unwrap();

        let mut change = ExplorerChange::Created {
            path: file_path.clone(),
            is_dir: false,
            trashed_backup: None,
        };

        // Undo creation (moves to trash, captures backup)
        let undo_result = crate::state::undo::execute_explorer_change_inverse(&mut change);
        assert!(undo_result.is_ok());

        // File should not exist in original path anymore
        assert!(!file_path.exists());

        // Redo creation (should restore from trash rather than creating empty file)
        let redo_result = crate::state::undo::execute_explorer_change(&mut change);
        assert!(redo_result.is_ok());

        // File should exist and contain the original critical data!
        assert!(file_path.exists());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            content, "user critical data",
            "redo of created entry must restore trashed content, not create an empty file"
        );
        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn test_windows_trailing_dot_and_slash_normalization() {
        let name_with_dot = "my_folder.";
        let normalized = if cfg!(windows) {
            name_with_dot.trim_end_matches('.')
        } else {
            name_with_dot
        };
        if cfg!(windows) {
            assert_eq!(normalized, "my_folder");
        }

        let name_with_slashes = "nested/folder/file.rs";
        let parts: Vec<&str> = name_with_slashes
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(parts, vec!["nested", "folder", "file.rs"]);
    }

    #[test]
    fn test_temporary_unfolding_restore() {
        let wt_id = WorktreeId(1);
        let folder_id = ExplorerEntryId(42);

        let mut expanded: std::collections::HashMap<
            WorktreeId,
            std::collections::BTreeSet<ExplorerEntryId>,
        > = std::collections::HashMap::new();

        // Simulate folder being temporarily expanded
        expanded.entry(wt_id).or_default().insert(folder_id);
        assert!(expanded.get(&wt_id).unwrap().contains(&folder_id));

        // When edit is discarded with temporarily_unfolded set:
        let temporarily_unfolded = Some((wt_id, folder_id));
        if let Some((worktree_id, parent_id)) = temporarily_unfolded {
            if let Some(expanded_set) = expanded.get_mut(&worktree_id) {
                expanded_set.remove(&parent_id);
            }
        }

        // Folder is safely re-folded
        assert!(!expanded.get(&wt_id).unwrap().contains(&folder_id));
    }

    #[test]
    fn test_auto_fold_dirs_single_child_chain() {
        let mut entries_by_path = std::collections::BTreeMap::new();
        let mut id_for_path = std::collections::HashMap::new();
        let mut path_for_id = std::collections::HashMap::new();
        let mut children_by_parent = std::collections::HashMap::new();

        let root = PathBuf::from("/project");
        let root_entry = WorktreeEntry {
            id: ExplorerEntryId(0),
            path: root.clone(),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        entries_by_path.insert(root.clone(), root_entry);
        id_for_path.insert(root.clone(), ExplorerEntryId(0));
        path_for_id.insert(ExplorerEntryId(0), root.clone());

        // Single child: /project/crates (id 1)
        let crates_path = root.join("crates");
        entries_by_path.insert(
            crates_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(1),
                path: crates_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(crates_path.clone(), ExplorerEntryId(1));
        path_for_id.insert(ExplorerEntryId(1), crates_path.clone());
        children_by_parent.insert(ExplorerEntryId(0), vec![ExplorerEntryId(1)]);

        // Single child: /project/crates/explorer (id 2)
        let explorer_path = crates_path.join("explorer");
        entries_by_path.insert(
            explorer_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(2),
                path: explorer_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(explorer_path.clone(), ExplorerEntryId(2));
        path_for_id.insert(ExplorerEntryId(2), explorer_path.clone());
        children_by_parent.insert(ExplorerEntryId(1), vec![ExplorerEntryId(2)]);

        // Single child: /project/crates/explorer/src (id 3)
        let src_path = explorer_path.join("src");
        entries_by_path.insert(
            src_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(3),
                path: src_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(src_path.clone(), ExplorerEntryId(3));
        path_for_id.insert(ExplorerEntryId(3), src_path.clone());
        children_by_parent.insert(ExplorerEntryId(2), vec![ExplorerEntryId(3)]);

        // Multiple children inside src: lib.rs (id 4), main.rs (id 5)
        let lib_path = src_path.join("lib.rs");
        entries_by_path.insert(
            lib_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(4),
                path: lib_path.clone(),
                kind: WorktreeEntryKind::File,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(lib_path.clone(), ExplorerEntryId(4));
        path_for_id.insert(ExplorerEntryId(4), lib_path);

        let main_path = src_path.join("main.rs");
        entries_by_path.insert(
            main_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(5),
                path: main_path.clone(),
                kind: WorktreeEntryKind::File,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(main_path.clone(), ExplorerEntryId(5));
        path_for_id.insert(ExplorerEntryId(5), main_path);

        children_by_parent.insert(
            ExplorerEntryId(3),
            vec![ExplorerEntryId(4), ExplorerEntryId(5)],
        );

        let snapshot = Arc::new(WorktreeSnapshot {
            worktree_id: WorktreeId(1),
            entries_by_path,
            id_for_path,
            path_for_id,
            inode_to_id: std::collections::HashMap::new(),
            dir_child_counts: std::collections::HashMap::new(),
            children_by_parent,
        });

        // Case 1: auto_fold_dirs = true
        let mut expanded = HashMap::new();
        let mut exp_set = BTreeSet::new();
        exp_set.insert(ExplorerEntryId(0)); // root expanded
        expanded.insert(WorktreeId(1), exp_set.clone());

        let rows_folded = build_explorer_rows(
            std::slice::from_ref(&snapshot),
            &expanded,
            None,
            ExplorerSortMode::DirectoriesFirst,
            ExplorerSortOrder::Ascending,
            true, // auto_fold_dirs = true
            false,
        );

        let labels_folded: Vec<String> = rows_folded
            .into_iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label),
                _ => None,
            })
            .collect();

        // With auto_fold_dirs = true, crates/explorer/src collapses into a single row!
        assert_eq!(labels_folded, vec!["project", "crates / explorer / src"]);

        // When the folded row (id 3) is also expanded:
        let mut exp_set_with_src = exp_set.clone();
        exp_set_with_src.insert(ExplorerEntryId(3));
        expanded.insert(WorktreeId(1), exp_set_with_src);

        let rows_expanded = build_explorer_rows(
            std::slice::from_ref(&snapshot),
            &expanded,
            None,
            ExplorerSortMode::DirectoriesFirst,
            ExplorerSortOrder::Ascending,
            true,
            false,
        );
        let labels_expanded: Vec<String> = rows_expanded
            .into_iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label),
                _ => None,
            })
            .collect();
        assert_eq!(
            labels_expanded,
            vec!["project", "crates / explorer / src", "lib.rs", "main.rs"]
        );

        // Case 2: auto_fold_dirs = false
        expanded.insert(WorktreeId(1), exp_set);
        let rows_unfolded = build_explorer_rows(
            &[snapshot],
            &expanded,
            None,
            ExplorerSortMode::DirectoriesFirst,
            ExplorerSortOrder::Ascending,
            false, // auto_fold_dirs = false
            false,
        );
        let labels_unfolded: Vec<String> = rows_unfolded
            .into_iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label),
                _ => None,
            })
            .collect();
        // Without folding, only "crates" is visible under root!
        assert_eq!(labels_unfolded, vec!["project", "crates"]);
    }

    #[test]
    fn test_hide_gitignore_filtering() {
        let mut entries_by_path = std::collections::BTreeMap::new();
        let mut id_for_path = std::collections::HashMap::new();
        let mut path_for_id = std::collections::HashMap::new();
        let mut children_by_parent = std::collections::HashMap::new();

        let root = PathBuf::from("/project");
        let root_entry = WorktreeEntry {
            id: ExplorerEntryId(0),
            path: root.clone(),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        entries_by_path.insert(root.clone(), root_entry);
        id_for_path.insert(root.clone(), ExplorerEntryId(0));
        path_for_id.insert(ExplorerEntryId(0), root.clone());

        // Regular child: src (id 1)
        let src_path = root.join("src");
        entries_by_path.insert(
            src_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(1),
                path: src_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(src_path.clone(), ExplorerEntryId(1));
        path_for_id.insert(ExplorerEntryId(1), src_path.clone());

        // Ignored child: target (id 2)
        let target_path = root.join("target");
        entries_by_path.insert(
            target_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(2),
                path: target_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: true,
            },
        );
        id_for_path.insert(target_path.clone(), ExplorerEntryId(2));
        path_for_id.insert(ExplorerEntryId(2), target_path.clone());

        // Regular file: Cargo.toml (id 3)
        let cargo_path = root.join("Cargo.toml");
        entries_by_path.insert(
            cargo_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(3),
                path: cargo_path.clone(),
                kind: WorktreeEntryKind::File,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(cargo_path.clone(), ExplorerEntryId(3));
        path_for_id.insert(ExplorerEntryId(3), cargo_path);

        // Ignored file: .env (id 4)
        let env_path = root.join(".env");
        entries_by_path.insert(
            env_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(4),
                path: env_path.clone(),
                kind: WorktreeEntryKind::File,
                inode: None,
                is_ignored: true,
            },
        );
        id_for_path.insert(env_path.clone(), ExplorerEntryId(4));
        path_for_id.insert(ExplorerEntryId(4), env_path);

        children_by_parent.insert(
            ExplorerEntryId(0),
            vec![
                ExplorerEntryId(1),
                ExplorerEntryId(2),
                ExplorerEntryId(3),
                ExplorerEntryId(4),
            ],
        );

        let snapshot = Arc::new(WorktreeSnapshot {
            worktree_id: WorktreeId(1),
            entries_by_path,
            id_for_path,
            path_for_id,
            inode_to_id: std::collections::HashMap::new(),
            dir_child_counts: std::collections::HashMap::new(),
            children_by_parent,
        });

        let mut expanded = HashMap::new();
        let mut exp_set = BTreeSet::new();
        exp_set.insert(ExplorerEntryId(0));
        expanded.insert(WorktreeId(1), exp_set);

        // When hide_gitignore = false: target and .env are present with is_ignored = true
        let rows_with_ignored = build_explorer_rows(
            std::slice::from_ref(&snapshot),
            &expanded,
            None,
            ExplorerSortMode::DirectoriesFirst,
            ExplorerSortOrder::Ascending,
            false,
            false, // hide_gitignore = false
        );
        let labels_with_ignored: Vec<(String, bool)> = rows_with_ignored
            .into_iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some((e.label, e.is_ignored)),
                _ => None,
            })
            .collect();
        assert_eq!(
            labels_with_ignored,
            vec![
                ("project".to_string(), false),
                ("src".to_string(), false),
                ("target".to_string(), true),
                (".env".to_string(), true),
                ("Cargo.toml".to_string(), false),
            ]
        );

        // When hide_gitignore = true: target and .env are filtered out
        let rows_hidden = build_explorer_rows(
            &[snapshot],
            &expanded,
            None,
            ExplorerSortMode::DirectoriesFirst,
            ExplorerSortOrder::Ascending,
            false,
            true, // hide_gitignore = true
        );
        let labels_hidden: Vec<String> = rows_hidden
            .into_iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label),
                _ => None,
            })
            .collect();
        assert_eq!(labels_hidden, vec!["project", "src", "Cargo.toml"]);
    }

    #[test]
    fn test_anchor_based_range_selection() {
        let wt_id = WorktreeId(1);
        let mut state = ExplorerState::default();

        // Create 5 visible file rows: row 0 to row 4
        for i in 0..5 {
            state.entries.push(ExplorerRow::Entry(VisibleExplorerEntry {
                worktree_id: wt_id,
                id: ExplorerEntryId(i),
                parent_id: None,
                path: PathBuf::from(format!("/workspace/file_{i}.txt")),
                label: format!("file_{i}.txt"),
                depth: 0,
                kind: ExplorerEntryKind::File,
                is_expanded: false,
                has_children: false,
                is_ignored: false,
                folded_ancestors: Vec::new(),
            }));
        }

        // Step 1: User selects row 1 (file_1.txt) without Shift (extend = false)
        state.update_selection_at_index(1, false);
        assert_eq!(
            state.selected,
            Some(SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(1),
            })
        );
        assert_eq!(state.selection_anchor, Some(1));
        assert!(state.marked.is_empty());

        // Step 2: User presses Shift+Down (extend = true) to row 2
        state.update_selection_at_index(2, true);
        assert_eq!(state.selection_anchor, Some(1));
        assert_eq!(
            state.selected,
            Some(SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(2),
            })
        );
        assert_eq!(state.marked.len(), 2);
        assert!(state.marked.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(1),
        }));
        assert!(state.marked.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(2),
        }));

        // Step 3: User presses Shift+Down (extend = true) to row 3
        state.update_selection_at_index(3, true);
        assert_eq!(state.selection_anchor, Some(1));
        assert_eq!(state.marked.len(), 3);
        assert!(state.marked.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(1),
        }));
        assert!(state.marked.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(2),
        }));
        assert!(state.marked.contains(&SelectedEntry {
            worktree_id: wt_id,
            entry_id: ExplorerEntryId(3),
        }));

        // Step 4: User presses Shift+Up (extend = true) backtracking to row 2
        state.update_selection_at_index(2, true);
        assert_eq!(state.selection_anchor, Some(1));
        assert_eq!(state.marked.len(), 2);
        assert!(
            !state.marked.contains(&SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(3),
            }),
            "backtracking with Shift must unselect row 3"
        );

        // Step 5: Normal movement without Shift to row 4 (extend = false)
        state.update_selection_at_index(4, false);
        assert_eq!(state.selection_anchor, Some(4));
        assert_eq!(
            state.selected,
            Some(SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(4),
            })
        );
        assert!(
            state.marked.is_empty(),
            "regular movement must clear range marks"
        );
    }

    #[test]
    fn test_auto_reveal_active_file() {
        let wt_id = WorktreeId(1);
        let mut entries_by_path = std::collections::BTreeMap::new();
        let mut id_for_path = std::collections::HashMap::new();
        let mut path_for_id = std::collections::HashMap::new();
        let mut children_by_parent = std::collections::HashMap::new();

        let root = PathBuf::from("/project");
        let root_entry = WorktreeEntry {
            id: ExplorerEntryId(0),
            path: root.clone(),
            kind: WorktreeEntryKind::Directory,
            inode: None,
            is_ignored: false,
        };
        entries_by_path.insert(root.clone(), root_entry);
        id_for_path.insert(root.clone(), ExplorerEntryId(0));
        path_for_id.insert(ExplorerEntryId(0), root.clone());

        // /project/src (id 1)
        let src_path = root.join("src");
        entries_by_path.insert(
            src_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(1),
                path: src_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(src_path.clone(), ExplorerEntryId(1));
        path_for_id.insert(ExplorerEntryId(1), src_path.clone());

        // /project/src/nested (id 2)
        let nested_path = src_path.join("nested");
        entries_by_path.insert(
            nested_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(2),
                path: nested_path.clone(),
                kind: WorktreeEntryKind::Directory,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(nested_path.clone(), ExplorerEntryId(2));
        path_for_id.insert(ExplorerEntryId(2), nested_path.clone());

        // /project/src/nested/deep.rs (id 3)
        let deep_path = nested_path.join("deep.rs");
        entries_by_path.insert(
            deep_path.clone(),
            WorktreeEntry {
                id: ExplorerEntryId(3),
                path: deep_path.clone(),
                kind: WorktreeEntryKind::File,
                inode: None,
                is_ignored: false,
            },
        );
        id_for_path.insert(deep_path.clone(), ExplorerEntryId(3));
        path_for_id.insert(ExplorerEntryId(3), deep_path.clone());

        children_by_parent.insert(ExplorerEntryId(0), vec![ExplorerEntryId(1)]);
        children_by_parent.insert(ExplorerEntryId(1), vec![ExplorerEntryId(2)]);
        children_by_parent.insert(ExplorerEntryId(2), vec![ExplorerEntryId(3)]);

        let snapshot = Arc::new(WorktreeSnapshot {
            worktree_id: wt_id,
            entries_by_path,
            id_for_path,
            path_for_id,
            inode_to_id: std::collections::HashMap::new(),
            dir_child_counts: std::collections::HashMap::new(),
            children_by_parent,
        });

        let mut state = ExplorerState {
            snapshots: vec![snapshot],
            auto_reveal: true,
            auto_fold_dirs: false,
            ..Default::default()
        };

        // Initially only root is expanded
        state
            .expanded
            .entry(wt_id)
            .or_default()
            .insert(ExplorerEntryId(0));
        state.rebuild_explorer_entries();

        // deep.rs is not visible yet because src and nested are collapsed
        let labels_before: Vec<String> = state
            .entries
            .iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(labels_before, vec!["project", "src"]);

        // Active document set to /project/src/nested/deep.rs
        state.active_file = Some(deep_path);
        let revealed = state.reveal_active_file();
        assert!(revealed);

        // Ancestors (0, 1, 2) must be expanded
        let exp = state.expanded.get(&wt_id).unwrap();
        assert!(exp.contains(&ExplorerEntryId(0)));
        assert!(exp.contains(&ExplorerEntryId(1)));
        assert!(exp.contains(&ExplorerEntryId(2)));

        // Visible entries must now contain deep.rs!
        let labels_after: Vec<String> = state
            .entries
            .iter()
            .filter_map(|r| match r {
                ExplorerRow::Entry(e) => Some(e.label.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(labels_after, vec!["project", "src", "nested", "deep.rs"]);

        // Selected entry must be deep.rs (id 3)
        assert_eq!(
            state.selected,
            Some(SelectedEntry {
                worktree_id: wt_id,
                entry_id: ExplorerEntryId(3),
            })
        );
        assert_eq!(state.selection_anchor, Some(3));
    }

    #[test]
    fn test_copy_relative_path_slash_normalization() {
        let path = Path::new("crates\\explorer\\src\\lib.rs");
        let normalized = path.to_string_lossy().replace('\\', "/");
        assert_eq!(normalized, "crates/explorer/src/lib.rs");
    }
}
