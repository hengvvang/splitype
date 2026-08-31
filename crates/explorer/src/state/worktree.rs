//! Worktree entity — disk facts for one project root (mirrors Zed's
//! `Worktree` architecture): an immutable entry snapshot, a stable id
//! allocator shared across worktrees, a recursive filesystem watcher, and
//! background rescans that reuse ids so selections and expansions survive
//! renames and moves.
//!
//! The panel never mutates a worktree; it consumes `snapshot()` clones and
//! rebuilds its visible list when `WorktreeEvent::UpdatedEntries` fires.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(not(test))]
use futures::StreamExt;
use gpui::*;
#[cfg(not(test))]
use notify::Watcher as _;

use crate::ExplorerState;

// ── Identifiers ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorktreeId(pub usize);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExplorerEntryId(pub u64);

// ── Entry model ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorktreeEntryKind {
    Directory,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// Stable id (survives renames/moves).
    pub id: ExplorerEntryId,
    /// Absolute path.
    pub path: PathBuf,
    pub kind: WorktreeEntryKind,
    /// Best-effort stable file id for rename detection.
    pub inode: Option<u64>,
}

/// Immutable snapshot of a scanned worktree. Replaced wholesale after each
/// rescan (mirrors Zed's `Snapshot`); cheap to clone (Arc'd by the panel).
#[derive(Clone, Debug, Default)]
pub struct WorktreeSnapshot {
    pub worktree_id: WorktreeId,
    /// Entries ordered by absolute path — the traversal index.
    pub entries_by_path: BTreeMap<PathBuf, WorktreeEntry>,
    /// Path → id.
    pub id_for_path: HashMap<PathBuf, ExplorerEntryId>,
    /// Id → path.
    pub path_for_id: HashMap<ExplorerEntryId, PathBuf>,
    /// File id → entry id (rename/move detection between rescans).
    inode_to_id: HashMap<u64, ExplorerEntryId>,
}

impl WorktreeSnapshot {
    #[inline]
    pub fn id(&self) -> WorktreeId {
        self.worktree_id
    }

    #[inline]
    pub fn entry_for_id(&self, id: ExplorerEntryId) -> Option<&WorktreeEntry> {
        let path = self.path_for_id.get(&id)?;
        self.entries_by_path.get(path)
    }

    #[inline]
    pub fn entry_for_path(&self, path: &Path) -> Option<&WorktreeEntry> {
        self.entries_by_path.get(path)
    }

    #[inline]
    pub fn root_entry(&self) -> Option<&WorktreeEntry> {
        self.entries_by_path.values().next()
    }

    pub fn child_entries<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a WorktreeEntry> {
        self.entries_by_path
            .range(path.to_path_buf()..)
            .skip(1)
            .take_while(move |(p, _)| p.starts_with(path))
            .filter_map(move |(p, entry)| {
                if p.parent() == Some(path) {
                    Some(entry)
                } else {
                    None
                }
            })
    }
}

/// Returns the parent directories of `path` that do not yet exist in the given
/// worktree and would therefore be created when an entry is placed at `path`,
/// ordered from the deepest to the shallowest (mirrors Zed's `missing_parent_dirs`).
pub fn missing_parent_dirs(snapshot: &WorktreeSnapshot, path: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut current = path.parent();
    while let Some(parent) = current {
        if parent.as_os_str().is_empty() || snapshot.id_for_path.contains_key(parent) {
            break;
        }
        dirs.push(parent.to_path_buf());
        current = parent.parent();
    }
    dirs
}

// ── Events ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum WorktreeEvent {
    /// The snapshot was replaced (initial scan or rescan).
    UpdatedEntries,
    /// The worktree root could not be scanned (missing / unreadable).
    Deleted,
}

// ── Entity ──────────────────────────────────────────────────────────────

pub struct Worktree {
    id: WorktreeId,
    root: PathBuf,
    root_id: ExplorerEntryId,
    snapshot: Arc<WorktreeSnapshot>,
    /// Shared across all worktrees in one panel (mirrors Zed's
    /// `WorktreeStore::next_entry_id`).
    next_entry_id: Arc<AtomicU64>,
    /// Skip dotfiles in scans (persisted explorer setting).
    hide_hidden: bool,
    /// Handle to this entity, captured at construction so background
    /// tasks can wake and re-enter the worktree (the owning explorer state
    /// keeps no `Context` to derive it from).
    self_weak: WeakEntity<Worktree>,
    /// Weak handle to the owning [`ExplorerState`] entity; scan events
    /// re-enter it to refresh the visible tree.
    explorer: WeakEntity<ExplorerState>,
    /// Window handle for try-borrow-safe re-entry from background tasks:
    /// `AnyWindowHandle::update` skips a wake-up that lands mid-render
    /// instead of panicking ("RefCell already borrowed"). `None` in tests.
    window_handle: Option<AnyWindowHandle>,
    #[cfg_attr(test, allow(dead_code))]
    fs_watch_task: Option<Task<()>>,
    scan_task: Option<Task<()>>,
    #[cfg_attr(test, allow(dead_code))]
    fs_refresh_task: Option<Task<()>>,
    needs_rescan: bool,
}

