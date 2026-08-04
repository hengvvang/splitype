//! Workspace sidebar model — file-tree scanning and outline parsing.
//!
//! This module owns the pure workspace model: [`WorkspaceNodeKind`],
//! [`WorkspaceNode`], [`WorkspaceSelection`], and [`Workspace`], plus the
//! pure functions that scan directories and build outline trees.
//! Rendering and editor interactions stay in `ui::views::workspace_view`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::model::block::BlockKind;

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

// ── Outline helpers ─────────────────────────────────────────────────────

/// Prune expanded-node state and selection that no longer exist in the
/// current outline tree.
pub fn prune_outline_state(workspace: &mut Workspace, outline: &[WorkspaceNode]) {
    let mut current_ids = HashSet::new();
    collect_node_ids(outline, &mut current_ids);
    workspace
        .expanded
        .retain(|id| !is_outline_node_id(id) || current_ids.contains(id));

    if matches!(
        &workspace.selected,
        Some(WorkspaceSelection::Outline(id)) if !current_ids.contains(id)
    ) {
        workspace.selected = None;
    }
}

/// Collect all node ids from a tree (recursively) into `ids`.
pub fn collect_node_ids(nodes: &[WorkspaceNode], ids: &mut HashSet<String>) {
    for node in nodes {
        ids.insert(node.id.clone());
        collect_node_ids(&node.children, ids);
    }
}

/// Returns `true` when `id` is an outline-node id (starts with "outline:").
pub fn is_outline_node_id(id: &str) -> bool {
    id.starts_with("outline:")
}

/// Parse a Markdown document into an outline tree (headings only).
///
/// Code-fence content is skipped so headings inside fenced blocks are not
/// included in the outline.
pub fn build_outline_tree(markdown: &str) -> Vec<WorkspaceNode> {
    let mut roots = Vec::new();
    let mut stack: Vec<(u8, Vec<usize>)> = Vec::new();
    let mut fence: Option<(char, usize)> = None;

    for (line_index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some((marker, len)) = fence {
            if is_closing_fence(trimmed, marker, len) {
                fence = None;
            }
            continue;
        }

        if let Some(next_fence) = opening_fence(trimmed) {
            fence = Some(next_fence);
            continue;
        }

        let Some((level, title)) = BlockKind::parse_atx_heading_line(line) else {
            continue;
        };

        while stack
            .last()
            .is_some_and(|(parent_level, _)| *parent_level >= level)
        {
            stack.pop();
        }

        let node = WorkspaceNode {
            id: format!("outline:{line_index}"),
            label: title,
            kind: WorkspaceNodeKind::Heading {
                line: line_index,
                level,
            },
            children: Vec::new(),
        };

        let siblings = if let Some((_, parent_path)) = stack.last() {
            children_at_path_mut(&mut roots, parent_path)
        } else {
            &mut roots
        };
        siblings.push(node);

        let mut node_path = stack
            .last()
            .map(|(_, path)| path.clone())
            .unwrap_or_default();
        node_path.push(siblings.len() - 1);
        stack.push((level, node_path));
    }

    roots
}

/// Navigate to a child list at the given index path.
fn children_at_path_mut<'a>(
    nodes: &'a mut Vec<WorkspaceNode>,
    path: &[usize],
) -> &'a mut Vec<WorkspaceNode> {
    let mut current = nodes;
    for &index in path {
        current = &mut current[index].children;
    }
    current
}

/// Detect an opening code fence.
fn opening_fence(trimmed: &str) -> Option<(char, usize)> {
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    (len >= 3).then_some((marker, len))
}

/// Detect a closing code fence.
fn is_closing_fence(trimmed: &str, marker: char, len: usize) -> bool {
    let count = trimmed.chars().take_while(|ch| *ch == marker).count();
    count >= len && trimmed[count..].trim().is_empty()
}

// ── Editor methods ────────────────────────────────────────────────────────
