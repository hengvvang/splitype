//! Footnote reference binding and real-id display.

use gpui::{AppContext, Point, TestAppContext, px};

use crate::editor::engine::controller::Editor;
use splitype_model::parse::BlockKind;

#[gpui::test]
async fn footnote_tooltip_resolves_definition_content_only_when_bound(cx: &mut TestAppContext) {
    let markdown = "引用[^a]和缺失[^missing]。\n\n[^a]: A 脚注内容".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.update(cx, |editor, cx| {
        let position = Point::new(px(100.0), px(100.0));
        // Reference with a definition resolves the definition text (prefix stripped).
        editor.update_footnote_tooltip("a", None, position, true, cx);
        assert_eq!(
            editor
                .footnote_tooltip
                .as_ref()
                .map(|t| t.content.to_string()),
            Some("A 脚注内容".to_string())
        );
        // Reference without a definition hides the tooltip.
        editor.update_footnote_tooltip("missing", None, position, true, cx);
        assert!(editor.footnote_tooltip.is_none());
        // Definition headers carry their own text directly and get stripped.
        editor.update_footnote_tooltip("a", Some("a: A 脚注内容".into()), position, true, cx);
        assert_eq!(
            editor
                .footnote_tooltip
                .as_ref()
                .map(|t| t.content.to_string()),
            Some("A 脚注内容".to_string())
        );
        // Hiding clears it.
        editor.update_footnote_tooltip("a", None, position, false, cx);
        assert!(editor.footnote_tooltip.is_none());
    });
}

#[gpui::test]
async fn root_level_footnotes_render_real_ids_and_keep_in_place(cx: &mut TestAppContext) {
    let markdown = [
        "Here is a footnote reference.[^1]",
        "",
        "Here is another footnote reference.[^longnote]",
        "",
        "A footnote can appear after multiple paragraphs, lists, and code blocks.",
        "",
        "[^1]: Footnote text.",
        "",
        "[^longnote]: Footnote text with **bold**, `code`, and a nested list:",
        "    - item 1",
        "    - item 2",
        "    ",
        "    Second paragraph in the footnote.",
    ]
    .join("\n");
    let canonical_markdown = markdown.clone();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let entries = editor.doc().blocks();

        let first_ref = entries
            .iter()
            .find(|entries| {
                entries
                    .entity
                    .read(cx)
                    .display_text()
                    .contains("Here is a footnote reference.")
            })
            .expect("first footnote reference")
            .entity
            .clone();
        assert_eq!(
            first_ref.read(cx).display_text(),
            "Here is a footnote reference.1"
        );

        let second_ref = entries
            .iter()
            .find(|entries| {
                entries
                    .entity
                    .read(cx)
                    .display_text()
                    .contains("Here is another footnote reference.")
            })
            .expect("second footnote reference")
            .entity
            .clone();
        assert_eq!(
            second_ref.read(cx).display_text(),
            "Here is another footnote reference.longnote"
        );

        let footnote_defs = entries
            .iter()
            .filter_map(|entries| {
                let block = entries.entity.read(cx);
                (block.kind() == BlockKind::FootnoteDefinition).then_some(entries.entity.clone())
            })
            .collect::<Vec<_>>();
        assert_eq!(footnote_defs.len(), 2);
        assert_eq!(
            footnote_defs[0].read(cx).display_text(),
            "1: Footnote text."
        );
        assert_eq!(
            footnote_defs[1].read(cx).display_text(),
            "longnote: Footnote text with bold, code, and a nested list:"
        );

        assert_eq!(editor.doc().serialize_markdown(cx), canonical_markdown);
    });
}

