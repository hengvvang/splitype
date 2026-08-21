//! Multi-panel isolation: per-panel source pane states and tab
//! switching renders the active document.

use gpui::{AppContext, TestAppContext, VisualTestContext};

use crate::editor::controller::Editor;
use crate::model::inline::text::BlockText;
use crate::model::parse::BlockKind;

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
                .pane_state_ref(1)
                .and_then(|state| state.source_block.as_ref().map(|block| block.entity_id()))
        })
    }

    // The first frame materializes the panel's block; every following
    // frame must keep it alive (rendering used to drop other panels'
    // source pane states, rebuilding the block entity every frame).
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

    // The second area's entity owns its own pane state, fully independent
    // of the first area's entity.
    second.update(&mut cx.cx, |second, cx| second.sync_source_pane(1, cx));
    let second_id = second.read_with(&cx.cx, |second, _cx| {
        second
            .pane_state_ref(1)
            .and_then(|state| state.source_block.as_ref().map(|block| block.entity_id()))
    });
    assert!(second_id.is_some(), "second area source block should exist");
    assert_ne!(before, second_id, "each area owns its own source block");
}

#[gpui::test]
async fn switching_tabs_rebuilds_the_source_pane_block(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    // Two fresh tabs: both start at document_revision 0, which used to make
    // the source pane think nothing changed when switching between them.
    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\nbeta".to_string(), None)
    });
    editor.update(cx, |editor, cx| {
        let list = &mut editor.session_mut().tab_list;
        list.tabs.push(Editor::new_tab_from_markdown(
            cx,
            "gamma\ndelta".to_string(),
            None,
        ));
    });

    fn source_text(editor: &gpui::Entity<Editor>, cx: &mut VisualTestContext) -> String {
        editor.read_with(cx, |editor, _cx| {
            editor
                .pane_state_ref(1)
                .and_then(|state| state.source_block.as_ref())
                .map(|block| block.read(_cx).display_text().to_string())
                .unwrap_or_default()
        })
    }

    // The default pane tree holds a single SourceCode pane with id 1.
    editor.update(cx, |editor, cx| editor.sync_source_pane(1, cx));
    assert_eq!(source_text(&editor, cx), "alpha\nbeta");

    // Switching to the second tab must rebuild the source block even though
    // both tabs share revision 0.
    editor.update(cx, |editor, cx| {
        editor.activate_tab(1, cx);
        editor.sync_source_pane(1, cx);
    });
    assert_eq!(source_text(&editor, cx), "gamma\ndelta");

    // And back again.
    editor.update(cx, |editor, cx| {
        editor.activate_tab(0, cx);
        editor.sync_source_pane(1, cx);
    });
    assert_eq!(source_text(&editor, cx), "alpha\nbeta");
}

#[gpui::test]
async fn stale_source_block_events_do_not_clobber_the_active_tab(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\nbeta".to_string(), None)
    });
    editor.update(cx, |editor, cx| {
        let list = &mut editor.session_mut().tab_list;
        list.tabs.push(Editor::new_tab_from_markdown(
            cx,
            "gamma\ndelta".to_string(),
            None,
        ));
    });

    // Sync the pane against tab A, then switch to tab B: the rebuild drops
    // the old block entity, but a handle to it can stay alive (mounted,
    // even focused) for a frame or two after the rebuild.
    editor.update(cx, |editor, cx| editor.sync_source_pane(1, cx));
    let stale_block = editor
        .read_with(cx, |editor, _cx| {
            editor
                .pane_state_ref(1)
                .and_then(|state| state.source_block.clone())
        })
        .expect("source block must exist");
    editor.update(cx, |editor, cx| {
        editor.activate_tab(1, cx);
        editor.sync_source_pane(1, cx);
    });
    let current_block = editor
        .read_with(cx, |editor, _cx| {
            editor
                .pane_state_ref(1)
                .and_then(|state| state.source_block.clone())
        })
        .expect("source block must exist");
    assert_ne!(stale_block.entity_id(), current_block.entity_id());

    // A late keystroke lands in the replaced block: its Changed event must
    // NOT rewrite tab B's document with tab A's text.
    stale_block.update(&mut cx.cx, |block, cx| {
        block.apply_text_edit(
            BlockText::plain("alpha\nbetax".to_string()),
            block.display_text().len(),
            None,
            None,
            None,
            false,
            cx,
        );
    });
    let doc = editor.read_with(cx, |editor, cx| editor.doc().serialize_markdown(cx));
    assert_eq!(
        doc, "gamma\ndelta",
        "stale block events must not clobber the active tab"
    );
}

