//! Footnote parsing edge cases: same-line and adjacent-line syntax.

use gpui::{AppContext, TestAppContext};

use crate::editor::controller::Editor;
use crate::model::parse::BlockKind;

#[gpui::test]
async fn adjacent_lines_reference_then_definition_both_recognized(cx: &mut TestAppContext) {
    let markdown = "正文引用[^a]\n\n[^a]: 脚注内容".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        let has_definition = entries
            .iter()
            .any(|entry| entry.entity.read(cx).kind() == BlockKind::FootnoteDefinition);
        assert!(has_definition, "footnote definition not recognized");
        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}

#[gpui::test]
async fn two_references_on_one_line_both_render_real_ids(cx: &mut TestAppContext) {
    let markdown = "引用[^a]和[^b]都在一行。\n\n[^a]: A 内容\n\n[^b]: B 内容".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root paragraph").clone();
        let text = block.read(cx).display_text();
        assert!(text.contains("a"), "first reference id missing: {text}");
        assert!(text.contains("b"), "second reference id missing: {text}");
        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}

#[gpui::test]
async fn two_definitions_on_one_line_both_recognized(cx: &mut TestAppContext) {
    // `[^a]: x [^b]: y` carries two definition heads on one line; both
    // become native definitions (serialized back one per line).
    let markdown = "[^a]: A 内容 [^b]: B 内容".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        let definitions = entries
            .iter()
            .filter(|entry| entry.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
            .collect::<Vec<_>>();
        assert_eq!(definitions.len(), 2);
        assert_eq!(definitions[0].entity.read(cx).display_text(), "a: A 内容");
        assert_eq!(definitions[1].entity.read(cx).display_text(), "b: B 内容");
        assert_eq!(
            editor.doc().serialize_markdown(cx),
            "[^a]: A 内容\n\n[^b]: B 内容"
        );
    });
}

#[gpui::test]
async fn three_definitions_on_one_line_keep_trailing_content(cx: &mut TestAppContext) {
    let markdown = "[^a]: A [^b]: B [^c]: C 尾部".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let definitions = editor
            .doc()
            .blocks()
            .iter()
            .filter(|entry| entry.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
            .map(|entry| entry.entity.read(cx).display_text().to_string())
            .collect::<Vec<_>>();
        assert_eq!(definitions, vec!["a: A", "b: B", "c: C 尾部"]);
        assert_eq!(
            editor.doc().serialize_markdown(cx),
            "[^a]: A\n\n[^b]: B\n\n[^c]: C 尾部"
        );
    });
}

#[gpui::test]
async fn reference_without_definition_renders_real_id_like_resolved(cx: &mut TestAppContext) {
    // Even without a matching definition the reference renders its real id
    // with the same style; only the binding (jump/hover content) is absent.
    let markdown = "缺失引用[^missing]".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root paragraph").clone();
        assert_eq!(block.read(cx).display_text(), "缺失引用missing");
        assert!(
            editor
                .tab()
                .references
                .footnotes
                .binding("missing")
                .is_none()
        );
        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}
