use editor_contracts::OutlineNode;
use markdown_parser::parse::BlockKind;
use syntax_highlighter::language::CodeLanguageKey;

/// Extracts an outline from the buffer text based on the document's language.
/// Markdown files extract headings; code files extract Tree-Sitter AST symbols.
pub fn extract_outline(language: Option<CodeLanguageKey>, text: &str) -> Vec<OutlineNode> {
    match language {
        Some(CodeLanguageKey::Markdown) | None => extract_outline_headings(text),
        Some(lang) => {
            let symbols = syntax_highlighter::extract_symbols(lang, text);
            if symbols.is_empty() {
                extract_outline_headings(text)
            } else {
                symbols
            }
        }
    }
}

/// Extracts all heading nodes from the raw Markdown source text. Heading
/// line recognition is delegated to the canonical Markdown parser helpers.
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
        if let Some((level, content)) = BlockKind::parse_atx_heading_line(line) {
            let label = content.trim().to_string();
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
                kind: editor_contracts::OutlineNodeKind::Heading,
            });
            line_idx += 1;
            continue;
        }

        // Setext heading: `Heading Line\n===` or `Heading Line\n---`
        if line_idx + 1 < lines.len() && !trimmed.is_empty() {
            let next_line = lines[line_idx + 1];
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
                    kind: editor_contracts::OutlineNodeKind::Heading,
                });
                line_idx += 2;
                continue;
            }
        }

        line_idx += 1;
    }
    list
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

    #[test]
    fn extracts_symbols_when_language_specified() {
        let rust_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }\nstruct Point { x: i32 }";
        let nodes = extract_outline(Some(CodeLanguageKey::Rust), rust_code);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].label, "add");
        assert_eq!(nodes[0].kind, editor_contracts::OutlineNodeKind::Function);
        assert_eq!(nodes[1].label, "Point");
        assert_eq!(nodes[1].kind, editor_contracts::OutlineNodeKind::Struct);

        // Markdown fallback when None
        let md = "# Doc Title";
        let md_nodes = extract_outline(None, md);
        assert_eq!(md_nodes.len(), 1);
        assert_eq!(md_nodes[0].label, "Doc Title");
    }
}
