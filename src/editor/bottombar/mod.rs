//! Bottom status bar of an Editor area: mode pill, cursor position, word
//! count, custom buttons, and split/close controls.
//!
//! Hover state lives in [`state`]; the pure word counter lives in
//! [`words`]. The shared bar container comes from `crate::ui`.

pub(crate) mod state;
pub(crate) mod words;

use crate::ui::bottombar::bottombar_container;

use crate::ui::button::{icon_chip_button, small_pill_button};

use gpui::prelude::*;
use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::controller::EditorMode;
use crate::editor::session::InnerPanelLocation;
use crate::infra::config::settings::{EditorSettings, StatusBarButton, StatusBarSettings};
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::{Theme, ThemeColors, ThemeDimensions};
use crate::splitter::Axis;

use state::BottombarState;
use words::count_words;

/// Render a cursor-position label (e.g. `12 : 47`).
pub fn render_cursor((line, col): (usize, usize), theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let label = format!("{} : {}", &line.to_string(), &col.to_string());

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

/// Status bar button — transparent with a state-driven hover background.
fn bottombar_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(d.bottombar_height - 4.0))
        .px(px(6.0))
        .flex()
        .items_center()
        .rounded(px(4.0))
        .cursor_pointer()
        .text_size(px(d.bottombar_text_size))
        .text_color(c.bottombar_text)
}
pub fn render_custom_button(
    state: &mut BottombarState,
    button: &StatusBarButton,
    theme: &Theme,
    cx: &mut Context<Editor>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let id = button.id.clone();
    let hovered = state.custom_button_hovered.as_deref() == Some(&button.id);

    bottombar_button(
        ElementId::Name(format!("bottombar-custom-button-{}", button.id).into()),
        c,
        d,
    )
    .bg(if hovered {
        c.bottombar_button_hover
    } else {
        hsla(0., 0., 0., 0.)
    })
    .cursor_pointer()
    .text_size(px(d.bottombar_text_size))
    .text_color(c.bottombar_text)
    .child(button.label.clone())
    .on_hover(cx.listener(
        move |editor: &mut Editor,
              hovered: &bool,
              _window: &mut Window,
              cx: &mut Context<Editor>| {
            if *hovered {
                editor.bottombar_state.custom_button_hovered = Some(id.clone());
            } else if editor.bottombar_state.custom_button_hovered.as_deref() == Some(&id) {
                editor.bottombar_state.custom_button_hovered = None;
            }
            cx.notify();
        },
    ))
    .into_any_element()
}

#[allow(dead_code)]
pub fn render_sidebar_toggle(
    state: &mut BottombarState,
    _is_open: bool,
    theme: &Theme,
    strings: &I18nStrings,
    cx: &mut Context<Editor>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    bottombar_button("bottombar-sidebar-toggle", c, d)
        .bg(if state.sidebar_hovered {
            c.bottombar_button_hover
        } else {
            hsla(0., 0., 0., 0.)
        })
        .cursor_pointer()
        .text_size(px(d.bottombar_text_size))
        .text_color(c.bottombar_text)
        .child(strings.status_bar_files.clone())
        .on_hover(cx.listener(
            |editor: &mut Editor,
             hovered: &bool,
             _window: &mut Window,
             cx: &mut Context<Editor>| {
                editor.bottombar_state.sidebar_hovered = *hovered;
                cx.notify();
            },
        ))
        .on_click(cx.listener(
            |editor: &mut Editor,
             _: &gpui::ClickEvent,
             window: &mut Window,
             cx: &mut Context<Editor>| {
                editor.toggle_explorer_drawer(window, cx);
            },
        ))
        .into_any_element()
}

#[allow(dead_code)]
pub fn render_mode_switch(
    state: &mut BottombarState,
    view_mode: EditorMode,
    theme: &Theme,
    strings: &I18nStrings,
    cx: &mut Context<Editor>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let label = match view_mode {
        EditorMode::SourceCode => strings.status_bar_mode_rendered.clone(),
        EditorMode::Wysiwyg => strings.status_bar_mode_source.clone(),
    };

    bottombar_button("bottombar-mode-switch", c, d)
        .bg(if state.mode_hovered {
            c.bottombar_button_hover
        } else {
            hsla(0., 0., 0., 0.)
        })
        .cursor_pointer()
        .text_size(px(d.bottombar_text_size))
        .text_color(c.bottombar_text)
        .child(label)
        .on_hover(cx.listener(
            |editor: &mut Editor,
             hovered: &bool,
             _window: &mut Window,
             cx: &mut Context<Editor>| {
                editor.bottombar_state.mode_hovered = *hovered;
                cx.notify();
            },
        ))
        .on_click(cx.listener(
            |editor: &mut Editor,
             _: &gpui::ClickEvent,
             _window: &mut Window,
             cx: &mut Context<Editor>| {
                editor.toggle_view_mode_from_ui(cx);
            },
        ))
        .into_any_element()
}

