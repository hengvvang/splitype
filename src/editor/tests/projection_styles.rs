//! Inline projection keeps delimiter styles while editing.

use gpui::{AppContext, TestAppContext};

use crate::editor::engine::controller::Editor;
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

#[gpui::test]
async fn inline_code_delimiters_have_no_code_style_and_are_included_in_markers(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "调用 `printf(\"Hello World\")` 函数".to_string(),
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
            assert!(text.contains("`printf(\"Hello World\")`"));

            // Check that the backtick delimiters are in projected_delimiter_ranges
            let ranges = block.projected_delimiter_ranges();
            let markers: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
            assert!(markers.contains(&"`"), "backtick delimiter missing in markers");

            // Check that the backtick spans do NOT have code style, while inner text DOES
            for span in cache.spans() {
                let segment = &text[span.range.clone()];
                if segment == "`" {
                    assert!(
                        !span.style.code,
                        "backtick delimiter should not have code style (no background highlight)"
                    );
                } else if segment.contains("printf") {
                    assert!(
                        span.style.code,
                        "inner code text should have code style (background highlight)"
                    );
                }
            }
        });
    });
}

#[gpui::test]
async fn script_and_footnote_always_expand_with_purple_markers_when_line_active(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "下标: H~2~O, 上标: X^2^, 引用[^note]尾部".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("root paragraph").clone();
        paragraph.update(cx, |block, _cx| {
            // Caret is at position 0 ("下|标..."), NOT touching ~2~ or ^2^ or [^note]
            block.selected_range = 0..0;
            block.rebuild_inline_projection(0..0, None);

            let cache = block.display_cache();
            let text = cache.text();
            // All script and footnote syntax must be expanded in source form
            assert!(text.contains("~2~"), "subscript ~2~ should be expanded: {text}");
            assert!(text.contains("^2^"), "superscript ^2^ should be expanded: {text}");
            assert!(text.contains("[^note]"), "footnote reference [^note] should be expanded: {text}");

            let delimiter_ranges = block.projected_delimiter_ranges();
            let marker_strings: Vec<&str> = delimiter_ranges
                .iter()
                .map(|r| &text[r.clone()])
                .collect();
            assert!(marker_strings.contains(&"~"), "subscript ~ delimiter in markers");
            assert!(marker_strings.contains(&"^"), "superscript ^ delimiter in markers");
            assert!(marker_strings.contains(&"[^"), "footnote [^ delimiter in markers");
            assert!(marker_strings.contains(&"]"), "footnote ] delimiter in markers");
        });
    });
}

#[gpui::test]
async fn footnote_definition_displays_clean_when_unfocused_and_projects_when_focused(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "正文内容[^note1]。\n\n[^note1]: 这里是脚注内容".to_string(),
            None,
        )
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
            // In unfocused state, footnote definitions display clean "note1: 脚注内容" without raw markdown markers
            block.sync_inline_projection_for_focus(false);
            let unfocused_text = block.display_text();
            assert_eq!(
                unfocused_text,
                "note1: 这里是脚注内容",
                "unfocused footnote definition must show clean rendered text"
            );

            // In focused state, footnote definitions project full markdown source syntax "[^note1]: 这里是脚注内容"
            block.sync_inline_projection_for_focus(true);
            let focused_text = block.display_text();
            assert_eq!(
                focused_text,
                "[^note1]: 这里是脚注内容",
                "focused footnote definition must project markdown syntax markers"
            );

            let delimiter_ranges = block.projected_delimiter_ranges();
            let marker_strings: Vec<&str> = delimiter_ranges
                .iter()
                .map(|r| &focused_text[r.clone()])
                .collect();
            assert!(marker_strings.contains(&"[^"), "missing [^ delimiter in markers");
            assert!(marker_strings.contains(&"]"), "missing ] delimiter in markers");
        });
    });
}

#[gpui::test]
async fn highlight_projects_equal_delimiters_while_editing(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "这是 ==高亮内容== 结束".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root paragraph").clone();
        block.update(cx, |block, _cx| {
            // Unfocused: delimiters hidden, clean display text
            block.sync_inline_projection_for_focus(false);
            assert_eq!(block.display_text(), "这是 高亮内容 结束");

            // Focused with caret inside highlight span: projects `==` delimiters
            // "这是 高亮内容 结束" -> "高亮内容" starts at plain offset 7 (byte length of "这是 ")
            let highlight_offset = "这是 ".len();
            block.selected_range = highlight_offset..highlight_offset;
            block.sync_inline_projection_for_focus(true);

            let focused_text = block.display_text();
            assert_eq!(
                focused_text,
                "这是 ==高亮内容== 结束",
                "focused highlight must reveal `==` delimiters"
            );

            let delimiter_ranges = block.projected_delimiter_ranges();
            let marker_strings: Vec<&str> = delimiter_ranges
                .iter()
                .map(|r| &focused_text[r.clone()])
                .collect();
            assert_eq!(
                marker_strings,
                vec!["==", "=="],
                "revealed delimiters must be `==`"
            );
        });
    });
}



