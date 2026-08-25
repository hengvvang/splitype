//! Outline panel — heading tree derived from document blocks or markdown source.

pub(crate) mod render;
pub(crate) mod state;

use std::collections::HashSet;

use gpui::App;

use crate::editor::document::Document;
use crate::model::parse::BlockKind;

use state::{OutlineNode, OutlineNodeKind, OutlinePaneState};

/// Prune expanded-node state and selection that no longer exist in the
/// current outline tree.
pub(crate) fn prune_outline_state(outline: &mut OutlinePaneState, nodes: &[OutlineNode]) {
    let mut current_ids = HashSet::new();
    collect_node_ids(nodes, &mut current_ids);
    outline.expanded.retain(|id| current_ids.contains(id));

    if matches!(
        &outline.selected,
        Some(id) if !current_ids.contains(id)
    ) {
        outline.selected = None;
    }
}

/// Collect all node ids from a tree (recursively) into `ids`.
pub(crate) fn collect_node_ids(nodes: &[OutlineNode], ids: &mut HashSet<String>) {
    for node in nodes {
        ids.insert(node.id.clone());
        collect_node_ids(&node.children, ids);
    }
}

/// Build an outline tree directly from the active Document's block entities.
///
/// This avoids redundant string serialization/reparsing and automatically
/// supports all heading syntaxes (ATX and Setext) while preserving direct
/// entity references for instant jump navigation.
pub(crate) fn build_outline_tree_from_doc(doc: &Document, cx: &App) -> Vec<OutlineNode> {
    let mut roots = Vec::new();
    let mut stack: Vec<(u8, Vec<usize>)> = Vec::new();

    for (block_index, entry) in doc.blocks().iter().enumerate() {
        let block = entry.entity.read(cx);
        let BlockKind::Heading { level } = block.kind() else {
            continue;
        };

        let heading_text = block.data.text.plain_text().trim().to_string();
        let entity_id = entry.entity.entity_id();

        while stack
            .last()
            .is_some_and(|(parent_level, _)| *parent_level >= level)
        {
            stack.pop();
        }

        let node = OutlineNode {
            id: format!("outline:{}", entity_id),
            label: heading_text,
            kind: OutlineNodeKind::Heading {
                line: block_index,
                level,
                block_id: Some(entity_id),
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

/// Parse a Markdown document into an outline tree (headings only).
///
/// Supports both ATX headings (`# Title`) and Setext headings (`Title\n===`).
/// Code-fence content is skipped so headings inside fenced blocks are not
/// included in the outline.
#[cfg(test)]
pub(crate) fn build_outline_tree(markdown: &str) -> Vec<OutlineNode> {
    let mut roots = Vec::new();
    let mut stack: Vec<(u8, Vec<usize>)> = Vec::new();
    let mut fence: Option<(char, usize)> = None;

    let lines: Vec<&str> = markdown.lines().collect();
    let mut line_index = 0;

    while line_index < lines.len() {
        let line = lines[line_index];
        let trimmed = line.trim_start();

        if let Some((marker, len)) = fence {
            if is_closing_fence(trimmed, marker, len) {
                fence = None;
            }
            line_index += 1;
            continue;
        }

        if let Some(next_fence) = opening_fence(trimmed) {
            fence = Some(next_fence);
            line_index += 1;
            continue;
        }

        // 1. Try ATX heading: `# Heading`
        if let Some((level, heading_text)) = BlockKind::parse_atx_heading_line(line) {
            insert_outline_node(
                &mut roots,
                &mut stack,
                line_index,
                level,
                heading_text,
                None,
            );
            line_index += 1;
            continue;
        }

        // 2. Try Setext heading: `Heading Line\n===` or `Heading Line\n---`
        if line_index + 1 < lines.len() && !trimmed.is_empty() {
            let next_line = lines[line_index + 1].trim_start();
            if let Some(level) = BlockKind::parse_setext_underline(next_line) {
                insert_outline_node(
                    &mut roots,
                    &mut stack,
                    line_index,
                    level,
                    trimmed.to_string(),
                    None,
                );
                line_index += 2;
                continue;
            }
        }

        line_index += 1;
    }

    roots
}

#[cfg(test)]
fn insert_outline_node(
    roots: &mut Vec<OutlineNode>,
    stack: &mut Vec<(u8, Vec<usize>)>,
    line: usize,
    level: u8,
    label: String,
    block_id: Option<gpui::EntityId>,
) {
    while stack
        .last()
        .is_some_and(|(parent_level, _)| *parent_level >= level)
    {
        stack.pop();
    }

    let node = OutlineNode {
        id: format!("outline:{line}"),
        label,
        kind: OutlineNodeKind::Heading {
            line,
            level,
            block_id,
        },
        children: Vec::new(),
    };

    let siblings = if let Some((_, parent_path)) = stack.last() {
        children_at_path_mut(roots, parent_path)
    } else {
        roots
    };
    siblings.push(node);

    let mut node_path = stack
        .last()
        .map(|(_, path)| path.clone())
        .unwrap_or_default();
    node_path.push(siblings.len() - 1);
    stack.push((level, node_path));
}

/// Navigate to a child list at the given index path.
fn children_at_path_mut<'a>(
    nodes: &'a mut Vec<OutlineNode>,
    path: &[usize],
) -> &'a mut Vec<OutlineNode> {
    let mut current = nodes;
    for &index in path {
        current = &mut current[index].children;
    }
    current
}

/// Detect an opening code fence.
#[cfg(test)]
fn opening_fence(trimmed: &str) -> Option<(char, usize)> {
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    (len >= 3).then_some((marker, len))
}

/// Detect a closing code fence.
#[cfg(test)]
fn is_closing_fence(trimmed: &str, marker: char, len: usize) -> bool {
    let count = trimmed.chars().take_while(|ch| *ch == marker).count();
    count >= len && trimmed[count..].trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_atx_headings() {
        let md = "# Chapter 1
## Section 1.1
### Subsection 1.1.1
## Section 1.2
# Chapter 2";
        let tree = build_outline_tree(md);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].label, "Chapter 1");
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].label, "Section 1.1");
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children[0].label, "Subsection 1.1.1");
        assert_eq!(tree[0].children[1].label, "Section 1.2");
        assert_eq!(tree[1].label, "Chapter 2");
    }

    #[test]
    fn parses_setext_headings() {
        let md = "Main Title
==========
Subtitle
--------
Paragraph text";
        let tree = build_outline_tree(md);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].label, "Main Title");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].label, "Subtitle");
    }

    #[test]
    fn ignores_headings_inside_code_fences() {
        let md = "# Real Heading
```markdown
# Fake Heading in Code
```
## Real Subheading";
        let tree = build_outline_tree(md);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].label, "Real Heading");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].label, "Real Subheading");
    }
}
