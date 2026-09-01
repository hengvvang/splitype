//! Preview pane outline extraction and heading navigation.

use crate::node::PreviewBlock;
use crate::state::PreviewState;
use editor_contracts::OutlineNode;
use markdown_parser::parse::BlockKind;

/// Extracts all heading nodes directly from the snapshot preview blocks.
pub fn extract_preview_headings(blocks: &[PreviewBlock]) -> Vec<OutlineNode> {
    let mut list = Vec::new();
    for (block_index, block) in blocks.iter().enumerate() {
        let level = match block.kind() {
            BlockKind::Heading { level } => level,
            _ => continue,
        };
        let label = block.display_text().trim().to_string();
        list.push(OutlineNode {
            id: format!("outline:preview:block:{block_index}"),
            label: if label.is_empty() {
                format!("Heading {level}")
            } else {
                label
            },
            level,
            block_index,
            block_id: None,
        });
    }
    list
}

/// Extracts all heading nodes from the serialized Markdown document (fallback/legacy).
pub fn extract_outline_headings(markdown: &str) -> Vec<OutlineNode> {
    let mut list = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_fence = false;
    let mut fence_char = '`';
    let mut fence_len = 3;

    let mut line_idx = 0;
    while line_idx < lines.len() {
        let line = lines[line_idx];
        let trimmed = line.trim_start();

        if in_fence {
            if trimmed.starts_with(fence_char) {
                let count = trimmed.chars().take_while(|&c| c == fence_char).count();
                if count >= fence_len && trimmed[count..].trim().is_empty() {
                    in_fence = false;
                }
            }
            line_idx += 1;
            continue;
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence_char = trimmed.chars().next().unwrap_or('`');
            fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
            in_fence = true;
            line_idx += 1;
            continue;
        }

        // ATX heading: `# Heading`
        if let Some((level, raw_text)) = parse_atx_heading_line(line) {
            let label = raw_text.trim().to_string();
            list.push(OutlineNode {
                id: format!("outline:preview:line:{line_idx}"),
                label: if label.is_empty() {
                    format!("Heading {level}")
                } else {
                    label
                },
                level,
                block_index: line_idx,
                block_id: None,
            });
            line_idx += 1;
            continue;
        }

        // Setext heading: `Heading Line\n===` or `Heading Line\n---`
        if line_idx + 1 < lines.len() && !trimmed.is_empty() {
            let next_line = lines[line_idx + 1].trim_start();
            if let Some(level) = parse_setext_underline(next_line) {
                let label = trimmed.to_string();
                list.push(OutlineNode {
                    id: format!("outline:preview:line:{line_idx}"),
                    label: if label.is_empty() {
                        format!("Heading {level}")
                    } else {
                        label
                    },
                    level,
                    block_index: line_idx,
                    block_id: None,
                });
                line_idx += 2;
                continue;
            }
        }

        line_idx += 1;
    }
    list
}

/// Calculates approximate scroll Y offset for the given outline node in the Preview render tree.
pub fn calculate_scroll_offset_for_node(
    state: &PreviewState,
    node: &OutlineNode,
    line_height: f32,
) -> f32 {
    let mut y = 0.0;
    for (i, block) in state.blocks.iter().enumerate() {
        if i >= node.block_index {
            break;
        }
        let est_h = match block.kind() {
            BlockKind::Heading { level } => match level {
                1 => line_height * 2.2 + 16.0,
                2 => line_height * 1.8 + 14.0,
                3 => line_height * 1.5 + 12.0,
                _ => line_height * 1.3 + 10.0,
            },
            BlockKind::Paragraph => {
                let lines = (block.display_len() / 60).max(1);
                (lines as f32) * line_height + 12.0
            }
            BlockKind::CodeBlock { .. } => {
                let lines = block.display_text().lines().count().max(1);
                (lines as f32) * line_height + 24.0
            }
            BlockKind::Table => line_height * 4.0 + 16.0,
            BlockKind::ThematicBreak => line_height * 1.0 + 8.0,
            _ => line_height * 1.5 + 8.0,
        };
        y += est_h;
    }
    y
}

fn parse_atx_heading_line(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&level) {
        let rest = &trimmed[level..];
        if rest.starts_with(' ') || rest.starts_with('\t') || rest.is_empty() {
            return Some((level as u8, rest.trim()));
        }
    }
    None
}

fn parse_setext_underline(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().all(|c| c == '=') && trimmed.len() >= 3 {
        Some(1)
    } else if trimmed.chars().all(|c| c == '-') && trimmed.len() >= 3 {
        Some(2)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown_parser::parse::BlockData;

    #[test]
    fn extracts_preview_headings_from_blocks() {
        let blocks = vec![
            PreviewBlock::new(BlockData::with_plain_text(
                BlockKind::Heading { level: 1 },
                "Main Header",
            )),
            PreviewBlock::new(BlockData::paragraph("Some body text")),
            PreviewBlock::new(BlockData::with_plain_text(
                BlockKind::Heading { level: 3 },
                "Sub Header",
            )),
        ];
        let headings = extract_preview_headings(&blocks);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].label, "Main Header");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].block_index, 0);
        assert_eq!(headings[1].label, "Sub Header");
        assert_eq!(headings[1].level, 3);
        assert_eq!(headings[1].block_index, 2);
    }

    #[test]
    fn extracts_preview_headings() {
        let md = "# Main Header\n\nBody text\n\n### Sub Header";
        let headings = extract_outline_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].label, "Main Header");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[1].label, "Sub Header");
        assert_eq!(headings[1].level, 3);
    }
}