#[gpui::test]
async fn typing_in_the_source_block_after_a_tab_switch_still_syncs(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\nbeta".to_string(), None)
    });
    editor.update(cx, |editor, cx| {
        let list = &mut editor.session_mut().tab_list;
        list.tabs.push(Editor::new_tab_from_markdown(
            cx,
            "gamma\ndelta".to_string(),
            None,
        ));
    });

    // Switch to tab B and sync: the pane block is rebuilt from tab B.
    editor.update(cx, |editor, cx| {
        editor.activate_tab(1, cx);
        editor.sync_source_pane(1, cx);
    });
    let block = editor
        .read_with(cx, |editor, _cx| {
            editor
                .pane_state_ref(1)
                .and_then(|state| state.source_block.clone())
        })
        .expect("source block must exist");

    // Focus the rebuilt block and type: the edit must flow into tab B's
    // document, exactly as before the tab switch.
    cx.update(|window, cx| block.read(cx).focus_handle.focus(window));
    redraw(cx);
    cx.simulate_input("x");
    redraw(cx);

    let doc = editor.read_with(cx, |editor, cx| editor.doc().serialize_markdown(cx));
    assert!(
        doc.contains('x'),
        "typing after a tab switch must sync, got {doc:?}"
    );
    assert!(
        !doc.contains("alpha"),
        "typing must land in the active tab, got {doc:?}"
    );
}

