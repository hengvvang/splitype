//! Top bar of an Editor area: area type selector, split/close controls and tab bar.

use gpui::prelude::FluentBuilder;
use gpui::*;
use splitter::SplitAxis;
use theme::Theme;
use ui::button::{icon_chip_button, small_pill_button, toolbar_button_size, toolbar_icon_size};

use crate::editor::Editor;

fn topbar_icon(icon_prefix: &str, name: &str) -> SharedString {
    format!("{icon_prefix}/topbar/{name}.svg").into()
}

fn topbar_container(c: &theme::ThemeColors, height: f32, padding_x: f32) -> Div {
    div()
        .h(px(height))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(padding_x))
        .bg(c.dialog_surface)
        .border_b_1()
        .border_color(c.dialog_border)
}

impl Editor {
    /// Top bar of an Editor area: type selector and split/close controls plus the Editor-specific tab bar.
    pub(crate) fn render_editor_topbar(
        &mut self,
        icon_prefix: &'static str,
        theme: &Theme,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let panel_id = self.panel_id;
        let editor = cx.entity().downgrade();

        let type_editor = editor.clone();
        let is_active_editor = self.is_active_panel;
        let type_button = small_pill_button(c, d)
            .id(("panel-topbar-type", panel_id.as_usize()))
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child("Editor")
            .when(is_active_editor, |this| {
                this.child(
                    svg()
                        .path(topbar_icon(icon_prefix, "active"))
                        .size(px(d.topbar_height * 0.5))
                        .text_color(c.focus_accent),
                )
            })
            .on_click(move |_event, _window, cx| {
                let _ = type_editor.update(cx, |editor, cx| {
                    editor.defer_host_action(cx, move |host, cx| {
                        host.toggle_panel_dropdown(panel_id, cx);
                    });
                    cx.notify();
                });
            });

        let btn_icon_size = toolbar_icon_size(d.topbar_height);

        let split_h_editor = editor.clone();
        let split_h_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-h", panel_id.as_usize()))
            .child(
                svg()
                    .path(topbar_icon(icon_prefix, "split-h"))
                    .size(px(btn_icon_size))
                    .text_color(if is_maximized {
                        c.dialog_muted.opacity(0.3)
                    } else {
                        c.dialog_muted
                    }),
            )
            .when(!is_maximized, |this| {
                this.on_click(move |_event, _window, cx| {
                    let _ = split_h_editor.update(cx, |editor, cx| {
                        editor.defer_host_action(cx, move |host, cx| {
                            host.split_panel(panel_id, SplitAxis::Horizontal, 0.5, true, cx);
                        });
                        cx.notify();
                    });
                })
            });

        let split_v_editor = editor.clone();
        let split_v_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-v", panel_id.as_usize()))
            .child(
                svg()
                    .path(topbar_icon(icon_prefix, "split-v"))
                    .size(px(btn_icon_size))
                    .text_color(if is_maximized {
                        c.dialog_muted.opacity(0.3)
                    } else {
                        c.dialog_muted
                    }),
            )
            .when(!is_maximized, |this| {
                this.on_click(move |_event, _window, cx| {
                    let _ = split_v_editor.update(cx, |editor, cx| {
                        editor.defer_host_action(cx, move |host, cx| {
                            host.split_panel(panel_id, SplitAxis::Vertical, 0.5, true, cx);
                        });
                        cx.notify();
                    });
                })
            });

        let mut actions = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(split_v_button)
            .child(split_h_button);

        if leaf_count > 1 {
            let max_editor = editor.clone();
            let max_button = icon_chip_button(c, d)
                .id(("panel-topbar-max", panel_id.as_usize()))
                .child(
                    svg()
                        .path(if is_maximized {
                            topbar_icon(icon_prefix, "restore")
                        } else {
                            topbar_icon(icon_prefix, "maximize")
                        })
                        .size(px(btn_icon_size))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_editor.update(cx, |ed, cx| {
                        ed.defer_host_action(cx, move |host, cx| {
                            host.toggle_panel_maximize(panel_id, cx);
                        });
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = icon_chip_button(c, d)
                .id(("panel-topbar-close", panel_id.as_usize()))
                .child(
                    svg()
                        .path(topbar_icon(icon_prefix, "close"))
                        .size(px(btn_icon_size))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.defer_host_action(cx, move |host, cx| {
                            host.request_close_panel(panel_id, cx);
                        });
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        let mut left_section = div().flex().items_center().gap(px(8.0)).child(type_button);

        {
            let list = self.tab_list_mut();
            let active_tab = list.active_index();
            let tab_infos: Vec<(String, bool, bool, editor_contracts::DocumentId)> = list
                .iter()
                .map(|tab| {
                    let buffer = tab.buffer.read(cx);
                    let name = buffer
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "Untitled".to_string());
                    (name, tab.is_transient(), buffer.dirty, buffer.id)
                })
                .collect();

            let mut tab_elements: Vec<AnyElement> = Vec::new();
            for (index, (file_name, is_transient, _is_dirty, doc_id)) in
                tab_infos.iter().enumerate()
            {
                // Visual insertion indicator before tab `index` when targeted
                if self.tab_reorder_target == Some(index) {
                    tab_elements.push(
                        div()
                            .id(("tab-insert-indicator", index))
                            .w(px(2.0))
                            .h(px(toolbar_button_size(d.topbar_height) - 4.0))
                            .rounded(px(1.0))
                            .bg(c.focus_accent)
                            .into_any_element(),
                    );
                }

                let is_active = index == active_tab;
                let tab_bg = if is_active {
                    c.panel_row_hover
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                };
                let tab_text = if is_active {
                    c.text_default
                } else {
                    c.dialog_muted
                };

                let tab_editor = editor.clone();
                let close_editor = editor.clone();
                let drop_editor = editor.clone();
                let tab_drag_editor = editor.clone();

                let drag_view =
                    cx.new(|_| crate::layout::tab_drag::DraggedTabView::new(file_name.clone()));
                let active_hover_editor = std::sync::Arc::new(std::sync::Mutex::new(None));
                let drag_payload = crate::layout::tab_drag::DraggedTab {
                    source_panel_id: panel_id,
                    source_tab_index: index,
                    document_id: *doc_id,
                    title: SharedString::from(file_name.clone()),
                    drag_view: drag_view.clone(),
                    active_hover_editor: active_hover_editor.clone(),
                };

                let mut title_div = div()
                    .text_color(tab_text)
                    .cursor_pointer()
                    .child(file_name.clone());

                if *is_transient {
                    title_div = title_div.italic();
                }

                let height = toolbar_button_size(d.topbar_height);
                let tab_button = div()
                    .id(("editor-tab", index))
                    .h(px(height))
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .rounded(px(d.tab_radius))
                    .bg(tab_bg)
                    .hover(|this| this.bg(c.panel_row_hover))
                    .text_size(px(11.0))
                    .cursor_pointer()
                    .on_drag(drag_payload, {
                        let drag_view = drag_view.clone();
                        move |_payload, _offset, _window, _cx| drag_view.clone()
                    })
                    .on_drag_move::<crate::layout::tab_drag::DraggedTab>({
                        let tab_drag_editor = tab_drag_editor.clone();
                        move |event, _window, cx| {
                            let bounds = event.bounds;
                            let pos = event.event.position;
                            if !bounds.contains(&pos) {
                                return;
                            }
                            let (drag_view, active_hover_editor) = {
                                let drag = event.drag(cx);
                                (drag.drag_view.clone(), drag.active_hover_editor.clone())
                            };

                            if let Ok(mut active_guard) = active_hover_editor.lock() {
                                if let Some(prev_weak) = active_guard.take() {
                                    if let Some(prev_ed) = prev_weak.upgrade() {
                                        prev_ed.update(cx, |ed, cx| {
                                            ed.tab_drag_hover = None;
                                            cx.notify();
                                        });
                                    }
                                }
                            }
                            drag_view.update(cx, |view, cx| {
                                view.set_hover(None);
                                cx.notify();
                            });

                            let target_idx = if pos.x < bounds.origin.x + bounds.size.width / 2.0 {
                                index
                            } else {
                                index + 1
                            };

                            let _ = tab_drag_editor.update(cx, |ed, cx| {
                                let mut changed = false;
                                if ed.tab_drag_hover.is_some() {
                                    ed.tab_drag_hover = None;
                                    changed = true;
                                }
                                if ed.tab_reorder_target != Some(target_idx) {
                                    ed.tab_reorder_target = Some(target_idx);
                                    changed = true;
                                }
                                if changed {
                                    cx.notify();
                                }
                            });
                        }
                    })
                    .on_drop::<crate::layout::tab_drag::DraggedTab>({
                        let drop_editor = drop_editor.clone();
                        move |dragged, _window, cx| {
                            if let Ok(mut active_guard) = dragged.active_hover_editor.lock() {
                                *active_guard = None;
                            }
                            dragged.drag_view.update(cx, |view, cx| {
                                view.set_hover(None);
                                cx.notify();
                            });
                            let _ = drop_editor.update(cx, |ed, cx| {
                                let target = ed.tab_reorder_target.take().unwrap_or(index);
                                if dragged.source_panel_id == panel_id {
                                    let from = dragged.source_tab_index;
                                    let to = if target > from {
                                        target.saturating_sub(1)
                                    } else {
                                        target
                                    };
                                    ed.reorder_tab(from, to, cx);
                                }
                                ed.tab_drag_hover = None;
                                cx.notify();
                            });
                        }
                    })
                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                        let is_double = event.click_count > 1;
                        let _ = tab_editor.update(cx, |ed, cx| {
                            ed.defer_host_action(cx, move |host, cx| {
                                host.activate_panel(panel_id, cx);
                            });
                            if is_double {
                                if let Some(tab) = ed.session.tab_mut(index) {
                                    tab.persist();
                                }
                            }
                            ed.activate_tab(index, cx);
                            cx.notify();
                        });
                    })
                    .child(title_div)
                    .child(
                        div()
                            .size(px(12.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(d.tab_close_button_radius))
                            .hover(|this| this.bg(c.panel_row_hover))
                            .cursor_pointer()
                            .child(
                                svg()
                                    .path(topbar_icon(icon_prefix, "close"))
                                    .size(px(8.0))
                                    .text_color(c.dialog_muted),
                            )
                            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                cx.stop_propagation();
                                let _ = close_editor.update(cx, |ed, cx| {
                                    ed.request_close_tab(index, cx);
                                    cx.notify();
                                });
                            }),
                    );

                tab_elements.push(tab_button.into_any_element());
            }

            // Visual insertion indicator at the end of tab list when targeted
            if self.tab_reorder_target == Some(tab_infos.len()) {
                tab_elements.push(
                    div()
                        .id(("tab-insert-indicator", tab_infos.len()))
                        .w(px(2.0))
                        .h(px(toolbar_button_size(d.topbar_height) - 4.0))
                        .rounded(px(1.0))
                        .bg(c.focus_accent)
                        .into_any_element(),
                );
            }

            let add_editor = editor.clone();
            tab_elements.push(
                div()
                    .size(px(toolbar_button_size(d.topbar_height)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(d.icon_button_radius))
                    .hover(|this| this.bg(c.panel_row_hover))
                    .cursor_pointer()
                    .text_color(c.dialog_muted)
                    .child(
                        svg()
                            .path("plugin://splitype.editor/topbar/plus.svg")
                            .size(px(btn_icon_size))
                            .text_color(c.dialog_muted),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = add_editor.update(cx, |ed, cx| {
                            ed.defer_host_action(cx, move |host, cx| {
                                host.activate_panel(panel_id, cx);
                            });
                            ed.new_untitled_tab(cx);
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );

            let bar_drop_editor = editor.clone();
            let bar_drag_editor = editor.clone();
            left_section = left_section.child(
                div()
                    .id(("tab-bar-container", panel_id.as_usize()))
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .on_drag_move::<crate::layout::tab_drag::DraggedTab>({
                        let bar_drag_editor = bar_drag_editor.clone();
                        move |event, _window, cx| {
                            if !event.bounds.contains(&event.event.position) {
                                return;
                            }
                            let (drag_view, active_hover_editor) = {
                                let drag = event.drag(cx);
                                (drag.drag_view.clone(), drag.active_hover_editor.clone())
                            };

                            if let Ok(mut active_guard) = active_hover_editor.lock() {
                                if let Some(prev_weak) = active_guard.take() {
                                    if let Some(prev_ed) = prev_weak.upgrade() {
                                        prev_ed.update(cx, |ed, cx| {
                                            ed.tab_drag_hover = None;
                                            cx.notify();
                                        });
                                    }
                                }
                            }
                            drag_view.update(cx, |view, cx| {
                                view.set_hover(None);
                                cx.notify();
                            });
                            let _ = bar_drag_editor.update(cx, |ed, cx| {
                                let tab_count = ed.session.tab_count();
                                let mut changed = false;
                                if ed.tab_drag_hover.is_some() {
                                    ed.tab_drag_hover = None;
                                    changed = true;
                                }
                                if ed.tab_reorder_target.is_none() {
                                    ed.tab_reorder_target = Some(tab_count);
                                    changed = true;
                                }
                                if changed {
                                    cx.notify();
                                }
                            });
                        }
                    })
                    .on_drop::<crate::layout::tab_drag::DraggedTab>({
                        let bar_drop_editor = bar_drop_editor.clone();
                        move |dragged, _window, cx| {
                            if let Ok(mut active_guard) = dragged.active_hover_editor.lock() {
                                *active_guard = None;
                            }
                            dragged.drag_view.update(cx, |view, cx| {
                                view.set_hover(None);
                                cx.notify();
                            });
                            let _ = bar_drop_editor.update(cx, |ed, cx| {
                                let target = ed.tab_reorder_target.take();
                                if dragged.source_panel_id == panel_id {
                                    let from = dragged.source_tab_index;
                                    let last = ed.session.tab_count().saturating_sub(1);
                                    let to = match target {
                                        Some(t) if t > from => (t.saturating_sub(1)).min(last),
                                        Some(t) => t.min(last),
                                        None => last,
                                    };
                                    ed.reorder_tab(from, to, cx);
                                }
                                ed.tab_drag_hover = None;
                                cx.notify();
                            });
                        }
                    })
                    .children(tab_elements),
            );
        }

        let topbar_drag_editor = editor.clone();
        topbar_container(c, d.topbar_height, 8.0)
            .id(("panel-topbar", panel_id.as_usize()))
            .on_drag_move::<crate::layout::tab_drag::DraggedTab>({
                let topbar_drag_editor = topbar_drag_editor.clone();
                move |event, _window, cx| {
                    let bounds = event.bounds;
                    let pos = event.event.position;
                    if !bounds.contains(&pos) {
                        let _ = topbar_drag_editor.update(cx, |ed, cx| {
                            if ed.tab_reorder_target.is_some() {
                                ed.tab_reorder_target = None;
                                cx.notify();
                            }
                        });
                        return;
                    }
                    let (drag_view, active_hover_editor) = {
                        let drag = event.drag(cx);
                        (drag.drag_view.clone(), drag.active_hover_editor.clone())
                    };

                    if let Ok(mut active_guard) = active_hover_editor.lock() {
                        if let Some(prev_weak) = active_guard.take() {
                            if let Some(prev_ed) = prev_weak.upgrade() {
                                prev_ed.update(cx, |ed, cx| {
                                    ed.tab_drag_hover = None;
                                    cx.notify();
                                });
                            }
                        }
                    }
                    drag_view.update(cx, |view, cx| {
                        view.set_hover(None);
                        cx.notify();
                    });
                    let _ = topbar_drag_editor.update(cx, |ed, cx| {
                        if ed.tab_drag_hover.is_some() {
                            ed.tab_drag_hover = None;
                            cx.notify();
                        }
                    });
                }
            })
            .child(left_section)
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }
}
