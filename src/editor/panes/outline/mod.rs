//! Outline HUD — Notion-style floating outline ticks rail and hover TOC popover.

pub(crate) mod render;
pub(crate) mod state;

use gpui::App;

use crate::editor::document::Document;
use crate::model::parse::BlockKind;

#[allow(unused_imports)]
pub(crate) use state::{OutlineHudState, OutlineNode};

/// Extract all heading items from raw Markdown text.
pub(crate) fn build_outline_headings_from_markdown(markdown: &str) -> Vec<OutlineNode> {
    let mut list = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_fence = false;
    let mut fence_char = '`';
    let mut fence_len = 3;

    let mut line_idx = 0;
    while line_idx < lines.len() {
        let line = lines[line_idx];
        let trimmed = line.trim_start();

        // Handle code fences so headings inside ``` aren't parsed
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

        // 1. Try ATX heading: `# Heading`
        if let Some((level, raw_text)) = BlockKind::parse_atx_heading_line(line) {
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

        // 2. Try Setext heading: `Heading Line\n===` or `Heading Line\n---`
        if line_idx + 1 < lines.len() && !trimmed.is_empty() {
            let next_line = lines[line_idx + 1].trim_start();
            if let Some(level) = BlockKind::parse_setext_underline(next_line) {
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

/// Extract all heading items from the active Document block entities.
pub(crate) fn build_outline_headings_from_doc(doc: &Document, cx: &App) -> Vec<OutlineNode> {
    let mut list = Vec::new();
    for (block_index, entry) in doc.blocks().iter().enumerate() {
        let block = entry.entity.read(cx);
        let level = match block.kind() {
            BlockKind::Heading { level } => level,
            _ => {
                let text = block.data.text.plain_text();
                if let Some((lvl, _)) = BlockKind::parse_atx_heading_line(text.trim()) {
                    lvl
                } else {
                    continue;
                }
            }
        };

        let heading_text = block.data.text.plain_text().trim().to_string();
        let entity_id = entry.entity.entity_id();
        let display_label = if let Some((_, parsed_text)) = BlockKind::parse_atx_heading_line(&heading_text) {
            parsed_text
        } else {
            heading_text
        };

        list.push(OutlineNode {
            id: format!("outline:{entity_id}"),
            label: if display_label.is_empty() {
                format!("Heading {level}")
            } else {
                display_label
            },
            level,
            block_index,
            block_id: Some(entity_id),
        });
    }
    list
}

