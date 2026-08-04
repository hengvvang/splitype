//! Outline panel — heading tree derived from the document source.

pub(crate) mod render;

use std::collections::HashSet;

use crate::editor::windows::layout::workspace::{Workspace, WorkspaceNode, WorkspaceNodeKind, WorkspaceSelection};
use crate::model::block::BlockKind;

/// Prune expanded-node state and selection that no longer exist in the
/// current outline tree.
pub(crate) fn prune_outline_state(workspace: &mut Workspace, outline: &[WorkspaceNode]) {
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
pub(crate) fn collect_node_ids(nodes: &[WorkspaceNode], ids: &mut HashSet<String>) {
    for node in nodes {
        ids.insert(node.id.clone());
        collect_node_ids(&node.children, ids);
    }
}

/// Returns `true` when `id` is an outline-node id (starts with "outline:").
pub(crate) fn is_outline_node_id(id: &str) -> bool {
    id.starts_with("outline:")
}

/// Parse a Markdown document into an outline tree (headings only).
///
/// Code-fence content is skipped so headings inside fenced blocks are not
/// included in the outline.
pub(crate) fn build_outline_tree(markdown: &str) -> Vec<WorkspaceNode> {
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
