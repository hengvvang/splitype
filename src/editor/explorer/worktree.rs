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

use futures::StreamExt;
use gpui::*;
use notify::Watcher as _;

// ── Entry model ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorktreeEntryKind {
    Directory,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// Stable id (allocated from the shared counter; survives renames).
    pub id: u64,
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
    /// Entries ordered by absolute path — the traversal index.
    pub entries_by_path: BTreeMap<PathBuf, WorktreeEntry>,
    /// Path → id.
    pub id_for_path: HashMap<PathBuf, u64>,
    /// Id → path.
    pub path_for_id: HashMap<u64, PathBuf>,
    /// File id → entry id (rename/move detection between rescans).
    inode_to_id: HashMap<u64, u64>,
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
    root: PathBuf,
    root_id: u64,
    snapshot: Arc<WorktreeSnapshot>,
    /// Shared across all worktrees in one panel (mirrors Zed's
    /// `WorktreeStore::next_entry_id`).
    next_entry_id: Arc<AtomicU64>,
    /// Skip dotfiles in scans (persisted explorer setting).
    hide_hidden: bool,
    fs_watch_task: Option<Task<()>>,
    scan_task: Option<Task<()>>,
    fs_refresh_task: Option<Task<()>>,
    needs_rescan: bool,
}

impl EventEmitter<WorktreeEvent> for Worktree {}

impl Worktree {
    pub fn new(
        root: PathBuf,
        next_entry_id: Arc<AtomicU64>,
        hide_hidden: bool,
        cx: &mut App,
    ) -> Entity<Self> {
        let root_id = next_entry_id.fetch_add(1, Ordering::SeqCst);
        cx.new(|cx| {
            let mut this = Self {
                root,
                root_id,
                snapshot: Arc::new(WorktreeSnapshot::default()),
                next_entry_id,
                hide_hidden,
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

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn root_id(&self) -> u64 {
        self.root_id
    }

    pub fn snapshot(&self) -> Arc<WorktreeSnapshot> {
        self.snapshot.clone()
    }

    /// Update the dotfile-visibility setting and re-scan when it changed.
    pub fn set_hide_hidden(&mut self, hide_hidden: bool, cx: &mut Context<Self>) {
        if self.hide_hidden == hide_hidden {
            return;
        }
        self.hide_hidden = hide_hidden;
        self.rescan(cx);
    }

    /// Request a full background rescan. While one is in flight, further
    /// requests are coalesced via `needs_rescan` (a scan must never be
    /// starved by render-driven calls).
    pub fn rescan(&mut self, cx: &mut Context<Self>) {
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
        let weak = cx.weak_entity();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let scanned = cx
                .background_executor()
                .spawn(async move { scan_worktree_dir(&root_for_scan, hide_hidden) })
                .await;
            let _ = weak.update(cx, |this, cx| {
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
                    }
                    Ok(_) | Err(_) => {
                        eprintln!("[explorer] failed to scan '{}'", root_for_log.display());
                        if old_snapshot.entries_by_path.is_empty() {
                            cx.emit(WorktreeEvent::Deleted);
                        }
                    }
                }
                if this.needs_rescan {
                    this.rescan(cx);
                }
            });
        });
        self.scan_task = Some(task);
    }

    fn start_fs_watch(&mut self, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let weak = cx.weak_entity();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let (tx, mut rx) = futures::channel::mpsc::unbounded::<notify::Event>();
            let mut watcher =
                match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                    if let Ok(event) = res {
                        let _ = tx.unbounded_send(event);
                    }
                }) {
                    Ok(watcher) => watcher,
                    Err(err) => {
                        eprintln!("[explorer] failed to start fs watcher: {err}");
                        return;
                    }
                };
            if let Err(err) = watcher.watch(&root, notify::RecursiveMode::Recursive) {
                eprintln!("[explorer] failed to watch '{}': {err}", root.display());
                return;
            }
            while rx.next().await.is_some() {
                let _ = weak.update(cx, |this, cx| this.on_fs_event(cx));
            }
        });
        self.fs_watch_task = Some(task);
    }

    /// Debounce filesystem events into a single background rescan.
    fn on_fs_event(&mut self, cx: &mut Context<Self>) {
        if self.fs_refresh_task.is_some() {
            return;
        }
        let weak = cx.weak_entity();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(250))
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.fs_refresh_task = None;
                this.rescan(cx);
            });
        });
        self.fs_refresh_task = Some(task);
    }
}

// ── Background scan ─────────────────────────────────────────────────────

/// Recursively collect every entry under `root` (root itself included).
fn scan_worktree_dir(
    root: &Path,
    hide_hidden: bool,
) -> std::io::Result<BTreeMap<PathBuf, WorktreeEntry>> {
    let mut entries = BTreeMap::new();
    let meta = std::fs::symlink_metadata(root)?;
    entries.insert(
        root.to_path_buf(),
        WorktreeEntry {
            id: 0,
            path: root.to_path_buf(),
            kind: if meta.is_dir() {
                WorktreeEntryKind::Directory
            } else {
                WorktreeEntryKind::File
            },
            inode: file_id(&root, &meta),
        },
    );
    if meta.is_dir() {
        walk_dir(root, hide_hidden, &mut entries)?;
    }
    Ok(entries)
}

fn walk_dir(
    dir: &Path,
    hide_hidden: bool,
    out: &mut BTreeMap<PathBuf, WorktreeEntry>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if is_ignored_entry(&name) || (hide_hidden && name.to_string_lossy().starts_with('.')) {
            continue;
        }
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue, // vanished mid-traversal
        };
        let kind = if meta.is_dir() {
            WorktreeEntryKind::Directory
        } else {
            WorktreeEntryKind::File
        };
        out.insert(
            path.clone(),
            WorktreeEntry {
                id: 0,
                path: path.clone(),
                kind,
                inode: file_id(&path, &meta),
            },
        );
        if meta.is_dir() {
            walk_dir(&path, hide_hidden, out)?;
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
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE,
        FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
        OPEN_EXISTING,
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
            FILE_ATTRIBUTE_NORMAL,
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
    root_id: u64,
    out: &mut WorktreeSnapshot,
) {
    for (path, entry) in new_entries.iter_mut() {
        let id = if path == root {
            root_id
        } else {
            let reused = old.id_for_path.get(path).copied().or_else(|| {
                entry
                    .inode
                    .and_then(|inode| old.inode_to_id.get(&inode).copied())
            });
            reused.unwrap_or_else(|| next_entry_id.fetch_add(1, Ordering::SeqCst))
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