impl EventEmitter<WorktreeEvent> for Worktree {}

impl Worktree {
    pub fn new(
        id: WorktreeId,
        root: PathBuf,
        next_entry_id: Arc<AtomicU64>,
        hide_hidden: bool,
        window_handle: Option<AnyWindowHandle>,
        explorer: WeakEntity<ExplorerState>,
        cx: &mut App,
    ) -> Entity<Self> {
        let root_id = ExplorerEntryId(next_entry_id.fetch_add(1, Ordering::SeqCst));
        cx.new(|cx| {
            let snapshot = WorktreeSnapshot {
                worktree_id: id,
                ..Default::default()
            };
            let mut this = Self {
                id,
                root,
                root_id,
                snapshot: Arc::new(snapshot),
                next_entry_id,
                hide_hidden,
                self_weak: cx.weak_entity(),
                explorer,
                window_handle,
                fs_watch_task: None,
                scan_task: None,
                fs_refresh_task: None,
                needs_rescan: true,
            };
            this.start_fs_watch(cx);
            this.rescan(cx);
            this
        })
    }

    #[inline]
    pub fn id(&self) -> WorktreeId {
        self.id
    }

    #[inline]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[inline]
    pub fn root_id(&self) -> ExplorerEntryId {
        self.root_id
    }

    #[inline]
    pub fn snapshot(&self) -> Arc<WorktreeSnapshot> {
        self.snapshot.clone()
    }

    /// Update the dotfile-visibility setting and re-scan when it changed.
    pub fn set_hide_hidden(&mut self, hide_hidden: bool, cx: &mut App) {
        if self.hide_hidden == hide_hidden {
            return;
        }
        self.hide_hidden = hide_hidden;
        self.rescan(cx);
    }

    /// Request a full background rescan. While one is in flight, further
    /// requests are coalesced via `needs_rescan` (a scan must never be
    /// starved by render-driven calls).
    pub fn rescan(&mut self, cx: &mut App) {
        if self.scan_task.is_some() {
            self.needs_rescan = true;
            return;
        }
        self.needs_rescan = false;
        let root = self.root.clone();
        let root_for_log = root.clone();
        let root_for_scan = root.clone();
        let next_entry_id = self.next_entry_id.clone();
        let old_snapshot = self.snapshot.clone();
        let hide_hidden = self.hide_hidden;
        let root_id = self.root_id;
        let weak = self.self_weak.clone();
        let window_handle = self.window_handle;
        let task = cx.spawn(async move |cx: &mut AsyncApp| {
            let scanned = cx
                .background_executor()
                .spawn(async move { scan_worktree_dir(&root_for_scan, hide_hidden) })
                .await;
            // Re-enter through the window handle when available
            // (`AnyWindowHandle::update` uses try_borrow_mut, so a scan that
            // completes mid-render is skipped instead of panicking).
            let run = |this: &mut Worktree, cx: &mut Context<Worktree>| {
                this.scan_task = None;
                match scanned {
                    Ok(mut entries) if !entries.is_empty() => {
                        let mut snapshot = WorktreeSnapshot::default();
                        assign_stable_ids(
                            &old_snapshot,
                            &mut entries,
                            &next_entry_id,
                            &root,
                            root_id,
                            &mut snapshot,
                        );
                        this.snapshot = Arc::new(snapshot);
                        this.needs_rescan = false;
                        cx.emit(WorktreeEvent::UpdatedEntries);
                        // Refresh the explorer's visible tree from the new
                        // snapshot (the explorer owns no subscription).
                        //
                        // This must be deferred past the end of this update:
                        // `on_explorer_worktree_event` re-reads every
                        // worktree snapshot — including this one, which is
                        // mid-update right now — and GPUI panics when an
                        // entity is read while it is being updated.
                        let worktree_entity = cx.entity();
                        let explorer_weak = this.explorer.clone();
                        cx.defer(move |cx| {
                            let _ = explorer_weak.update(cx, |explorer, cx| {
                                explorer.on_explorer_worktree_event(
                                    worktree_entity,
                                    &WorktreeEvent::UpdatedEntries,
                                    cx,
                                );
                            });
                        });
                    }
                    Ok(_) | Err(_) => {
                        tracing::warn!(root = %root_for_log.display(), "[explorer] failed to scan worktree");
                        if old_snapshot.entries_by_path.is_empty() {
                            cx.emit(WorktreeEvent::Deleted);
                        }
                    }
                }
                if this.needs_rescan {
                    this.rescan(cx);
                }
            };
            match &window_handle {
                Some(handle) => {
                    let _ = handle.update(cx, |_view, _window, cx| {
                        let _ = weak.update(cx, run);
                    });
                }
                None => {
                    let _ = weak.update(cx, run);
                }
            }
        });
        self.scan_task = Some(task);
    }

