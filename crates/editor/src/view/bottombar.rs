//! Bottom status bar of an Editor area: mode pill, cursor position, word count, and split/close controls.

use gpui::prelude::*;
use gpui::*;
use splitter::SplitAxis;
use theme::Theme;
use ui::button::{icon_chip_button, small_pill_button, toolbar_icon_size};
use ui::menu_item::menu_item;
use ui::popover::menu_panel;

fn bottombar_container(c: &theme::ThemeColors, height: f32, padding_x: f32) -> Div {
    div()
        .h(px(height))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(padding_x))
        .bg(c.dialog_surface)
        .border_t_1()
        .border_color(c.dialog_border)
}

use config::language::I18nStrings;
use config::settings::PluginSettings;
use editor_contracts::{PaneId, PaneKind, PaneRegistry};

use crate::editor::Editor;
use crate::settings::EditorSettings;

/// Render a cursor-position label (e.g. `12 : 47`).
pub fn render_cursor((line, col): (usize, usize), theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let label = format!("{} : {}", line, col);

    div()
        .text_size(px(d.bottombar_text_size))
        .text_color(c.bottombar_text)
        .child(label)
        .into_any_element()
}

/// Render a word-count label, optionally showing selection vs total.
pub fn render_word_count(
    selection_count: Option<usize>,
    total_count: usize,
    theme: &Theme,
    strings: &I18nStrings,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let label = if let Some(sel) = selection_count {
        format!(
            "{} / {} {}",
            sel, total_count, strings.status_bar_word_count_suffix
        )
    } else {
        format!("{} {}", total_count, strings.status_bar_word_count_suffix)
    };

    div()
        .text_size(px(d.bottombar_text_size))
        .text_color(c.bottombar_text_dim)
        .child(label)
        .into_any_element()
}

