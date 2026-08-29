#[cfg(test)]
mod tests {
    use crate::parse::parser::{build_wysiwyg_blocks_from_lines, parse_wysiwyg_document};
    use crate::parse::BlockKind;
    use crate::tree::SumTree;

    #[test]
    fn test_markdown_heading_parsing() {
        let markdown = "# Heading 1\n## Heading 2\n### Heading 3";
        let blocks = parse_wysiwyg_document(markdown);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].kind, BlockKind::Heading { level: 1 });
        assert_eq!(blocks[1].kind, BlockKind::Heading { level: 2 });
        assert_eq!(blocks[2].kind, BlockKind::Heading { level: 3 });
        assert_eq!(blocks[0].text.plain_text(), "Heading 1");
    }

    #[test]
    fn test_markdown_lists_and_checkboxes() {
        let markdown = "- Item 1\n- [ ] Task incomplete\n- [x] Task done\n1. Numbered item";
        let blocks = parse_wysiwyg_document(markdown);
        assert_eq!(blocks.len(), 4);
        assert!(matches!(blocks[0].kind, BlockKind::BulletListItem));
        assert!(matches!(blocks[1].kind, BlockKind::TaskListItem { checked: false }));
        assert!(matches!(blocks[2].kind, BlockKind::TaskListItem { checked: true }));
        assert!(matches!(blocks[3].kind, BlockKind::NumberedListItem));
    }

    #[test]
    fn test_markdown_fenced_code_and_mermaid_math() {
        let markdown = "```rust\nfn main() {}\n```\n$$\nx^2 + y^2 = z^2\n$$\n```mermaid\ngraph TD;\nA-->B;\n```";
        let blocks = parse_wysiwyg_document(markdown);
        assert!(blocks.iter().any(|b| matches!(b.kind, BlockKind::CodeBlock { .. })));
        assert!(blocks.iter().any(|b| matches!(b.kind, BlockKind::MathBlock)));
        assert!(blocks.iter().any(|b| matches!(b.kind, BlockKind::MermaidBlock)));
    }

    #[test]
    fn test_line_based_wysiwyg_block_builder() {
        let lines = vec![
            "# Title".to_string(),
            "".to_string(),
            "Paragraph text with **bold** and *italic*.".to_string(),
        ];
        let blocks = build_wysiwyg_blocks_from_lines(&lines);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].kind, BlockKind::Heading { level: 1 });
        assert_eq!(blocks[1].kind, BlockKind::Paragraph);
        assert_eq!(blocks[2].kind, BlockKind::Paragraph);
    }

    #[test]
    fn test_sum_tree_indexing() {
        let mut tree = SumTree::new();
        let markdown = "# Hello\nWorld\n```rust\nfn foo() {}\n```";
        let blocks = parse_wysiwyg_document(markdown);
        for block in blocks {
            tree.push(block, &());
        }
        assert!(!tree.is_empty());
        assert_eq!(tree.len(&()), 3);
    }
}
