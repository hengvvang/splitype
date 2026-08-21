//! Markdown and source text serialization engine for the document block tree.

use gpui::*;

use super::Document;
use crate::editor::tree::block::Block;
use crate::model::block::CalloutKind;
use crate::model::block::image::parse_standalone_image;
use crate::model::block::table::serialize_table_markdown_lines;
use crate::model::parse::BlockKind;

impl Document {
    pub(crate) fn serialize_markdown(&self, cx: &App) -> String {
        let mut lines = Vec::new();
        Self::collect_root_markdown_lines(&self.roots, cx, &mut lines);
        lines.join("\n")
    }

    pub(crate) fn serialize_source_text(&self, cx: &App) -> String {
        self.index
            .entries
            .iter()
            .map(|entries| entries.entity.read(cx).display_text().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub(crate) fn is_empty_root_paragraph(block: &Block) -> bool {
        block.kind() == BlockKind::Paragraph
            && block.data.text.plain_text().is_empty()
            && block.children.is_empty()
    }

    pub(crate) fn collect_root_markdown_lines(
        blocks: &[Entity<Block>],
        cx: &App,
        lines: &mut Vec<String>,
    ) {
        let mut pending_empty_roots = 0usize;
        let mut wrote_non_empty_root = false;
        let mut previous_was_list_item = false;

        for block in blocks {
            let block_ref = block.read(cx);
            if Self::is_empty_root_paragraph(block_ref) {
                pending_empty_roots += 1;
                continue;
            }

            let current_is_list_item = block_ref.kind().is_list_item();
            let current_is_footnote = block_ref.kind() == BlockKind::FootnoteDefinition;
            if wrote_non_empty_root {
                let separator_count = if previous_was_list_item && current_is_list_item {
                    pending_empty_roots
                } else if current_is_footnote && pending_empty_roots == 0 {
                    // Adjacent footnote definitions (or a definition directly
                    // after a paragraph) stay tight: no blank line is forced
                    // between them, so `[^a]: x\n[^b]: y` round-trips.
                    0
                } else {
                    pending_empty_roots + 1
                };
                lines.extend(std::iter::repeat_n(String::new(), separator_count));
            } else if pending_empty_roots > 0 {
                lines.extend(std::iter::repeat_n(String::new(), pending_empty_roots));
            }

            Self::collect_single_block_markdown_lines(block_ref, 0, cx, lines);
            wrote_non_empty_root = true;
            pending_empty_roots = 0;
            previous_was_list_item = current_is_list_item;
        }

        if wrote_non_empty_root {
            if pending_empty_roots > 0 {
                lines.extend(std::iter::repeat_n(String::new(), pending_empty_roots + 1));
            }
        } else if pending_empty_roots > 1 {
            lines.extend(std::iter::repeat_n(String::new(), pending_empty_roots));
        }
    }

    pub(crate) fn collect_single_block_markdown_lines(
        block_ref: &Block,
        list_depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
    ) {
        match block_ref.kind() {
            BlockKind::Table => {
                if let Some(table) = block_ref.data.table.as_ref() {
                    lines.extend(serialize_table_markdown_lines(table));
                }
            }
            BlockKind::CodeBlock { language } => {
                let indentation = "  ".repeat(list_depth);
                let lang_str = language.as_ref().map(|s| s.as_ref()).unwrap_or("");
                let fence = crate::editor::tree::serialize::safe_code_fence_with_info(
                    &block_ref.data.text.plain_text(),
                    language.as_ref().map(|language| language.as_ref()),
                );
                lines.push(format!("{indentation}{fence}{lang_str}"));
                let content = block_ref.data.text.plain_text();
                for code_line in content.split('\n') {
                    lines.push(format!("{indentation}{code_line}"));
                }
                lines.push(format!("{indentation}{fence}"));
            }
            BlockKind::Blockquote => {
                let text_markdown =
                    CalloutKind::escape_plain_quote_header(&block_ref.data.text_markdown());
                let indentation = "  ".repeat(list_depth);
                if !text_markdown.is_empty() || block_ref.children.is_empty() {
                    for line in text_markdown.split('\n') {
                        lines.push(format!("{indentation}> {line}"));
                    }
                }

                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    Self::collect_markdown_lines(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        false,
                    );
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                }
            }
            BlockKind::Callout(variant) => {
                let indentation = "  ".repeat(list_depth);
                lines.push(format!(
                    "{indentation}> {}",
                    variant.header_markdown(&block_ref.data.text_markdown())
                ));
                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    Self::collect_markdown_lines(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        false,
                    );
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                }
            }
            BlockKind::FootnoteDefinition => {
                let indentation = "  ".repeat(list_depth);
                let full_text = block_ref.data.text_markdown();
                let (id, first_line) =
                    crate::model::block::footnote::split_footnote_definition_text(&full_text);
                if first_line.is_empty() && block_ref.children.is_empty() {
                    lines.push(format!("{indentation}[^{id}]:"));
                    return;
                }

                let mut first_lines = first_line.split('\n');
                let first = first_lines.next().unwrap_or_default();
                if first.is_empty() {
                    lines.push(format!("{indentation}[^{id}]:"));
                } else {
                    lines.push(format!("{indentation}[^{id}]: {first}"));
                }
                for line in first_lines {
                    if line.is_empty() {
                        lines.push(String::new());
                    } else {
                        lines.push(format!("{indentation}    {line}"));
                    }
                }

                if !block_ref.children.is_empty() {
                    lines.push(String::new());
                    Self::collect_markdown_lines(&block_ref.children, 2, cx, lines, true);
                }
            }
            BlockKind::RawMarkdown | BlockKind::HtmlComment | BlockKind::HtmlBlock => {
                let indentation = "  ".repeat(list_depth);
                let raw_markdown = block_ref
                    .data
                    .raw_source
                    .clone()
                    .unwrap_or_else(|| block_ref.data.text_markdown());
                for line in raw_markdown.split('\n') {
                    if indentation.is_empty() {
                        lines.push(line.to_string());
                    } else {
                        lines.push(format!("{indentation}{line}"));
                    }
                }
            }
            BlockKind::BulletListItem
            | BlockKind::TaskListItem { .. }
            | BlockKind::NumberedListItem => {
                lines.push(
                    block_ref
                        .data
                        .serialize_markdown_line(list_depth, block_ref.list_ordinal),
                );
                let child_list_depth = list_depth + 1;
                for child in &block_ref.children {
                    let child_ref = child.read(cx);
                    if Self::list_child_requires_leading_blank_line(child_ref) {
                        lines.push(String::new());
                    }
                    Self::collect_single_block_markdown_lines(
                        child_ref,
                        child_list_depth,
                        cx,
                        lines,
                    );
                }
            }
            _ => {
                lines.push(
                    block_ref
                        .data
                        .serialize_markdown_line(list_depth, block_ref.list_ordinal),
                );
                let child_list_depth = list_depth + usize::from(block_ref.kind().is_list_item());
                Self::collect_markdown_lines(
                    &block_ref.children,
                    child_list_depth,
                    cx,
                    lines,
                    false,
                );
            }
        }
    }

