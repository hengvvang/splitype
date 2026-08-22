//! Bottom status bar of an Editor area: mode pill, cursor position, word
//! count, and split/close controls.
//!
//! The pure word counter lives in [`words`]. The shared bar container comes
//! from `crate::ui`.

pub(crate) mod words;

use crate::ui::bottombar::bottombar_container;

use crate::ui::button::{icon_chip_button, small_pill_button, toolbar_icon_size};

use gpui::prelude::*;
use gpui::*;

use crate::editor::controller::Editor;
use crate::infra::config::settings::{EditorSettings, StatusBarSettings};
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;
use crate::splitter::SplitAxis;

use words::count_words;

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
/// Returns an element, or `None` when the status bar itself is disabled.
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

// ── Editor methods ────────────────────────────────────────────────────────

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
            focused_pane_id.and_then(|pane_id| self.session().root.tree.find_leaf_kind(pane_id));

        let mut left_items: Vec<AnyElement> = Vec::new();
        let mut right_items: Vec<AnyElement> = Vec::new();

        // Mode pill on the left, always shown so the status bar stays
        // consistent across the two editor states. In the welcome state it
        // displays the outer mode itself ("Welcome") and is disabled; in the
        // editing state it displays the focused panel kind and opens the
        // panel-type dropdown.
        if let (Some(pane_id), Some(focused_kind)) = (focused_pane_id, focused_kind) {
            let mode = self.panel_mode();
            let editing = mode.is_editing();
            let toggle_editor = cx.entity().downgrade();
            let label = if editing {
                focused_kind.name().to_string()
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
                self.compute_source_cursor_position(cx),
                theme,
            ));
        }

        if self.has_tabs() && prefs.show_word_count {
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

        // Split / close buttons on the far right of the status bar. Available
        // even in the welcome state so the panels can be split before any
        // document is opened.
        if let (Some(pane_id), Some(_)) = (focused_pane_id, focused_kind) {
            let editor = cx.entity().downgrade();
            let btn_icon_size = toolbar_icon_size(d.bottombar_height);

            // Split H button.
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

            // Split V button.
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

            // Close button (only when multiple panels).
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
            for entry in self.doc().blocks() {
                if entry.entity.entity_id() == entity_id {
                    found = true;
                    break;
                }
                before_lines += entry.entity.read(cx).display_text().matches('\n').count() + 1;
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
        let text = self.doc().serialize_source_text(cx);
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
