//! Multi-panel isolation: per-panel source runtimes and tab
//! switching renders the active document.

use gpui::{AppContext, TestAppContext};

use crate::editor::controller::Editor;
use crate::model::block::BlockKind;

use super::*;

#[gpui::test]
async fn rendering_one_editor_panel_keeps_other_panels_source_block(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    use crate::app::window_panels::DEFAULT_EDITOR_PANEL_ID;
    use crate::editor::session::{EditorPaneKind, EditorSession};

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\nbeta".to_string(), None)
    });

    // Two Editor panels in one window: each panel is its own entity holding
    // a SourceCode panel (the Shell materializes one entity per area).
    let second_entity = editor.update(cx, |editor, cx| {
        editor
            .session
            .root
            .tree
            .set_leaf_kind(1, EditorPaneKind::SourceCode);
        let mut second = Editor::with_session(2, EditorSession::welcome(), cx);
        second
            .session
            .tab_list
            .tabs
            .push(Editor::new_tab_from_markdown(
                cx,
                "gamma\ndelta".to_string(),
                None,
            ));
        second
            .session
            .root
            .tree
            .set_leaf_kind(1, EditorPaneKind::SourceCode);
        second
    });
    let second = cx.cx.new(|_cx| second_entity);

    fn source_block_id(
        editor: &gpui::Entity<Editor>,
        cx: &mut gpui::VisualTestContext,
        _panel_id: usize,
    ) -> Option<gpui::EntityId> {
        editor.read_with(cx, |editor, _cx| {
            editor
                .source_pane_runtimes
                .get(&1)
                .and_then(|runtime| runtime.block.as_ref().map(|block| block.entity_id()))
        })
    }

    // The first frame materializes the panel's block; every following
    // frame must keep it alive (rendering used to drop other panels'
    // source runtimes, rebuilding the block entity every frame).
    redraw(cx);
    let before = source_block_id(&editor, cx, DEFAULT_EDITOR_PANEL_ID);
    assert!(before.is_some(), "first area source block should exist");
    for _ in 0..3 {
        redraw(cx);
        assert_eq!(
            before,
            source_block_id(&editor, cx, DEFAULT_EDITOR_PANEL_ID),
            "source block entity must survive other render passes"
        );
    }

    // The second area's entity owns its own runtime, fully independent of
    // the first area's entity.
    second.update(&mut cx.cx, |second, cx| second.sync_source_pane(1, cx));
    let second_id = second.read_with(&mut cx.cx, |second, _cx| {
        second
            .source_pane_runtimes
            .get(&1)
            .and_then(|runtime| runtime.block.as_ref().map(|block| block.entity_id()))
    });
    assert!(second_id.is_some(), "second area source block should exist");
    assert_ne!(before, second_id, "each area owns its own source block");
}

#[gpui::test]
async fn switching_tabs_renders_the_new_document_immediately(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\nbeta".to_string(), None)
    });
    editor.update(cx, |editor, cx| {
        let list = &mut editor.session_mut().tab_list;
        list.tabs.push(Editor::new_tab_from_markdown(
            cx,
            "gamma\n\ndelta".to_string(),
            None,
        ));
        editor.activate_tab(1, cx);
    });

    fn active_blocks(
        editor: &gpui::Entity<Editor>,
        cx: &mut gpui::VisualTestContext,
    ) -> (Vec<gpui::EntityId>, String) {
        editor.read_with(cx, |editor, cx| {
            let blocks = editor.doc().blocks();
            let ids = blocks
                .iter()
                .map(|visible| visible.entity.entity_id())
                .collect();
            let text = editor.doc().to_markdown(cx);
            (ids, text)
        })
    }

    // Tab 1 (gamma/delta) is active; its blocks render.
    redraw(cx);
    let (tab1_ids, tab1_text) = active_blocks(&editor, cx);
    assert_eq!(tab1_text, "gamma\n\ndelta");
    assert_eq!(tab1_ids.len(), 2);

    // Switch back to tab 0 (alpha/beta): the very next frame must render
    // tab 0's blocks, not the previous tab's.
    editor.update(cx, |editor, cx| {
        editor.activate_tab(0, cx);
    });
    redraw(cx);
    let (tab0_ids, tab0_text) = active_blocks(&editor, cx);
    assert_eq!(tab0_text, "alpha\nbeta", "tab 0 document must be active");
    assert!(
        tab0_ids.iter().all(|id| !tab1_ids.contains(id)),
        "tab 0 must render its own block entities"
    );

    // And back to tab 1 again.
    editor.update(cx, |editor, cx| {
        editor.activate_tab(1, cx);
    });
    redraw(cx);
    let (ids_again, text_again) = active_blocks(&editor, cx);
    assert_eq!(
        text_again, "gamma\n\ndelta",
        "tab 1 document must be active"
    );
    assert_eq!(
        ids_again, tab1_ids,
        "tab 1 blocks must be stable across switches"
    );
}