    pub(crate) fn list_child_requires_leading_blank_line(block_ref: &Block) -> bool {
        if block_ref.kind() != BlockKind::Paragraph || !block_ref.children.is_empty() {
            return false;
        }

        let markdown = block_ref.data.text_markdown();
        !markdown.is_empty() && parse_standalone_image(&markdown).is_none()
    }

    pub(crate) fn collect_markdown_lines(
        blocks: &[Entity<Block>],
        depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
        blank_line_between_siblings: bool,
    ) {
        let mut first = true;
        let mut previous_was_list_item = false;
        for block in blocks {
            let current_is_list_item = block.read(cx).kind().is_list_item();
            if !first
                && blank_line_between_siblings
                && !(previous_was_list_item && current_is_list_item)
            {
                lines.push(String::new());
            }
            first = false;

            let block_ref = block.read(cx);
            Self::collect_single_block_markdown_lines(block_ref, depth, cx, lines);
            previous_was_list_item = current_is_list_item;
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use crate::editor::controller::Editor;
    use crate::model::parse::{BlockData, BlockKind};

    #[gpui::test]
    async fn snapshot_tracks_nested_visible_order(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "- a\n  - b\n    - c\n- d".to_string(), None));

        editor.update(cx, |editor, _cx| {
            let entries = editor.doc().blocks().to_vec();
            let a = entries[0].entity.clone();
            let b = entries[1].entity.clone();
            let c = entries[2].entity.clone();
            let d = entries[3].entity.clone();

            assert_eq!(editor.doc().index_for_entity_id(a.entity_id()), Some(0));
            assert_eq!(editor.doc().index_for_entity_id(b.entity_id()), Some(1));
            assert_eq!(editor.doc().index_for_entity_id(c.entity_id()), Some(2));
            assert_eq!(editor.doc().index_for_entity_id(d.entity_id()), Some(3));

            let c_location = editor
                .doc()
                .find_block_location(c.entity_id())
                .expect("location");
            assert_eq!(
                c_location.parent.expect("nested parent").entity_id(),
                b.entity_id()
            );
            assert_eq!(c_location.index, 0);

            assert_eq!(
                editor
                    .doc()
                    .last_descendant(a.entity_id())
                    .expect("descendant")
                    .entity_id(),
                c.entity_id()
            );
        });
    }

