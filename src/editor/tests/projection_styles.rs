//! Inline projection keeps delimiter styles while editing.

use gpui::{AppContext, TestAppContext};

use crate::editor::controller::Editor;
use crate::model::inline::style::InlineScript;
use crate::model::parse::BlockKind;

#[gpui::test]
async fn script_delimiters_keep_script_style_while_editing(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "H~2~O 和 x^2^".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root paragraph").clone();
        block.update(cx, |block, _cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.rebuild_inline_projection(0..len, None);

            let cache = block.display_cache();
            let text = cache.text();
            let mut saw_tilde_subscript = false;
            let mut saw_caret_superscript = false;
            for span in cache.spans() {
                let segment = &text[span.range.clone()];
                if segment.contains('~') && span.style.script == InlineScript::Subscript {
                    saw_tilde_subscript = true;
                }
                if segment.contains('^') && span.style.script == InlineScript::Superscript {
                    saw_caret_superscript = true;
                }
            }
            assert!(
                saw_tilde_subscript,
                "`~` markers should keep subscript style"
            );
            assert!(
                saw_caret_superscript,
                "`^` markers should keep superscript style"
            );
        });
    });
}

#[gpui::test]
async fn footnote_reference_delimiters_keep_superscript_style_while_editing(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "引用[^note]结尾。\n\n[^note]: 脚注内容".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("root paragraph").clone();
        paragraph.update(cx, |block, _cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.rebuild_inline_projection(0..len, None);

            let cache = block.display_cache();
            let text = cache.text();
            assert!(text.contains("[^note]"), "expanded footnote source: {text}");
            let mut saw_bracket_superscript = false;
            for span in cache.spans() {
                let segment = &text[span.range.clone()];
                if (segment.contains("[^") || segment == "]")
                    && span.style.script == InlineScript::Superscript
                {
                    saw_bracket_superscript = true;
                }
            }
            assert!(
                saw_bracket_superscript,
                "`[^` / `]` markers should keep superscript style"
            );
        });
    });
}

#[gpui::test]
async fn footnote_definition_head_projects_markers_while_editing(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(cx, "正文[^note]。\n\n[^note]: 脚注内容".to_string(), None)
    });

    editor.update(cx, |editor, cx| {
        let definition = editor
            .doc()
            .blocks()
            .iter()
            .find(|entry| entry.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
            .expect("footnote definition block")
            .entity
            .clone();
        definition.update(cx, |block, _cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.rebuild_inline_projection(0..len, None);

            let text = block.display_cache().text().to_string();
            assert_eq!(
                text, "[^note]: 脚注内容",
                "footnote definition head should reveal its markers"
            );
        });
    });
}

#[gpui::test]
async fn projected_delimiter_ranges_cover_script_and_footnote_markers(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "H~2~O 和 x^2^ 引用[^note]结尾。\n\n[^note]: 脚注内容".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("root paragraph").clone();
        paragraph.update(cx, |block, _cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.rebuild_inline_projection(0..len, None);

            let text = block.display_cache().text();
            let ranges = block.projected_delimiter_ranges();
            let markers: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
            for marker in &markers {
                assert!(
                    matches!(*marker, "~" | "^" | "[^" | "]"),
                    "unexpected marker {marker:?}"
                );
            }
            assert!(
                markers.contains(&"~"),
                "subscript delimiters missing: {markers:?}"
            );
            assert!(
                markers.contains(&"^"),
                "superscript delimiters missing: {markers:?}"
            );
            assert!(
                markers.contains(&"[^"),
                "footnote open marker missing: {markers:?}"
            );
            assert!(
                markers.contains(&"]"),
                "footnote close marker missing: {markers:?}"
            );
        });
    });
}

#[gpui::test]
async fn projected_delimiter_ranges_cover_link_and_image_markers(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "链接 [title](https://example.com) 和图片 ![alt](image.png)".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("root paragraph").clone();
        paragraph.update(cx, |block, _cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.rebuild_inline_projection(0..len, None);

            let text = block.display_cache().text();
            let ranges = block.projected_delimiter_ranges();
            let markers: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
            assert!(
                markers.contains(&"["),
                "link open delimiter missing: {markers:?}"
            );
            assert!(
                markers.contains(&"]("),
                "link middle delimiter missing: {markers:?}"
            );
            assert!(
                markers.contains(&")"),
                "link close delimiter missing: {markers:?}"
            );
            assert!(
                markers.contains(&"!["),
                "image open delimiter missing: {markers:?}"
            );
        });
    });
}
