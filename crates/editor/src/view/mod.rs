//! Editor view components — UI rendering, topbar, bottombar, and frame synchronization.

pub mod bottombar;
pub mod sync;
pub mod topbar;
pub mod words;

use gpui::*;

use crate::editor::Editor;
use config::language::I18nManager;
use theme::ThemeManager;

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.has_tabs() {
            let active_pane = self.active_pane_id();
            self.apply_pending_focus(active_pane, window, cx);
            self.sync_pending_save(window, cx);
            self.sync_pending_save_as(window, cx);
            self.sync_window_edited_state(window, cx);
        }

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        if self.has_tabs() {
            self.sync_window_title(window, &strings, cx);
        }

        let follow_modifier_active = window.modifiers().secondary();
        let d = &theme.dimensions;
        let c = &theme.colors;
        let panel_id = self.panel_id;
        let leaf_count = self.leaf_count;
        let is_maximized = self.is_maximized;

        let base = div()
            .id(("editor-panel-tile", panel_id.as_usize()))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .rounded(px(d.panel_tile_radius))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .shadow_lg()
            .font(theme::TypographyStore::prose_font(cx))
            .on_modifiers_changed(move |event, window, _| {
                if event.modifiers.secondary() != follow_modifier_active {
                    window.refresh();
                }
            })
            .capture_key_down(cx.listener(Self::on_editor_key_down_capture))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_save_document))
            .on_action(cx.listener(Self::on_save_document_as))
            .on_action(cx.listener(Self::on_export_html))
            .on_action(cx.listener(Self::on_export_pdf))
            .on_action(cx.listener(Self::on_toggle_pane_kind))
            .on_action(cx.listener(Self::on_toggle_maximize_pane))
            .on_action(cx.listener(Self::on_toggle_search))
            .on_action(cx.listener(Self::on_toggle_replace))
            .on_action(cx.listener(Self::on_find_next))
            .on_action(cx.listener(Self::on_find_previous))
            .on_action(cx.listener(Self::on_replace_current))
            .on_action(cx.listener(Self::on_replace_all))
            .on_action(cx.listener(Self::on_page_up))
            .on_action(cx.listener(Self::on_page_down))
            .on_action(cx.listener(Self::on_jump_to_top))
            .on_action(cx.listener(Self::on_jump_to_bottom))
            .child(self.render_editor_topbar(
                crate::plugin::TOPBAR_ICON_PREFIX,
                &theme,
                leaf_count,
                is_maximized,
                cx,
            ));
        let content_body_editor = cx.entity().downgrade();
        let content_drop_editor = cx.entity().downgrade();

        let mut content_body = div()
            .id(("editor-content-body", panel_id.as_usize()))
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .relative()
            .on_drag_move::<crate::layout::tab_drag::DraggedTab>({
                let content_body_editor = content_body_editor.clone();
                move |event, _window, cx| {
                    let bounds = event.bounds;
                    let pos = event.event.position;
                    if bounds.size.width > px(0.0) && bounds.size.height > px(0.0) {
                        let rel_x = f32::from(pos.x - bounds.origin.x) / f32::from(bounds.size.width);
                        let rel_y = f32::from(pos.y - bounds.origin.y) / f32::from(bounds.size.height);
                        let target = crate::layout::tab_drag::calc_tab_dock_target(rel_x, rel_y);
                        let shift_held = event.event.modifiers.shift;
                        let pointer_pos = point(pos.x - bounds.origin.x, pos.y - bounds.origin.y);

                        let drag = event.drag(cx);
                        let drag_view = drag.drag_view.clone();
                        let active_hover_editor = drag.active_hover_editor.clone();

                        // 1. Update the dragged tab view so its follow card expands into the unified card
                        drag_view.update(cx, |view, cx| {
                            view.set_hover(Some(crate::layout::tab_drag::TabDragHoverInfo {
                                target,
                                shift_held,
                            }));
                            cx.notify();
                        });

                        // 2. Ensure only the current editor shows the partition wireframe
                        if let Ok(mut active_guard) = active_hover_editor.lock() {
                            if let Some(prev_weak) = active_guard.as_ref() {
                                if let Some(prev_ed) = prev_weak.upgrade() {
                                    if prev_weak != &content_body_editor {
                                        let _ = prev_ed.update(cx, |ed, cx| {
                                            ed.tab_drag_hover = None;
                                            cx.notify();
                                        });
                                    }
                                }
                            }
                            *active_guard = Some(content_body_editor.clone());
                        }

                        // 3. Update this editor's tab_drag_hover
                        let _ = content_body_editor.update(cx, |ed, cx| {
                            ed.tab_drag_hover = Some(crate::layout::tab_drag::TabDragHoverState {
                                target,
                                shift_held,
                                pointer_pos,
                            });
                            cx.notify();
                        });
                    }
                }
            })
            .on_drop::<crate::layout::tab_drag::DraggedTab>({
                let content_drop_editor = content_drop_editor.clone();
                move |dragged, _window, cx| {
                    if let Ok(mut active_guard) = dragged.active_hover_editor.lock() {
                        *active_guard = None;
                    }
                    dragged.drag_view.update(cx, |view, cx| {
                        view.set_hover(None);
                        cx.notify();
                    });
                    let _ = content_drop_editor.update(cx, |ed, cx| {
                        let hover = ed.tab_drag_hover.take();
                        if let Some(hover) = hover {
                            match hover.target {
                                crate::layout::tab_drag::TabDockTarget::MergeCenter => {
                                    if dragged.source_panel_id == ed.panel_id {
                                        let last = ed.session.tab_count().saturating_sub(1);
                                        ed.reorder_tab(dragged.source_tab_index, last, cx);
                                    } else {
                                        let source_panel = dragged.source_panel_id;
                                        let tab_idx = dragged.source_tab_index;
                                        ed.defer_host_action(cx, move |host, cx| {
                                            host.split_editor_with_tab(
                                                source_panel,
                                                tab_idx,
                                                splitter::Direction::Right,
                                                false,
                                                cx,
                                            );
                                        });
                                    }
                                }
                                crate::layout::tab_drag::TabDockTarget::InnerPane(dir) => {
                                    if dragged.source_panel_id == ed.panel_id {
                                        if dragged.source_tab_index != ed.session.active_tab_index() {
                                            ed.activate_tab(dragged.source_tab_index, cx);
                                        }
                                        let axis = match dir {
                                            splitter::Direction::Up | splitter::Direction::Down => splitter::SplitAxis::Vertical,
                                            splitter::Direction::Left | splitter::Direction::Right => splitter::SplitAxis::Horizontal,
                                        };
                                        let active_pane = ed.active_pane_id();
                                        ed.split_pane_with_ratio(active_pane, axis, 0.5);
                                    }
                                }
                                crate::layout::tab_drag::TabDockTarget::OuterEditor(dir) => {
                                    let source_panel = dragged.source_panel_id;
                                    let tab_idx = dragged.source_tab_index;
                                    let copy_tab = hover.shift_held;
                                    ed.defer_host_action(cx, move |host, cx| {
                                        host.split_editor_with_tab(
                                            source_panel,
                                            tab_idx,
                                            dir,
                                            copy_tab,
                                            cx,
                                        );
                                    });
                                }
                            }
                        }
                        cx.notify();
                    });
                }
            })
            .child(self.render_editor_pane_layout(&theme, &strings, window, cx));

        if let Some(hover) = &self.tab_drag_hover {
            content_body = content_body.child(crate::layout::tab_drag::render_tab_drag_compass(
                hover,
                &theme,
            ));
        }

        let base = base
            .child(content_body)
            .child(self.render_editor_bottombar(&theme, &strings, cx));

        let base =
            if let Some(search_overlay) = self.render_search_panel_overlay(&theme, window, cx) {
                base.child(search_overlay)
            } else {
                base
            };

        base.into_any_element()
    }
}