// ── Editor methods ────────────────────────────────────────────────────────

impl Editor {
    #[allow(dead_code)]
    pub(crate) fn render_bottombar(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let prefs = self.bottombar_settings(cx);
        if !prefs.enabled {
            return None;
        }

        let c = &theme.colors;
        let d = &theme.dimensions;

        let left_items: Vec<AnyElement> = Vec::new();

        let mut right_items: Vec<AnyElement> = Vec::new();

        if prefs.show_cursor_position && self.tab().mode == EditorMode::SourceCode {
            right_items.push(render_cursor(
                self.compute_source_cursor_position(cx),
                theme,
            ));
        }

        if prefs.show_word_count {
            let text = self.serialized_document_text(cx);
            let total_count = count_words(&text);
            let selection_count = self.selected_markdown_text(cx).as_deref().map(count_words);
            right_items.push(render_word_count(
                selection_count,
                total_count,
                theme,
                strings,
            ));
        }

        for button in &prefs.custom_buttons {
            right_items.push(render_custom_button(
                &mut self.bottombar_state,
                button,
                theme,
                cx,
            ));
        }

        let bar = bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id("bottombar")
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(d.bottombar_item_gap))
                    .children(left_items),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(d.bottombar_item_gap))
                    .children(right_items),
            )
            .into_any_element();

        Some(bar)
    }

    /// Bottom bar of an Editor area: inner-panel switch, split/close
    /// controls, cursor position and word count.
    pub(crate) fn render_editor_bottombar(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let prefs = self.bottombar_settings(cx);

        // The status bar renders after the area container restored the
        // routing hint; set it for this area so tab()/doc() reads below hit
        // THIS editor's document.
        let previous = self.current_tab_area;
        self.current_tab_area = Some(area_id);

        let inner_leaf_count = self
            .ensure_editor_session(area_id)
            .root.tree
            .count_leaves();

        let focused = self.focused_editor_inner_panel;
        let focused_panel_id = focused.and_then(|loc| {
            if loc.area_id == area_id {
                Some(loc.panel_id)
            } else {
                None
            }
        });
        let focused_kind = focused_panel_id.and_then(|panel_id| {
            self.ensure_editor_session(area_id)
                .root.tree
                .find_leaf_kind(panel_id)
        });

        let mut left_items: Vec<AnyElement> = Vec::new();
        let mut right_items: Vec<AnyElement> = Vec::new();

        // Mode pill on the left, always shown so the status bar stays
        // consistent across the two editor states. In the welcome state it
        // displays the outer mode itself ("Welcome") and is disabled; in the
        // editing state it displays the focused panel kind and opens the
        // panel-type dropdown.
        if let (Some(panel_id), Some(ftype)) = (focused_panel_id, focused_kind) {
            let editing = self.area_mode(area_id).is_editing();
            let toggle_editor = cx.entity().downgrade();
            let label = ftype.name().to_string();
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
                            ed.toggle_editor_inner_panel_dropdown(area_id, panel_id);
                            cx.notify();
                        });
                    });
            }
            left_items.push(mode_pill.into_any_element());
        }

        if self.area_has_tabs(area_id) && prefs.show_cursor_position {
            left_items.push(
                div()
                    .text_size(px(11.0))
                    .text_color(c.bottombar_text_dim)
                    .child("\u{2502}")
                    .into_any_element(),
            );
            left_items.push(render_cursor(
                self.compute_source_cursor_position(cx),
                theme,
            ));
        }

        if self.area_has_tabs(area_id) && prefs.show_word_count {
            let text = self.serialized_document_text_for(area_id, cx);
            let total_count = count_words(&text);
            let selection_count = self.selected_markdown_text(cx).as_deref().map(count_words);
            right_items.push(render_word_count(
                selection_count,
                total_count,
                theme,
                strings,
            ));
        }

        // Split / close buttons on the far right of the status bar. Available
        // even in the welcome state so the panels can be split before any
        // document is opened.
        if let (Some(panel_id), Some(_)) = (focused_panel_id, focused_kind) {
            let editor = cx.entity().downgrade();

            // Split H button.
            let split_h_editor = editor.clone();
            right_items.push(
                icon_chip_button(c, d)
                    .child(
                        svg()
                            .path("icons/editor/bottombar/split-h.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = split_h_editor.update(cx, |ed, cx| {
                            ed.split_editor_inner_panel(area_id, panel_id, Axis::Horizontal);
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );

            // Split V button.
            let split_v_editor = editor.clone();
            right_items.push(
                icon_chip_button(c, d)
                    .child(
                        svg()
                            .path("icons/editor/bottombar/split-v.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = split_v_editor.update(cx, |ed, cx| {
                            ed.split_editor_inner_panel(area_id, panel_id, Axis::Vertical);
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );

            // Close button (only when multiple panels).
            if inner_leaf_count > 1 {
                let close_editor = editor.clone();
                right_items.push(
                    icon_chip_button(c, d)
                        .child(
                            svg()
                                .path("icons/editor/bottombar/close.svg")
                                .size(px(14.0))
                                .text_color(c.dialog_muted),
                        )
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = close_editor.update(cx, |ed, cx| {
                                ed.close_editor_inner_panel(area_id, panel_id);
                                if ed.focused_editor_inner_panel
                                    == Some(InnerPanelLocation { area_id, panel_id })
                                {
                                    ed.focused_editor_inner_panel = None;
                                }
                                cx.notify();
                            });
                        })
                        .into_any_element(),
                );
            }
        }

        let bar = bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(ElementId::Name(format!("panel-bottombar-{area_id}").into()))
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
            .into_any_element();
        self.current_tab_area = previous;
        bar
    }

    pub(crate) fn bottombar_settings(&self, cx: &App) -> StatusBarSettings {
        EditorSettings::status_bar_settings(cx)
    }

    /// Returns (line, col), both 1-based, for the current caret.
    ///
    /// Block-local anchors count newlines in the anchored block plus every
    /// visible block before it, avoiding a full-document source mapping and
    /// raw-source rebuild on every frame the status bar is visible.
    pub(crate) fn compute_source_cursor_position(&self, cx: &App) -> (usize, usize) {
        use unicode_segmentation::UnicodeSegmentation;

        let snapshot = self.capture_source_selection_snapshot(cx);
        if let Some(anchor) = &snapshot.block_anchor
            && let Some(block) = self.block_entity_by_path(&anchor.path, cx)
        {
            let entity_id = block.entity_id();
            let mut before_lines = 0usize;
            let mut found = false;
            for visible in self.doc().blocks() {
                if visible.entity.entity_id() == entity_id {
                    found = true;
                    break;
                }
                before_lines += visible.entity.read(cx).display_text().matches('\n').count() + 1;
            }
            if found {
                let text = block.read(cx).display_text();
                let clamped = anchor.content_range.end.min(text.len());
                let safe = if text.is_char_boundary(clamped) {
                    clamped
                } else {
                    (0..=clamped)
                        .rev()
                        .find(|&i| text.is_char_boundary(i))
                        .unwrap_or(0)
                };
                let line = text[..safe].matches('\n').count() + 1;
                let last_newline = text[..safe].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let col = text[last_newline..safe].graphemes(true).count() + 1;
                return (before_lines + line, col);
            }
        }

        // Cross-block selections and runtime-only blocks (table cells) fall
        // back to the global source-range path.
        let cursor_offset = snapshot.range.end;
        let text = self.doc().to_raw_source(cx);
        // Snap to valid UTF-8 char boundary to avoid panics on multi-byte chars.
        let clamped = cursor_offset.min(text.len());
        let safe = if text.is_char_boundary(clamped) {
            clamped
        } else {
            (0..=clamped)
                .rev()
                .find(|&i| text.is_char_boundary(i))
                .unwrap_or(0)
        };

        let line = text[..safe].matches('\n').count() + 1;
        let last_newline = text[..safe].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = text[last_newline..safe].graphemes(true).count() + 1;
        (line, col)
    }
}
