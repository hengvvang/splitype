//! Outline heading extraction — pure `outline_headings_from_markdown`.

use crate::{outline_headings_from_markdown, parse_atx_heading_line, parse_setext_underline};

#[test]
fn atx_headings_parse_levels_and_content() {
    assert_eq!(
        parse_atx_heading_line("## hello"),
        Some((2, "hello".to_string()))
    );
    assert_eq!(
        parse_atx_heading_line("  ### title ######"),
        Some((3, "title".to_string()))
    );
    assert_eq!(parse_atx_heading_line("#"), Some((1, String::new())));
    assert_eq!(parse_atx_heading_line("## "), Some((2, String::new())));
    assert_eq!(parse_atx_heading_line("### ###"), Some((3, String::new())));
    assert_eq!(
        parse_atx_heading_line("#\ttitle with tab"),
        Some((1, "title with tab".to_string()))
    );
    // Too many leading spaces is indented code, not a heading.
    assert_eq!(parse_atx_heading_line("    # not a heading"), None);
    // No space after the hashes is not ATX.
    assert_eq!(parse_atx_heading_line("#nospace"), None);
}

#[test]
fn setext_underlines_parse_levels() {
    assert_eq!(parse_setext_underline("==="), Some(1));
    assert_eq!(parse_setext_underline("="), Some(1));
    assert_eq!(parse_setext_underline("---"), Some(2));
    assert_eq!(parse_setext_underline("-"), Some(2));
    assert_eq!(parse_setext_underline("- - -"), None);
    assert_eq!(parse_setext_underline(""), None);
}

#[test]
fn outline_extracts_atx_and_setext_headings() {
    let markdown = "# Title\n\nSome paragraph.\n\n## Section\n\n### Sub\n\nText\n===\n";
    let items = outline_headings_from_markdown(markdown);
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].label, "Title");
    assert_eq!(items[0].level, 1);
    assert_eq!(items[0].block_index, 0);
    assert_eq!(items[1].label, "Section");
    assert_eq!(items[1].level, 2);
    assert_eq!(items[2].label, "Sub");
    assert_eq!(items[2].level, 3);
    // Setext H1 from the `Text\n===` pair.
    assert_eq!(items[3].label, "Text");
    assert_eq!(items[3].level, 1);
}

#[test]
fn outline_skips_headings_inside_fenced_code() {
    let markdown = "# Real\n\n```md\n# Not a heading\n```\n\n## Also real\n";
    let items = outline_headings_from_markdown(markdown);
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, vec!["Real", "Also real"]);
}

#[test]
fn outline_empty_heading_gets_placeholder_label() {
    let items = outline_headings_from_markdown("#\n");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "Heading 1");
}
