//! Source code pane outline extraction and heading navigation.

use editor_outline::OutlineNode;
use crate::SourceCodeState;

/// Extracts all heading nodes from the raw Markdown source text.
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
                id: format!("outline:line:{line_idx}"),
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
                    id: format!("outline:line:{line_idx}"),
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

/// Navigates the source code buffer to the line specified in the outline node.
pub fn navigate_to_node(state: &mut SourceCodeState, node: &OutlineNode) {
    let line_start = state.line_start_offset(node.block_index);
    state.cursor = line_start;
    state.selection = None;
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

    #[test]
    fn extracts_atx_and_setext_headings() {
        let md = "# Title 1\n\nSome paragraph\n\n## Subtitle\n\nSetext Title\n===\n\n```rust\n# Not a heading\n```";
        let headings = extract_outline_headings(md);
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].label, "Title 1");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[1].label, "Subtitle");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[2].label, "Setext Title");
        assert_eq!(headings[2].level, 1);
    }
}