impl Editor {
    /// Bottom bar of an Editor area: pane switch, split/close
    /// controls, cursor position and word count.
    pub(crate) fn render_editor_bottombar(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let prefs = self.bottombar_settings(cx);
        if !prefs.status_bar_enabled {
            return div().into_any_element();
        }
        let c = &theme.colors;
        let d = &theme.dimensions;

        let panel_id = self.panel_id;
        let inner_leaf_count = self.session().root.tree.count_leaves();

        let focused_pane_id = self
            .focused_pane_id
            .or_else(|| self.session().root.tree.first_leaf_id().map(PaneId));
        let focused_kind =
            focused_pane_id.and_then(|pane_id| self.session().root.tree.find_leaf_kind(pane_id.0));

        let mut left_items: Vec<AnyElement> = Vec::new();
        let mut right_items: Vec<AnyElement> = Vec::new();

        if let (Some(pane_id), Some(focused_kind)) = (focused_pane_id, focused_kind.clone()) {
            let toggle_editor = cx.entity().downgrade();
            let label = editor_contracts::PaneRegistry::registered(focused_kind.clone())
                .ok()
                .flatten()
                .map(|descriptor| descriptor.display_name().to_string())
                .unwrap_or_else(|| focused_kind.as_str().to_string());
            let dropdown_open = self
                .session()
                .root
                .tree
                .find_leaf(pane_id.0)
                .is_some_and(|panel| panel.open_dropdown);
            let mode_pill = small_pill_button(c, d)
                .text_size(px(11.0))
                .text_color(c.text_default)
                .opacity(1.0)
                .when(dropdown_open, |this| this.bg(c.panel_row_hover))
                .child(label)
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = toggle_editor.update(cx, |ed, cx| {
                        ed.toggle_pane_dropdown(pane_id, cx);
                        cx.notify();
                    });
                });
            left_items.push(mode_pill.into_any_element());
        }

        if prefs.show_cursor_position {
            left_items.push(
                div()
                    .text_size(px(11.0))
                    .text_color(c.bottombar_text_dim)
                    .child("\u{2502}")
                    .into_any_element(),
            );
            left_items.push(render_cursor(self.active_pane_cursor_position(cx), theme));
        }

        if prefs.show_word_count {
            let total_count = self.active_tab_word_count(cx);
            let selection_count = None;
            right_items.push(render_word_count(
                selection_count,
                total_count,
                theme,
                strings,
            ));
        }

        let is_pane_maximized = focused_pane_id
            .and_then(|id| self.session().root.tree.find_leaf(id.0))
            .is_some_and(|p| p.maximized);

        if let (Some(pane_id), Some(_)) = (focused_pane_id, focused_kind.clone()) {
            let editor = cx.entity().downgrade();
            let btn_icon_size = toolbar_icon_size(d.bottombar_height);

            // Splitting is disabled while the pane is maximized (mirrors the
            // window-level panels), so the split buttons are hidden.
            if !is_pane_maximized {
                let split_h_editor = editor.clone();
                right_items.push(
                    icon_chip_button(c, d)
                        .child(
                            svg()
                                .path("plugin://splitype.editor/bottombar/split-h.svg")
                                .size(px(btn_icon_size))
                                .text_color(c.dialog_muted),
                        )
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = split_h_editor.update(cx, |ed, cx| {
                                ed.split_pane(pane_id, SplitAxis::Horizontal);
                                cx.notify();
                            });
                        })
                        .into_any_element(),
                );

                let split_v_editor = editor.clone();
                right_items.push(
                    icon_chip_button(c, d)
                        .child(
                            svg()
                                .path("plugin://splitype.editor/bottombar/split-v.svg")
                                .size(px(btn_icon_size))
                                .text_color(c.dialog_muted),
                        )
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = split_v_editor.update(cx, |ed, cx| {
                                ed.split_pane(pane_id, SplitAxis::Vertical);
                                cx.notify();
                            });
                        })
                        .into_any_element(),
                );
            }

            if inner_leaf_count > 1 || is_pane_maximized {
                let max_editor = editor.clone();
                right_items.push(
                    icon_chip_button(c, d)
                        .child(
                            svg()
                                .path(if is_pane_maximized {
                                    "plugin://splitype.editor/bottombar/restore.svg"
                                } else {
                                    "plugin://splitype.editor/bottombar/maximize.svg"
                                })
                                .size(px(btn_icon_size))
                                .text_color(c.dialog_muted),
                        )
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = max_editor.update(cx, |ed, cx| {
                                ed.toggle_pane_maximize(pane_id);
                                cx.notify();
                            });
                        })
                        .into_any_element(),
                );
            }

            if inner_leaf_count > 1 {
                let close_editor = editor.clone();
                right_items.push(
                    icon_chip_button(c, d)
                        .child(
                            svg()
                                .path("plugin://splitype.editor/bottombar/close.svg")
                                .size(px(btn_icon_size))
                                .text_color(c.dialog_muted),
                        )
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = close_editor.update(cx, |ed, cx| {
                                ed.close_pane(pane_id);
                                if ed.focused_pane_id == Some(pane_id) {
                                    ed.focused_pane_id = None;
                                }
                                cx.notify();
                            });
                        })
                        .into_any_element(),
                );
            }
        }

        let mut bar = bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(ElementId::Name(
                format!("panel-bottombar-{panel_id}").into(),
            ))
            .relative()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .children(left_items),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .children(right_items),
            );

        if let Some(pane_id) = focused_pane_id {
            let dropdown_open = self
                .session
                .root
                .tree
                .find_leaf(pane_id.0)
                .is_some_and(|panel| panel.open_dropdown);
            if dropdown_open && let Some(focused_kind) = focused_kind.clone() {
                let menu = self.render_pane_type_dropdown_menu(pane_id, focused_kind, theme, cx);
                bar = bar.child(menu);
            }
        }

        bar.into_any_element()
    }

    pub(crate) fn render_pane_type_dropdown_menu(
        &mut self,
        pane_id: PaneId,
        current_kind: PaneKind,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        let available_descriptors = PaneRegistry::registered_descriptors().unwrap_or_default();

        menu_panel(c, d)
            .id(("pane-dropdown-overlay", pane_id.0))
            .absolute()
            .occlude()
            .bottom(px(d.bottombar_height + 4.0))
            .left(px(8.0))
            .w(px(d.menu_panel_width))
            .children(
                available_descriptors
                    .into_iter()
                    .enumerate()
                    .map(|(idx, desc)| {
                        let kind_id = desc.kind();
                        let is_current = kind_id == current_kind;
                        let option_editor = editor.clone();
                        let display_name = desc.display_name();
                        menu_item(("pane-type-opt", idx), c, d)
                            .w_full()
                            .justify_between()
                            .bg(c.dialog_surface)
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_button_weight.to_font_weight())
                            .text_color(if is_current {
                                c.dialog_primary_button_bg
                            } else {
                                c.dialog_secondary_button_text
                            })
                            .child(display_name)
                            .child(if is_current {
                                svg()
                                    .path("plugin://splitype.editor/topbar/check.svg")
                                    .size(px(13.0))
                                    .text_color(c.dialog_primary_button_bg)
                                    .into_any_element()
                            } else {
                                div().w(px(13.0)).into_any_element()
                            })
                            .on_click(move |_event, _window, cx| {
                                let kind_id = kind_id.clone();
                                let _ = option_editor.update(cx, |ed, cx| {
                                    ed.select_pane_kind(pane_id, kind_id, cx);
                                    cx.notify();
                                });
                            })
                            .into_any_element()
                    }),
            )
            .into_any_element()
    }

    pub(crate) fn active_tab_word_count(&mut self, cx: &mut App) -> usize {
        let Some(buffer) = self.session.active_tab().map(|tab| tab.buffer.clone()) else {
            return 0;
        };
        buffer.update(cx, |buffer, _cx| buffer.word_count())
    }

    pub(crate) fn bottombar_settings(&self, cx: &App) -> EditorSettings {
        PluginSettings::<EditorSettings>::get(cx)
    }

    pub(crate) fn active_pane_cursor_position(&self, cx: &App) -> (usize, usize) {
        let active_pane = self.active_pane_id();
        if let Some(pos) = self
            .pane_state_ref(active_pane)
            .and_then(|state| state.pane().cursor_position(cx))
        {
            return pos;
        }
        (1, 1)
    }
}