    #[gpui::test]
    async fn rebuild_hoists_children_from_leaf_blocks(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        editor.update(cx, |editor, cx| {
            let root = editor.doc().first_root().expect("root").clone();
            let child = Editor::new_block(cx, BlockData::paragraph("child"));

            root.update(cx, {
                let child = child.clone();
                move |root, _cx| {
                    root.children.push(child.clone());
                }
            });

            editor.doc_mut().rebuild_metadata_and_snapshot(cx);

            assert!(root.read(cx).children.is_empty());
            let visible_ids = editor
                .doc()
                .blocks()
                .iter()
                .map(|entries| entries.entity.entity_id())
                .collect::<Vec<_>>();
            assert_eq!(visible_ids, vec![root.entity_id(), child.entity_id()]);

            let location = editor
                .doc()
                .find_block_location(child.entity_id())
                .expect("child location");
            assert!(location.parent.is_none());
            assert_eq!(location.index, 1);
        });
    }

    #[gpui::test]
    async fn code_block_language_edit_serializes_to_opening_fence(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "```rust\nfn main() {}\n```".into(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().first_root().expect("code block").clone();
            block.update(cx, |block, cx| {
                let range = 0..block.code_language_text().len();
                block.replace_code_language_text_in_range(range, "unknown-lang", None, false, cx);
            });

            assert_eq!(
                editor.doc().serialize_markdown(cx),
                "```unknown-lang\nfn main() {}\n```"
            );
        });
    }

    #[gpui::test]
    async fn code_block_language_with_backtick_round_trips_with_tilde_fence(
        cx: &mut TestAppContext,
    ) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "```rust\nbody\n```".into(), None));

        let markdown = editor.update(cx, |editor, cx| {
            let block = editor.doc().first_root().expect("code block").clone();
            block.update(cx, |block, cx| {
                let range = 0..block.code_language_text().len();
                block.replace_code_language_text_in_range(range, "we`rd", None, false, cx);
            });
            editor.doc().serialize_markdown(cx)
        });

        assert_eq!(markdown, "~~~we`rd\nbody\n~~~");

        let round_tripped = cx.new(|cx| Editor::from_markdown(cx, markdown, None));
        round_tripped.update(cx, |editor, cx| {
            let block = editor.doc().first_root().expect("code block");
            assert_eq!(block.read(cx).code_language_text(), "we`rd");
            assert!(matches!(block.read(cx).kind(), BlockKind::CodeBlock { .. }));
        });
    }

    #[gpui::test]
    async fn structure_mutation_rebuilds_snapshot_after_relocation(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "- a\n- b\n- c".to_string(), None));

        editor.update(cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let a = entries[0].entity.clone();
            let b = entries[1].entity.clone();
            let c = entries[2].entity.clone();

            editor
                .doc_mut()
                .with_structure_mutation(cx, |document, cx| {
                    let moved = document
                        .remove_block_by_id_raw(c.entity_id(), cx)
                        .expect("remove c")
                        .0;
                    document.insert_blocks_at_raw(
                        Some(a.clone()),
                        a.read(cx).children.len(),
                        vec![moved],
                        cx,
                    );
                });

            assert_eq!(editor.doc().index_for_entity_id(a.entity_id()), Some(0));
            assert_eq!(editor.doc().index_for_entity_id(c.entity_id()), Some(1));
            assert_eq!(editor.doc().index_for_entity_id(b.entity_id()), Some(2));

            let c_location = editor
                .doc()
                .find_block_location(c.entity_id())
                .expect("c location");
            assert_eq!(
                c_location.parent.expect("nested parent").entity_id(),
                a.entity_id()
            );
            assert_eq!(c_location.index, 0);

            assert_eq!(
                editor
                    .doc()
                    .last_descendant(a.entity_id())
                    .expect("descendant")
                    .entity_id(),
                c.entity_id()
            );
        });
    }
}