#[gpui::test]
async fn callout_footnotes_number_and_render_in_place(cx: &mut TestAppContext) {
    let markdown = [
        "> [!WARNING]",
        "> Callout footnote reference.[^final]",
        "> ",
        "> [^final]: Nested footnote text.",
        "> Tail paragraph.",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let entries = editor.doc().blocks();

        let reference_block = entries
            .iter()
            .find(|entries| {
                entries
                    .entity
                    .read(cx)
                    .display_text()
                    .contains("Callout footnote reference.")
            })
            .expect("callout footnote reference")
            .entity
            .clone();
        assert_eq!(
            reference_block.read(cx).display_text(),
            "Callout footnote reference.final"
        );

        let definition = entries
            .iter()
            .find(|entries| entries.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
            .expect("callout footnote definition")
            .entity
            .clone();
        assert_eq!(
            definition.read(cx).display_text(),
            "final: Nested footnote text."
        );
        assert_eq!(definition.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}

#[gpui::test]
async fn root_reference_binds_to_nested_quote_footnote_definition(cx: &mut TestAppContext) {
    let markdown = "Root reference.[^note]\n\n> [^note]: Nested quote footnote".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let entries = editor.doc().blocks();

        let root_reference = entries
            .iter()
            .find(|entries| entries.entity.read(cx).quote_depth == 0)
            .expect("root reference block")
            .entity
            .clone();
        assert_eq!(
            root_reference.read(cx).display_text(),
            "Root reference.note"
        );

        let definition = entries
            .iter()
            .find(|entries| entries.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
            .expect("nested quote footnote definition")
            .entity
            .clone();
        assert_eq!(
            definition.read(cx).display_text(),
            "note: Nested quote footnote"
        );
        assert_eq!(definition.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}

#[gpui::test]
async fn unresolved_footnote_reference_renders_real_id_without_binding(cx: &mut TestAppContext) {
    let markdown = "Missing footnote[^missing].".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root paragraph").clone();
        // The reference still renders its real id with the normal style;
        // only the definition binding (jump target, hover content) is absent.
        assert_eq!(block.read(cx).display_text(), "Missing footnotemissing.");
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

#[gpui::test]
async fn footnote_tooltip_anchors_to_reference_and_renders_compact_element(cx: &mut TestAppContext) {
    super::init_editor_test_app(cx);

    let markdown = "这是第二个引用[^note2]。\n\n[^note2]: 第二个脚注内容。".to_string();
    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, markdown.clone(), None)
    });

    let theme = splitype_infra::theme::Theme::default_theme();

    editor.update_in(cx, |editor, window, cx| {
        let anchor_position = Point::new(px(150.0), px(80.0));
        editor.update_footnote_tooltip("note2", None, anchor_position, true, cx);
        assert!(editor.footnote_tooltip.is_some());
        assert_eq!(
            editor.footnote_tooltip.as_ref().unwrap().content.as_ref(),
            "第二个脚注内容。"
        );

        let element = editor.render_footnote_tooltip(&theme, window, cx);
        assert!(element.is_some());
    });
}

#[gpui::test]
async fn footnote_backref_jump_reveals_source_and_selects_only_id(cx: &mut TestAppContext) {
    super::init_editor_test_app(cx);

    let markdown = "这是一段带有脚注的文本[^note1], 还有第二个脚注[^note2]。\n\n[^note1]: 这是第一个脚注的详细说明内容。".to_string();
    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, markdown.clone(), None)
    });

    editor.update(cx, |editor, cx| {
        let jumped = editor.jump_to_footnote_backref("note1", cx);
        assert!(jumped, "should successfully jump to footnote backref");

        let active_id = editor
            .active_pane_focus()
            .active_entity
            .or(editor.active_pane_focus().pending)
            .expect("focused block id");
        let active_block = editor
            .focusable_entity_by_id(active_id)
            .expect("focused block");
        active_block.read_with(cx, |block, _cx| {
            let display_text = block.display_text();
            assert_eq!(
                display_text,
                "这是一段带有脚注的文本[^note1], 还有第二个脚注[^note2]。"
            );
            assert_eq!(
                &display_text[block.selected_range.clone()],
                "note1",
                "selection should only select footnote name 'note1', not [^ or ]"
            );
        });
    });
}

#[gpui::test]
async fn footnote_backref_jump_first_click_with_render_sync_keeps_exact_id_selection(
    cx: &mut TestAppContext,
) {
    super::init_editor_test_app(cx);

    let markdown = "这是一段带有脚注的文本[^note1], 还有第二个脚注[^note2]。\n\n[^note1]: 这是第一个脚注的详细说明内容。".to_string();
    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, markdown.clone(), None)
    });

    editor.update(cx, |editor, cx| {
        let jumped = editor.jump_to_footnote_backref("note1", cx);
        assert!(jumped, "should successfully jump to footnote backref on first click");

        let active_id = editor
            .active_pane_focus()
            .active_entity
            .or(editor.active_pane_focus().pending)
            .expect("focused block id");
        let active_block = editor
            .focusable_entity_by_id(active_id)
            .expect("focused block");

        active_block.update(cx, |block, _cx| {
            block.sync_inline_projection_for_focus(true);
            let display_text = block.display_text();
            assert_eq!(
                display_text,
                "这是一段带有脚注的文本[^note1], 还有第二个脚注[^note2]。"
            );
            assert_eq!(
                &display_text[block.selected_range.clone()],
                "note1",
                "first click must select only note1 and NEVER include ]"
            );
        });
    });
}

