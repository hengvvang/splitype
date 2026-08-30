//! Bottom status bar of an Editor area: mode pill, cursor position, word count, and split/close controls.

use gpui::prelude::*;
use gpui::*;
use splitter::SplitAxis;
use theme::Theme;
use ui::bottombar::bottombar_container;
use ui::button::{icon_chip_button, small_pill_button, toolbar_icon_size};

use config::language::I18nStrings;
use config::settings::{SettingsStore, StatusBarSettings};

use crate::editor::Editor;
use crate::view::words::count_words;

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
        let c = &theme.colors;
        let d = &theme.dimensions;
        let prefs = self.bottombar_settings(cx);

        let panel_id = self.panel_id;
        let inner_leaf_count = self.session().root.tree.count_leaves();

        let focused_pane_id = self.focused_pane_id;
        let focused_kind =
            focused_pane_id.and_then(|pane_id| self.session().root.tree.find_leaf_kind(pane_id.0));

        let mut left_items: Vec<AnyElement> = Vec::new();
        let mut right_items: Vec<AnyElement> = Vec::new();

        if let (Some(pane_id), Some(focused_kind)) = (focused_pane_id, focused_kind) {
            let mode = self.panel_mode();
            let editing = mode.is_editing();
            let toggle_editor = cx.entity().downgrade();
            let label = if editing {
                editor_model::PaneRegistry::global()
                    .lock()
                    .unwrap()
                    .get(focused_kind)
                    .map(|d| d.display_name().to_string())
                    .unwrap_or_else(|| focused_kind.as_str().to_string())
            } else {
                mode.name().to_string()
            };
            let mut mode_pill = small_pill_button(c, d)
                .text_size(px(11.0))
                .text_color(if editing {
                    c.text_default
                } else {
                    c.dialog_muted
                })
                .opacity(if editing { 1.0 } else { 0.6 })
                .child(label);
            if editing {
                mode_pill =
                    mode_pill.on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = toggle_editor.update(cx, |ed, cx| {
                            ed.toggle_pane_dropdown(pane_id, cx);
                            cx.notify();
                        });
                    });
            }
            left_items.push(mode_pill.into_any_element());
        }

        if self.has_tabs() && prefs.show_cursor_position {
            left_items.push(
                div()
                    .text_size(px(11.0))
                    .text_color(c.bottombar_text_dim)
                    .child("\u{2502}")
                    .into_any_element(),
            );
            left_items.push(render_cursor(
                self.active_pane_cursor_position(cx),
                theme,
            ));
        }

        if self.has_tabs() && prefs.show_word_count {
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

        if let (Some(pane_id), Some(_)) = (focused_pane_id, focused_kind) {
            let editor = cx.entity().downgrade();
            let btn_icon_size = toolbar_icon_size(d.bottombar_height);

            let split_h_editor = editor.clone();
            right_items.push(
                icon_chip_button(c, d)
                    .child(
                        svg()
                            .path("icons/editor/bottombar/split-h.svg")
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
                            .path("icons/editor/bottombar/split-v.svg")
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

            if inner_leaf_count > 1 || is_pane_maximized {
                let max_editor = editor.clone();
                right_items.push(
                    icon_chip_button(c, d)
                        .child(
                            svg()
                                .path(if is_pane_maximized {
                                    "icons/editor/bottombar/restore.svg"
                                } else {
                                    "icons/editor/bottombar/maximize.svg"
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
                                .path("icons/editor/bottombar/close.svg")
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

        bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(ElementId::Name(
                format!("panel-bottombar-{panel_id}").into(),
            ))
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
            )
            .into_any_element()
    }

    pub(crate) fn active_tab_word_count(&mut self, cx: &App) -> usize {
        let rev = self.tab().document_revision;
        if let Some((cached_rev, count)) = self.tab().cached_word_count {
            if cached_rev == rev {
                return count;
            }
        }
        let text = self.serialized_document_text(cx);
        let count = count_words(&text);
        self.tab_mut().cached_word_count = Some((rev, count));
        count
    }

    pub(crate) fn bottombar_settings(&self, cx: &App) -> StatusBarSettings {
        SettingsStore::settings(cx).status_bar
    }

    pub(crate) fn active_pane_cursor_position(&self, cx: &App) -> (usize, usize) {
        let active_pane = self.active_pane_id();
        if let Some(pos) = self
            .pane_state_ref(active_pane)
            .and_then(|state| state.pane.cursor_position(cx))
        {
            return pos;
        }
        (1, 1)
    }
}