#[gpui::test]
async fn two_wysiwyg_panes_map_clicks_in_each_pane_to_the_correct_caret(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\nbeta".to_string(), None)
    });
    cx.cx
        .update(|app| ensure_wysiwyg_editing_panel(&editor, app));
    editor.update(cx, |editor, _cx| {
        editor.split_pane_with_ratio(1, crate::splitter::SplitAxis::Horizontal, 0.5);
    });
    // The panes share one scroll handle: the first frame windows against the
    // window viewport (handle unbound), the second frame against the pane
    // viewport (handle bound) — redraw twice to reach the stable layout.
    redraw(cx);
    redraw(cx);

    // The same block entity is painted once per pane, so a single stored
    // geometry would only match the pane painted last (the right one). The
    // left pane's text begins a full pane width earlier with the same
    // centering.
    let (window_width, pane_gap) = cx.update(|window, cx| {
        let theme = cx
            .global::<crate::infra::theme::ThemeManager>()
            .current_arc();
        (window.viewport_size().width, theme.dimensions.pane_gap)
    });
    let (click_left, click_right, y) = editor.read_with(cx, |editor, cx| {
        let block = &editor.doc().blocks()[0].entity;
        let bounds = block
            .read(cx)
            .last_paint()
            .map(|paint| paint.bounds)
            .expect("block must have painted bounds");
        let pane_width = (f32::from(window_width) - pane_gap) / 2.0;
        let y = bounds.top() + gpui::px(10.0);
        (
            bounds.left() - gpui::px(pane_width) + gpui::px(20.0),
            bounds.left() + gpui::px(20.0),
            y,
        )
    });

    // The same position inside the text must map to the same caret in both
    // panes; the left pane used to be resolved against the right pane's
    // geometry and snapped to offset 0.
    let (left_index, right_index) = editor.read_with(cx, |editor, cx| {
        let block = &editor.doc().blocks()[0].entity;
        let block = block.read(cx);
        (
            block.index_for_mouse_position(gpui::point(click_left, y)),
            block.index_for_mouse_position(gpui::point(click_right, y)),
        )
    });
    assert!(
        left_index > 0,
        "left-pane click must map into the text, got {left_index}"
    );
    assert_eq!(
        left_index, right_index,
        "the same text position must map to the same caret in both panes"
    );

    // End-to-end: a real click in the left pane must land the caret in the
    // text instead of snapping to the block start.
    cx.simulate_mouse_down(
        gpui::point(click_left, y),
        gpui::MouseButton::Left,
        gpui::Modifiers::none(),
    );
    redraw(cx);
    let caret = editor.read_with(cx, |editor, cx| {
        editor.doc().blocks()[0]
            .entity
            .read(cx)
            .selected_range
            .start
    });
    assert!(
        caret > 0,
        "clicking the LEFT pane must place the caret inside the text, got {caret}"
    );
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
                .map(|entries| entries.entity.entity_id())
                .collect();
            let text = editor.doc().serialize_markdown(cx);
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
async fn wysiwyg_panes_scroll_independently(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\nbeta\ngamma\ndelta".to_string(), None)
    });
    cx.cx
        .update(|app| ensure_wysiwyg_editing_panel(&editor, app));
    editor.update(cx, |editor, _cx| {
        editor.split_pane_with_ratio(1, crate::splitter::SplitAxis::Horizontal, 0.5);
    });
    redraw(cx);
    redraw(cx);

    let pane_2 = editor.read_with(cx, |editor, _cx| {
        let mut ids = Vec::new();
        editor.session().root.tree.leaf_ids(&mut ids);
        ids[1]
    });

    // The two Wysiwyg panes share the document but own separate scroll
    // handles: scrolling pane 1 must not move pane 2.
    editor.update(cx, |editor, _cx| {
        let pane_a = &mut editor.pane_state(1).scroll;
        pane_a
            .handle
            .set_offset(gpui::point(gpui::px(0.0), gpui::px(-100.0)));
    });
    editor.update(cx, |editor, _cx| {
        let pane_b_offset = editor
            .pane_state_ref(pane_2)
            .map(|state| state.scroll.handle.offset());
        assert_eq!(
            pane_b_offset,
            Some(gpui::point(gpui::px(0.0), gpui::px(0.0))),
            "each Wysiwyg pane must own its own scroll position"
        );
        let pane_a_offset = editor
            .pane_state_ref(1)
            .map(|state| state.scroll.handle.offset());
        assert_eq!(
            pane_a_offset,
            Some(gpui::point(gpui::px(0.0), gpui::px(-100.0)))
        );
    });
}