    fn start_fs_watch(&mut self, cx: &mut App) {
        #[cfg(test)]
        {
            let _ = cx;
        }
        #[cfg(not(test))]
        {
            let root = self.root.clone();
            let weak = self.self_weak.clone();
            let window_handle = self.window_handle;
            let task = cx.spawn(async move |cx: &mut AsyncApp| {
                let (tx, mut rx) = futures::channel::mpsc::unbounded::<notify::Event>();
                let mut watcher =
                    match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                        if let Ok(event) = res {
                            let _ = tx.unbounded_send(event);
                        }
                    }) {
                        Ok(watcher) => watcher,
                        Err(err) => {
                            tracing::error!(error = %err, "[explorer] failed to start fs watcher");
                            return;
                        }
                    };
                if let Err(err) = watcher.watch(&root, notify::RecursiveMode::Recursive) {
                    tracing::warn!(root = %root.display(), error = %err, "[explorer] failed to watch directory");
                    return;
                }
                while rx.next().await.is_some() {
                    match &window_handle {
                        Some(handle) => {
                            let _ = handle.update(cx, |_view, _window, cx| {
                                let _ = weak.update(cx, |this, cx| this.on_fs_event(cx));
                            });
                        }
                        None => {
                            let _ = weak.update(cx, |this, cx| this.on_fs_event(cx));
                        }
                    }
                }
            });
            self.fs_watch_task = Some(task);
        }
    }

    /// Debounce filesystem events into a single background rescan.
    #[cfg_attr(test, allow(dead_code))]
    fn on_fs_event(&mut self, cx: &mut App) {
        if self.fs_refresh_task.is_some() {
            return;
        }
        let weak = self.self_weak.clone();
        let window_handle = self.window_handle;
        let task = cx.spawn(async move |cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(250))
                .await;
            // try-borrow path: skip the rescan request when it lands
            // mid-render (the fs watcher will fire again).
            let finish = |this: &mut Worktree, cx: &mut Context<Worktree>| {
                this.fs_refresh_task = None;
                this.rescan(cx);
            };
            match &window_handle {
                Some(handle) => {
                    let _ = handle.update(cx, |_view, _window, cx| {
                        let _ = weak.update(cx, |this, cx| finish(this, cx));
                    });
                }
                None => {
                    let _ = weak.update(cx, |this, cx| finish(this, cx));
                }
            }
        });
        self.fs_refresh_task = Some(task);
    }
}

// ── Background scan ─────────────────────────────────────────────────────

const MAX_SCAN_DEPTH: usize = 64;

/// Recursively collect every entry under `root` (root itself included).
fn scan_worktree_dir(
    root: &Path,
    hide_hidden: bool,
) -> std::io::Result<BTreeMap<PathBuf, WorktreeEntry>> {
    let mut entries = BTreeMap::new();
    let mut visited_dirs = std::collections::HashSet::new();
    let meta = std::fs::symlink_metadata(root)?;
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    visited_dirs.insert(canonical);

    entries.insert(
        root.to_path_buf(),
        WorktreeEntry {
            id: ExplorerEntryId(0),
            path: root.to_path_buf(),
            kind: if meta.is_dir() {
                WorktreeEntryKind::Directory
            } else {
                WorktreeEntryKind::File
            },
            inode: file_id(root, &meta),
        },
    );
    if meta.is_dir() {
        walk_dir(root, hide_hidden, 0, &mut visited_dirs, &mut entries)?;
    }
    Ok(entries)
}

