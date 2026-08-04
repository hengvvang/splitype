//! Workspace sidebar model — file-tree scanning and outline state.
//!
//! This module owns the pure workspace model: [`WorkspaceNodeKind`],
//! [`WorkspaceNode`], [`WorkspaceSelection`], and [`Workspace`], plus the
//! pure functions that scan directories. Outline parsing lives in
//! `crate::editor::panels::outline`; rendering and editor interactions stay
//! in `ui::window::workspace_view`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

// ── Icons & constants ───────────────────────────────────────────────────

pub const FOLDER_ICON: &str = "icon/workspace/folder.svg";
pub const MARKDOWN_ICON: &str = "icon/workspace/markdown.svg";
pub const FILE_ICON: &str = "icon/workspace/file.svg";
pub const WORKSPACE_NODE_HEIGHT: f32 = 28.0;
pub const WORKSPACE_NODE_INDENT: f32 = 14.0;

// ── Data types ──────────────────────────────────────────────────────────

/// What kind of filesystem or outline entry a tree node represents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceNodeKind {
    Directory(PathBuf),
    MarkdownFile(PathBuf),
    File(PathBuf),
    Heading { line: usize, level: u8 },
}

/// A node in the workspace file tree or outline tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceNode {
    pub id: String,
    pub label: String,
    pub kind: WorkspaceNodeKind,
    pub children: Vec<WorkspaceNode>,
}

/// Which item is currently selected in the explorer sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceSelection {
    File(PathBuf),
    Outline(String),
}

/// Top-level explorer sidebar state.
#[derive(Default)]
pub struct Workspace {
    pub is_open: bool,
    pub root: Option<PathBuf>,
    pub file_tree: Option<WorkspaceNode>,
    pub file_error: Option<String>,
    pub outline_tree: Vec<WorkspaceNode>,
    pub outline_source: Option<String>,
    pub expanded: HashSet<String>,
    pub selected: Option<WorkspaceSelection>,
}

// ── Filesystem helpers ──────────────────────────────────────────────────

/// Returns `true` when `path` has a `.md` extension (case-insensitive).
pub fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("md"))
}

/// Returns `true` for directory names that the workspace scanner skips.
pub fn is_ignored_workspace_entry(name: &str) -> bool {
    name == "node_modules"
        || name == "target"
        || name == "dist"
        || name == "build"
        || name == ".git"
}

/// Recursively scan a directory into a [`WorkspaceNode`] tree.
///
/// Directories sort before files; within each group entries are sorted
/// case-insensitively by label.
pub fn scan_workspace_dir(path: &Path) -> Result<WorkspaceNode> {
    let mut children = Vec::new();
    let read_dir = fs::read_dir(path)
        .map_err(|err| anyhow::anyhow!("failed to read '{}': {err}", path.display()))?;

    for entry in read_dir {
        let Ok(entry) = entry else {
            continue;
        };
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        if is_ignored_workspace_entry(&name) {
            continue;
        }

        if entry_path.is_dir() {
            let dir_children = scan_workspace_dir(&entry_path)
                .map(|node| node.children)
                .unwrap_or_default();
            children.push(WorkspaceNode {
                id: file_node_id(&entry_path),
                label: name,
                kind: WorkspaceNodeKind::Directory(entry_path),
                children: dir_children,
            });
        } else if entry_path.is_file() {
            let kind = if is_markdown_file(&entry_path) {
                WorkspaceNodeKind::MarkdownFile(entry_path.clone())
            } else {
                WorkspaceNodeKind::File(entry_path.clone())
            };
            children.push(WorkspaceNode {
                id: file_node_id(&entry_path),
                label: name,
                kind,
                children: Vec::new(),
            });
        }
    }

    children.sort_by(|left, right| {
        let left_dir = matches!(left.kind, WorkspaceNodeKind::Directory(_));
        let right_dir = matches!(right.kind, WorkspaceNodeKind::Directory(_));
        right_dir
            .cmp(&left_dir)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });

    Ok(WorkspaceNode {
        id: file_node_id(path),
        label: file_label(path),
        kind: WorkspaceNodeKind::Directory(path.to_path_buf()),
        children,
    })
}

/// Human-readable label for a path (its final component).
pub fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Stable string id for a file node.
pub fn file_node_id(path: &Path) -> String {
    format!("file:{}", path.to_string_lossy())
}

/// Stable numeric hash of a node id, for use as a DOM element id suffix.
pub fn stable_node_hash(id: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

// ── Editor methods ────────────────────────────────────────────────────────
