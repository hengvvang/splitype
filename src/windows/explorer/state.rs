//! ExplorerState sidebar model — file-tree scanning and outline state.
//!
//! This module owns the pure explorer model: [`ExplorerNodeKind`],
//! [`ExplorerNode`], [`ExplorerSelection`], and [`ExplorerState`], plus the
//! pure functions that scan directories. Outline parsing lives in
//! `crate::editor::panels::outline`; rendering and editor interactions stay
//! in `ui::window::explorer_view`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

// ── Icons & constants ───────────────────────────────────────────────────

pub const FOLDER_ICON: &str = "icon/explorer/folder.svg";
pub const MARKDOWN_ICON: &str = "icon/explorer/markdown.svg";
pub const FILE_ICON: &str = "icon/explorer/file.svg";
pub const EXPLORER_NODE_HEIGHT: f32 = 28.0;
pub const EXPLORER_NODE_INDENT: f32 = 14.0;

// ── Data types ──────────────────────────────────────────────────────────

/// What kind of filesystem or outline entry a tree node represents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerNodeKind {
    Directory(PathBuf),
    MarkdownFile(PathBuf),
    File(PathBuf),
    Heading { line: usize, level: u8 },
}

/// A node in the explorer file tree or outline tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerNode {
    pub id: String,
    pub label: String,
    pub kind: ExplorerNodeKind,
    pub children: Vec<ExplorerNode>,
}

/// Which item is currently selected in the explorer sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerSelection {
    File(PathBuf),
    Outline(String),
}

/// Top-level explorer sidebar state.
#[derive(Default)]
pub struct ExplorerState {
    pub is_open: bool,
    pub root: Option<PathBuf>,
    pub file_tree: Option<ExplorerNode>,
    pub file_error: Option<String>,
    pub outline_tree: Vec<ExplorerNode>,
    pub outline_source: Option<String>,
    pub expanded: HashSet<String>,
    pub selected: Option<ExplorerSelection>,
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

/// Recursively scan a directory into a [`ExplorerNode`] tree.
///
/// Directories sort before files; within each group entries are sorted
/// case-insensitively by label.
pub fn scan_explorer_dir(path: &Path) -> Result<ExplorerNode> {
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

        if entry_path.is_dir() {
            let dir_children = scan_explorer_dir(&entry_path)
                .map(|node| node.children)
                .unwrap_or_default();
            children.push(ExplorerNode {
                id: file_node_id(&entry_path),
                label: name,
                kind: ExplorerNodeKind::Directory(entry_path),
                children: dir_children,
            });
        } else if entry_path.is_file() {
            let kind = if is_markdown_file(&entry_path) {
                ExplorerNodeKind::MarkdownFile(entry_path.clone())
            } else {
                ExplorerNodeKind::File(entry_path.clone())
            };
            children.push(ExplorerNode {
                id: file_node_id(&entry_path),
                label: name,
                kind,
                children: Vec::new(),
            });
        }
    }

    children.sort_by(|left, right| {
        let left_dir = matches!(left.kind, ExplorerNodeKind::Directory(_));
        let right_dir = matches!(right.kind, ExplorerNodeKind::Directory(_));
        right_dir
            .cmp(&left_dir)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });

    Ok(ExplorerNode {
        id: file_node_id(path),
        label: file_label(path),
        kind: ExplorerNodeKind::Directory(path.to_path_buf()),
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