fn walk_dir(
    dir: &Path,
    hide_hidden: bool,
    depth: usize,
    visited_dirs: &mut std::collections::HashSet<PathBuf>,
    out: &mut BTreeMap<PathBuf, WorktreeEntry>,
) -> std::io::Result<()> {
    if depth >= MAX_SCAN_DEPTH {
        return Ok(());
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // Restricted or vanished directory: skip gracefully
    };
    for entry in read_dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let file_name = entry.file_name();
        if is_ignored_entry(&file_name) {
            continue;
        }
        if hide_hidden && file_name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_symlink = meta.is_symlink();
        let target_meta = if is_symlink {
            match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue, // Broken symlink: ignore
            }
        } else {
            meta.clone()
        };

        if target_meta.is_dir() {
            let canonical = match path.canonicalize() {
                Ok(c) => c,
                Err(_) => continue,
            };
            if !visited_dirs.insert(canonical) {
                continue; // Cycle detected: skip
            }
            out.insert(
                path.clone(),
                WorktreeEntry {
                    id: ExplorerEntryId(0),
                    path: path.clone(),
                    kind: WorktreeEntryKind::Directory,
                    inode: file_id(&path, &target_meta),
                },
            );
            walk_dir(&path, hide_hidden, depth + 1, visited_dirs, out)?;
        } else {
            out.insert(
                path.clone(),
                WorktreeEntry {
                    id: ExplorerEntryId(0),
                    path: path.clone(),
                    kind: WorktreeEntryKind::File,
                    inode: file_id(&path, &target_meta),
                },
            );
        }
    }
    Ok(())
}

/// Directory names the explorer scan prunes (build outputs, VCS metadata).
fn is_ignored_entry(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some("node_modules" | "target" | "dist" | "build" | ".git")
    )
}

/// Best-effort stable file id (Windows: volume serial + file index, via
/// `GetFileInformationByHandle` — the std metadata equivalents are still
/// unstable (`windows_by_handle`)).
#[cfg(windows)]
fn file_id(path: &Path, _meta: &std::fs::Metadata) -> Option<u64> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
    };
    use windows::core::HSTRING;

    let handle: HANDLE = unsafe {
        CreateFileW(
            &HSTRING::from(path.as_os_str()),
            // FILE_READ_ATTRIBUTES — access to the file's metadata only.
            0x80,
            FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0),
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(FILE_ATTRIBUTE_NORMAL.0 | FILE_FLAG_BACKUP_SEMANTICS.0),
            None,
        )
    }
    .ok()?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    let result = unsafe { GetFileInformationByHandle(handle, &mut info) };
    unsafe {
        let _ = CloseHandle(handle);
    }
    result.ok()?;
    let index = ((u64::from(info.nFileIndexHigh)) << 32) | u64::from(info.nFileIndexLow);
    let volume = u64::from(info.dwVolumeSerialNumber);
    Some(volume ^ index.rotate_left(32))
}

#[cfg(not(windows))]
fn file_id(_path: &Path, meta: &std::fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    let dev = meta.dev();
    let ino = meta.ino();
    (dev != 0 && ino != 0).then_some(dev ^ ino.rotate_left(32))
}

/// Reuse ids from the previous snapshot — same path, or same file id (a
/// rename/move) — and allocate fresh ids otherwise. Mirrors Zed's
/// `reuse_entry_id`: selections and expansions keyed by id survive rescans.
/// The root entry always keeps the id it got at construction time, so the
/// panel's root-row expansion key stays valid from the first render.
fn assign_stable_ids(
    old: &WorktreeSnapshot,
    new_entries: &mut BTreeMap<PathBuf, WorktreeEntry>,
    next_entry_id: &AtomicU64,
    root: &Path,
    root_id: ExplorerEntryId,
    out: &mut WorktreeSnapshot,
) {
    out.worktree_id = old.worktree_id;
    for (path, entry) in new_entries.iter_mut() {
        let id = if path == root {
            root_id
        } else {
            let reused = old.id_for_path.get(path).copied().or_else(|| {
                entry
                    .inode
                    .and_then(|inode| old.inode_to_id.get(&inode).copied())
            });
            reused.unwrap_or_else(|| ExplorerEntryId(next_entry_id.fetch_add(1, Ordering::SeqCst)))
        };
        entry.id = id;
        out.id_for_path.insert(path.clone(), id);
        out.path_for_id.insert(id, path.clone());
        if let Some(inode) = entry.inode {
            out.inode_to_id.insert(inode, id);
        }
    }
    out.entries_by_path = std::mem::take(new_entries);
}