#[gpui::test]
async fn switching_to_an_unrendered_tab_mounts_a_full_viewport(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\n\nbeta".to_string(), None)
    });
    cx.cx
        .update(|app| ensure_wysiwyg_editing_panel(&editor, app));

    // NOTE: deliberately NO first redraw — tab 0 is never rendered, so its
    // handle is never bound; the switch below tests the unbound-handle path.

    // Push a second tab that has never been rendered: its ScrollHandle has
    // never been bound, so its bounds are still (0,0,0,0).
    let long_doc = (0..80)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    editor.update(cx, |editor, cx| {
        let list = &mut editor.session_mut().tab_list;
        list.tabs
            .push(Editor::new_tab_from_markdown(cx, long_doc, None));
        editor.activate_tab(1, cx);
    });

    // Sanity check: the two tabs must NOT share a ScrollHandle.
    editor.update(cx, |editor, _cx| {
        let session = editor.session_mut();
        session.tab_list.tabs[0]
            .scroll
            .handle
            .set_offset(gpui::point(gpui::px(0.0), gpui::px(-500.0)));
        let tab1_offset = session.tab_list.tabs[1].scroll.handle.offset();
        assert_eq!(
            tab1_offset,
            gpui::point(gpui::px(0.0), gpui::px(0.0)),
            "each tab must own its own scroll handle"
        );
    });

    // The activate_tab update above flushes its notify as an immediate
    // draw: that very first frame must sync a real viewport (from the
    // window fallback), not a 0×0 unbound handle that would mount only a
    // 1px sliver of rows.
    let (viewport, handle_bounds) = editor.read_with(cx, |editor, _cx| {
        (
            editor.tab().scroll.last_viewport_size,
            editor.tab().scroll.handle.bounds(),
        )
    });
    assert!(
        viewport.is_some_and(|size| size.height > gpui::px(100.0)),
        "first frame after switching to an unrendered tab must sync a real viewport"
    );
    assert!(
        handle_bounds.size.height > gpui::px(100.0),
        "scroll handle must be bound to the real viewport by the first frame"
    );

    // The handle gets bound during layout, so later frames keep the full
    // window mounted.
    redraw(cx);
    let (start, end) = editor
        .read_with(cx, |editor, _cx| editor.tab().scroll.prev_render_window)
        .expect("document view must keep rendering");
    assert!(
        end - start > 10,
        "mounted rows must cover the viewport, got {start}..{end}"
    );
}

#[gpui::test]
async fn focused_thematic_break_accepts_typing(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\n\n---\n\nbeta".to_string(), None)
    });

    // Focus the thematic break block (the second visible block).
    let separator = editor
        .read_with(cx, |editor, cx| {
            editor
                .doc()
                .blocks()
                .iter()
                .find(|visible| visible.entity.read(cx).kind() == BlockKind::ThematicBreak)
                .map(|visible| visible.entity.clone())
        })
        .expect("document must contain a thematic break");
    focus_block(&editor, &separator, cx);
    redraw(cx);

    // Typing must land in the focused separator block.
    cx.simulate_input("x");
    redraw(cx);
    let text = separator.read_with(cx, |block, _cx| block.display_text().to_string());
    assert_ne!(text, "---", "focused thematic break must accept text input");
    assert!(
        text.contains('x'),
        "typed character must be inserted, got {text:?}"
    );

    // Serialization must round-trip the edited separator text.
    let markdown = editor.read_with(cx, |editor, cx| editor.doc().to_markdown(cx));
    assert!(
        markdown.contains(&text),
        "edited separator must serialize its new text, got {markdown:?}"
    );
}
