//! Cross-crate contract tests for the Markdown domain layer.
//!
//! These exercise `splitype-model` through its public API the way an
//! external consumer would, verifying that parsing produces the expected
//! block tree and that inline formatting survives serialization.

use splitype_model::parse::kind::BlockKind;
use splitype_model::parse::data::BlockData;
use splitype_model::inline::text::BlockText;
use splitype_model::parse::parser::parse_document;
use splitype_model::block::table::TableData;

fn roots(markdown: &str) -> Vec<BlockData> {
    parse_document(markdown)
}

/// Heading, paragraph, and thematic break parse into distinct root blocks.
#[test]
fn parses_headings_paragraphs_and_rules() {
    let blocks = roots("# Title\n\nbody text\n\n---\n");
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].kind, BlockKind::Heading { level: 1 });
    assert_eq!(blocks[0].text.plain_text(), "Title");
    assert_eq!(blocks[1].kind, BlockKind::Paragraph);
    assert_eq!(blocks[1].text.plain_text(), "body text");
    assert_eq!(blocks[2].kind, BlockKind::ThematicBreak);
}

/// Parent-child relationships are reconstructed from indentation.
#[test]
fn nested_list_children_are_linked_to_their_parent() {
    let blocks = roots("- parent\n  - child\n    - grandchild\n");
    let parent = blocks
        .iter()
        .find(|block| block.kind.is_list_item() && block.text.plain_text() == "parent")
        .expect("parent list item");
    assert_eq!(parent.children.len(), 1);

    let child = blocks
        .iter()
        .find(|block| block.text.plain_text() == "child")
        .expect("child list item");
    assert_eq!(child.parent, Some(parent.id));
    assert_eq!(child.children.len(), 1);

    let grandchild = blocks
        .iter()
        .find(|block| block.text.plain_text() == "grandchild")
        .expect("grandchild list item");
    assert_eq!(grandchild.parent, Some(child.id));
    assert!(grandchild.children.is_empty());
}

/// Nested quotes keep their nesting as parent-child chains.
#[test]
fn nested_quotes_chain_as_parents_and_children() {
    let blocks = roots("> level1\n> > level2\n> > > level3\n");
    let level1 = blocks
        .iter()
        .find(|block| block.kind == BlockKind::Blockquote && block.text.plain_text() == "level1")
        .expect("level1 quote");
    assert_eq!(level1.children.len(), 1);

    let level2 = blocks
        .iter()
        .find(|block| block.text.plain_text() == "level2")
        .expect("level2 quote");
    assert_eq!(level2.parent, Some(level1.id));

    let level3 = blocks
        .iter()
        .find(|block| block.text.plain_text() == "level3")
        .expect("level3 quote");
    assert_eq!(level3.parent, Some(level2.id));
}

/// Fenced code blocks keep their language info string.
#[test]
fn fenced_code_block_preserves_language() {
    let blocks = roots("```rust\nfn main() {}\n```\n");
    assert_eq!(blocks.len(), 1);
    let BlockKind::CodeBlock { language } = &blocks[0].kind else {
        panic!("expected code block, got {:?}", blocks[0].kind);
    };
    assert_eq!(language.as_deref().map(AsRef::as_ref), Some("rust"));
    assert_eq!(blocks[0].text.plain_text(), "fn main() {}");
}

/// Tables parse into native `TableData` with header and rows.
#[test]
fn table_parses_into_native_table_data() {
    let blocks = roots("| a | b |\n|---|---|\n| 1 | 2 |\n");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].kind, BlockKind::Table);
    let table: &TableData = blocks[0].table.as_ref().expect("table data");
    assert_eq!(table.header.len(), 2);
    assert_eq!(table.rows.len(), 1);
}

/// Inline formatting round-trips through markdown serialization.
#[test]
fn inline_formatting_round_trips() {
    let text = BlockText::from_markdown("**bold** and *italic* and `code`");
    assert_eq!(
        text.serialize_markdown(),
        "**bold** and *italic* and `code`"
    );
    assert!(!text.plain_text().is_empty());
}

/// Raw HTML blocks preserve their source.
#[test]
fn raw_html_block_keeps_source() {
    let blocks = roots("<div class=\"note\">hi</div>\n");
    let html = blocks
        .iter()
        .find(|block| block.kind == BlockKind::HtmlBlock)
        .expect("html block");
    assert_eq!(
        html.raw_source.as_deref(),
        Some("<div class=\"note\">hi</div>")
    );
}

/// A constructed block serializes its text back to markdown.
#[test]
fn block_text_serializes_serialize_markdown() {
    let block = BlockData::new(
        BlockKind::Paragraph,
        BlockText::from_markdown("hello **world**"),
    );
    assert_eq!(block.text_markdown(), "hello **world**");
    assert_eq!(block.serialize_markdown_line(0, None), "hello **world**");
}