#[gpui::test]
async fn clicking_a_block_in_another_pane_updates_that_panes_focus_target(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\n\nbeta".to_string(), None)
    });
    cx.cx
        .update(|app| ensure_wysiwyg_editing_panel(&editor, app));
    editor.update(cx, |editor, _cx| {
        editor.split_pane_with_ratio(1, crate::splitter::SplitAxis::Horizontal, 0.5);
    });
    redraw(cx);
    redraw(cx);

    let pane_2 = editor.read_with(cx, |editor, _cx| {
        let mut ids = Vec::new();
        editor.session().root.tree.leaf_ids(&mut ids);
        ids[1]
    });

    let pane1_target_before = editor.read_with(cx, |editor, _cx| {
        editor
            .pane_state_ref(1)
            .and_then(|state| state.focus.active_entity)
    });

    // Both panes render the same block entities; the stored paint is the
    // last-painted (right) pane's geometry. Click the SECOND block (beta,
    // not focused yet) inside pane 2: the RequestFocus it emits must route
    // to PANE 2's focus state (the pane div switches `focused_pane` in the
    // capture phase, before the block handles the click), not to pane 1's.
    let (click, block_id) = editor.read_with(cx, |editor, cx| {
        let block = &editor.doc().blocks()[1].entity;
        let paint = block
            .read(cx)
            .last_paint()
            .expect("block must have painted");
        (
            gpui::point(
                paint.bounds.left() + gpui::px(20.0),
                paint.bounds.top() + gpui::px(10.0),
            ),
            block.entity_id(),
        )
    });
    cx.simulate_mouse_down(click, gpui::MouseButton::Left, gpui::Modifiers::none());
    redraw(cx);

    let (focused_pane, pane1_target, pane2_target) = editor.read_with(cx, |editor, _cx| {
        (
            editor.focused_pane_id,
            editor
                .pane_state_ref(1)
                .and_then(|state| state.focus.active_entity),
            editor
                .pane_state_ref(pane_2)
                .and_then(|state| state.focus.active_entity),
        )
    });
    assert_eq!(focused_pane, Some(pane_2), "clicking pane 2 must focus pane 2");
    assert_eq!(
        pane2_target,
        Some(block_id),
        "clicking a block in pane 2 must record pane 2's edit target"
    );
    assert_eq!(
        pane1_target, pane1_target_before,
        "pane 1's edit target must not be overwritten by pane 2's click"
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
        session.tab_list.tabs[0].panes[&1]
            .scroll
            .handle
            .set_offset(gpui::point(gpui::px(0.0), gpui::px(-500.0)));
        let tab1_offset = session.tab_list.tabs[1].panes[&1].scroll.handle.offset();
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
            editor.active_pane_scroll().last_viewport_size,
            editor.active_pane_scroll().handle.bounds(),
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
    let block_count = editor.read_with(cx, |editor, _cx| editor.doc().blocks().len());
    assert!(
        block_count > 10,
        "mounted rows must cover the document, got {block_count}"
    );
}

#[gpui::test]
async fn focused_thematic_break_accepts_typing(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\n\n---\n\nbeta".to_string(), None)
    });

    // Focus the thematic break block (the second block in the document).
    let separator = editor
        .read_with(cx, |editor, cx| {
            editor
                .doc()
                .blocks()
                .iter()
                .find(|entries| entries.entity.read(cx).kind() == BlockKind::ThematicBreak)
                .map(|entries| entries.entity.clone())
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
    let markdown = editor.read_with(cx, |editor, cx| editor.doc().serialize_markdown(cx));
    assert!(
        markdown.contains(&text),
        "edited separator must serialize its new text, got {markdown:?}"
    );
}

#[gpui::test]
async fn border_menu_split_and_close_actions_operate_on_divider_split_id(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view({
        move |_window, cx| Editor::from_markdown(cx, "alpha\n\nbeta".to_string(), None)
    });

    // 1. Initial state: 1 pane (Leaf 1).
    assert_eq!(editor.read_with(cx, |ed, _cx| ed.session().root.tree.count_leaves()), 1);

    // 2. Split leaf 1 horizontally.
    editor.update(cx, |ed, _cx| {
        ed.split_pane_with_ratio(1, crate::splitter::SplitAxis::Horizontal, 0.5);
    });
    assert_eq!(editor.read_with(cx, |ed, _cx| ed.session().root.tree.count_leaves()), 2);

    // 3. Get the internal Split node ID (the divider ID passed by border menu).
    let split_id = editor.read_with(cx, |ed, _cx| {
        match &ed.session().root.tree {
            crate::splitter::SplitTree::Split { id, .. } => *id,
            _ => panic!("expected split tree"),
        }
    });

    // 4. Trigger split_pane_with_ratio using split_id (divider right-click action).
    editor.update(cx, |ed, _cx| {
        ed.split_pane_with_ratio(split_id, crate::splitter::SplitAxis::Vertical, 0.5);
    });
    assert_eq!(
        editor.read_with(cx, |ed, _cx| ed.session().root.tree.count_leaves()),
        3,
        "splitting from divider ID must successfully create a third pane"
    );

    // 5. Trigger close_pane using split_id (divider right-click close action).
    editor.update(cx, |ed, _cx| {
        ed.close_pane(split_id);
    });
    assert_eq!(
        editor.read_with(cx, |ed, _cx| ed.session().root.tree.count_leaves()),
        2,
        "closing from divider ID must successfully remove a pane"
    );
}
