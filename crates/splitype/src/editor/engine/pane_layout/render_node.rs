//! Recursive rendering for split layout tree nodes, split containers, and pane viewports.

use gpui::*;

use crate::editor::engine::controller::*;
use crate::editor::engine::session::EditorPaneKind;
use splitype_infra::i18n::I18nStrings;
use splitype_infra::theme::Theme;
use splitype_splitter::SplitAxis;
use splitype_splitter::tree::SplitTree;

impl Editor {
    pub(crate) fn render_editor_pane_node(
        &mut self,
        node: &splitype_splitter::tree::SplitTree<crate::editor::engine::session::EditorPaneKind>,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let overlay_style = splitype_splitter::interaction::OverlayStyle {
            accent: c.split_indicator,
            tile_radius: d.panel_tile_radius,
            border: c.dialog_border,
            selection: c.selection,
            active: c.focus_accent,
            surface: c.dialog_surface,
            text: c.dialog_title,
        };

        match node {
            SplitTree::Leaf(container) => {
                let pane_id = PaneId(container.id);
                let kind = container.kind;
                let inner_editor = cx.entity().downgrade();

                // The pane kind is the view type; in the welcome mode
                // (no tabs) every pane renders the guidance prompt instead
                // of its view, so the split layout stays visible.
                let inner_body: AnyElement = if self.panel_mode().is_editing() {
                    let pane_body = match kind {
                        // WYSIWYG — this editor's own block editor pane.
                        EditorPaneKind::Wysiwyg => self.render_wysiwyg_pane(pane_id, window, cx),
                        // Source — interactive source code editor. Uses a
                        // cached block in source-document mode; edits sync
                        // to the shared document via the block's Changed
                        // event.
                        EditorPaneKind::SourceCode => {
                            self.sync_source_pane(pane_id, cx);
                            self.render_source_pane(pane_id, theme, window, cx)
                        }
                        EditorPaneKind::Preview => {
                            self.render_preview_pane(pane_id, theme, strings, window, cx)
                        }
                    };
                    let outline_hud = self.render_floating_outline_hud(pane_id, kind, theme, cx);
                    div()
                        .relative()
                        .w_full()
                        .h_full()
                        .child(pane_body)
                        .child(outline_hud)
                        .into_any_element()
                } else {
                    self.render_welcome_prompt(pane_id, theme, cx)
                };

                let corner_handles = splitype_splitter::interaction::corner_drag_handles(
                    "inner-corner",
                    pane_id.0,
                    d.pane_gap,
                    20.0,
                    false,
                    false,
                    move |modifier, pos, cx| {
                        let _ = inner_editor.update(cx, |ed, cx| {
                            ed.session_mut()
                                .root
                                .start_corner_drag(pane_id.0, pos, modifier);
                            cx.notify();
                        });
                    },
                );

                // Auto-focus first pane if none is focused. The
                // area activation happens on user interaction (panel
                // click / focus) — a Shell update from inside this
                // editor's own render would re-enter `sync_panel_states`
                // and try to update this very entity while it renders.
                if self.focused_pane_id.is_none() {
                    self.focused_pane_id = Some(pane_id);
                }

                let focus_editor = cx.entity().downgrade();
                let panel_gap = d.pane_gap;

                // The leaf container is the split-out pane area: unadorned and
                // seamless, it only partitions the initialized region. The
                // pane floats inside it with a uniform inset on all four
                // sides and carries the content.
                div()
                    .id(("pane-wrapper", pane_id.0))
                    .w_full()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .id(("pane-card", pane_id.0))
                            .absolute()
                            .inset(px(panel_gap))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .rounded(px(d.panel_tile_radius))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .shadow_lg()
                            .child(
                                div()
                                    .w_full()
                                    .flex_1()
                                    .min_h(px(0.0))
                                    .overflow_hidden()
                                    .child(inner_body),
                            )
                            // Bubble phase is safe here: block mouse-downs emit
                            // RequestFocus, and gpui delivers entity events at
                            // the END of the update — after this handler set
                            // `focused_pane` — so focus_block always routes to
                            // the newly clicked pane.
                            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                                let _ = focus_editor.update(cx, |ed, cx| {
                                    // Select this panel and move the keyboard edit
                                    // focus to its editing target (source block or
                                    // the shared document block).
                                    ed.focus_pane(pane_id, window, cx);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(corner_handles)
                    .into_any_element()
            }
            SplitTree::Split {
                id,
                axis,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let split_axis = *axis;
                let r = *ratio;
                let first_elem = self.render_editor_pane_node(first, theme, strings, window, cx);
                let second_elem = self.render_editor_pane_node(second, theme, strings, window, cx);

                let inner_editor = cx.entity().downgrade();

                match axis {
                    SplitAxis::Horizontal => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        let bar_active = self
                            .session
                            .root
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_row()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .relative()
                            .child(
                                div()
                                    .w(relative(r))
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                div()
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .child(
                                splitype_splitter::interaction::splitter_bar_h(
                                    ("inner-root-bar-h", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.x);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        // The move handler tracks the pointer in the
                                        // area's local space, so rebase the start
                                        // position the same way.
                                        let local_start = ed
                                            .panel_rect
                                            .map(|rect| start_pos - f32::from(rect.origin.x))
                                            .unwrap_or(start_pos);
                                        let session = ed.session_mut();
                                        splitype_splitter::interaction::start_splitter_drag(
                                            &mut session.root,
                                            split_id,
                                            SplitAxis::Horizontal,
                                            local_start,
                                            r,
                                        );
                                        cx.notify();
                                    });
                                })
                                .on_mouse_down(
                                    MouseButton::Right,
                                    move |event, _window, cx| {
                                        let pos = event.position;
                                        let _ = menu_editor.update(cx, |ed, cx| {
                                            let session = ed.session_mut();
                                            splitype_splitter::interaction::open_border_menu(
                                                &mut session.root,
                                                split_id,
                                                split_axis,
                                                pos,
                                            );
                                            cx.notify();
                                        });
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                    SplitAxis::Vertical => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        let bar_active = self
                            .session
                            .root
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .relative()
                            .child(
                                div()
                                    .h(relative(r))
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .child(
                                splitype_splitter::interaction::splitter_bar_v(
                                    ("inner-root-bar-v", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.y);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        // Rebase the start position into the area's
                                        // local space, matching the move handler.
                                        let local_start = ed
                                            .panel_rect
                                            .map(|rect| start_pos - f32::from(rect.origin.y))
                                            .unwrap_or(start_pos);
                                        let session = ed.session_mut();
                                        splitype_splitter::interaction::start_splitter_drag(
                                            &mut session.root,
                                            split_id,
                                            SplitAxis::Vertical,
                                            local_start,
                                            r,
                                        );
                                        cx.notify();
                                    });
                                })
                                .on_mouse_down(
                                    MouseButton::Right,
                                    move |event, _window, cx| {
                                        let pos = event.position;
                                        let _ = menu_editor.update(cx, |ed, cx| {
                                            let session = ed.session_mut();
                                            splitype_splitter::interaction::open_border_menu(
                                                &mut session.root,
                                                split_id,
                                                split_axis,
                                                pos,
                                            );
                                            cx.notify();
                                        });
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                }
            }
        }
    }
}
