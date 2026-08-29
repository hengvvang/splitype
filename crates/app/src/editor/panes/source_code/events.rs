//! Event handling for the raw Markdown source code editor pane.

use gpui::*;

use crate::editor::engine::controller::{Editor, PaneId};
use theme::{ThemeManager, TypographyScope, TypographyStore};

impl Editor {
    /// Dispatches key-down events for a Source Code pane.
    pub(crate) fn handle_source_key_down(
        &mut self,
        pane_id: PaneId,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        let shift = event.keystroke.modifiers.shift;
        let alt = event.keystroke.modifiers.alt;

        if ctrl && !alt {
            match key {
                "a" | "A" => {
                    if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                        source.select_all();
                        cx.notify();
                        return true;
                    }
                }
                "c" | "C" => {
                    if let Some(source) = self.pane_state_ref(pane_id).and_then(|p| p.as_source_code()) {
                        if let Some(selected) = source.selected_text() {
                            cx.write_to_clipboard(ClipboardItem::new_string(selected.to_string()));
                            return true;
                        }
                    }
                }
                "x" | "X" => {
                    let mut text_to_copy = None;
                    if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                        if let Some(selected) = source.selected_text() {
                            text_to_copy = Some(selected.to_string());
                            source.delete_backward();
                        }
                    }
                    if let Some(text) = text_to_copy {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        self.sync_source_edit_to_document(pane_id, cx);
                        return true;
                    }
                }
                "v" | "V" => {
                    if let Some(clipboard) = cx.read_from_clipboard() {
                        if let Some(text) = clipboard.text() {
                            if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                                source.insert_text(&text);
                            }
                            self.sync_source_edit_to_document(pane_id, cx);
                            return true;
                        }
                    }
                }
                "z" | "Z" => {
                    if shift {
                        self.on_redo(&editor_wysiwyg::actions::Redo, _window, cx);
                    } else {
                        self.on_undo(&editor_wysiwyg::actions::Undo, _window, cx);
                    }
                    return true;
                }
                "y" | "Y" => {
                    self.on_redo(&editor_wysiwyg::actions::Redo, _window, cx);
                    return true;
                }
                "home" => {
                    if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                        source.move_to(0, shift);
                        cx.notify();
                    }
                    return true;
                }
                "end" => {
                    if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                        let len = source.text.len();
                        source.move_to(len, shift);
                        cx.notify();
                    }
                    return true;
                }
                _ => {}
            }
        }

        match key {
            "backspace" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.delete_backward();
                }
                self.sync_source_edit_to_document(pane_id, cx);
                true
            }
            "delete" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.delete_forward();
                }
                self.sync_source_edit_to_document(pane_id, cx);
                true
            }
            "enter" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.insert_text("\n");
                }
                self.sync_source_edit_to_document(pane_id, cx);
                true
            }
            "tab" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.insert_text("    ");
                }
                self.sync_source_edit_to_document(pane_id, cx);
                true
            }
            "space" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.insert_text(" ");
                }
                self.sync_source_edit_to_document(pane_id, cx);
                true
            }
            "left" | "arrowleft" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.move_left(shift);
                    cx.notify();
                }
                true
            }
            "right" | "arrowright" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.move_right(shift);
                    cx.notify();
                }
                true
            }
            "up" | "arrowup" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.move_up(shift);
                    cx.notify();
                }
                true
            }
            "down" | "arrowdown" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.move_down(shift);
                    cx.notify();
                }
                true
            }
            "home" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.move_to_line_start(shift);
                    cx.notify();
                }
                true
            }
            "end" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.move_to_line_end(shift);
                    cx.notify();
                }
                true
            }
            "pageup" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    for _ in 0..10 {
                        source.move_up(shift);
                    }
                    cx.notify();
                }
                true
            }
            "pagedown" => {
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    for _ in 0..10 {
                        source.move_down(shift);
                    }
                    cx.notify();
                }
                true
            }
            _ => {
                if !ctrl && !alt && !key.is_empty() {
                    let mut chars = key.chars();
                    if let Some(first) = chars.next() {
                        if chars.next().is_none() && !first.is_control() {
                            if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                                source.insert_text(key);
                            }
                            self.sync_source_edit_to_document(pane_id, cx);
                            return true;
                        }
                    } else if !key.starts_with("arrow") && !key.starts_with("f") {
                        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                            source.insert_text(key);
                        }
                        self.sync_source_edit_to_document(pane_id, cx);
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Handles mouse down events on the Source Code pane.
    pub(crate) fn handle_source_mouse_down(
        &mut self,
        pane_id: PaneId,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shift = event.modifiers.shift;
        let click_count = event.click_count;

        let Some(pane) = self.pane_state_ref(pane_id) else {
            return;
        };
        let Some(source) = pane.as_source_code() else {
            return;
        };

        let last_bounds = source.last_bounds;
        let total_lines = source.line_count();
        let theme = cx.global::<ThemeManager>().current_arc();
        let font_size = theme.typography.code_size.max(12.0);
        let line_height = (font_size * theme.typography.text_line_height).round();
        let padding = theme.dimensions.editor_padding;
        let font = TypographyStore::default_font(TypographyScope::Code);

        let line_digits = total_lines.to_string().len();
        let gutter_width = (line_digits as f32 * (font_size * 0.6) + 24.0).max(36.0);

        let bounds_origin = last_bounds.map(|b| b.origin).unwrap_or(point(px(0.0), px(0.0)));
        let rel_y = f32::from(event.position.y - bounds_origin.y) - padding;
        let line_idx = (rel_y / line_height).floor().max(0.0) as usize;
        let line_idx = line_idx.min(total_lines.saturating_sub(1));

        let line_str = source.line_str(line_idx);
        let rel_x = f32::from(event.position.x - bounds_origin.x) - gutter_width - 12.0;

        let col = if rel_x <= 0.0 || line_str.is_empty() {
            0
        } else {
            let shaped = window.text_system().shape_line(
                SharedString::new(line_str),
                px(font_size),
                &[TextRun {
                    len: line_str.len(),
                    font,
                    color: theme.colors.text_default,
                    ..Default::default()
                }],
                None,
            );
            shaped.index_for_x(px(rel_x)).unwrap_or(line_str.len())
        };

        let offset = source.offset_at_line_col(line_idx, col);

        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            if click_count >= 3 {
                source.select_line_at(line_idx);
            } else if click_count == 2 {
                source.select_word_at(offset);
            } else if shift {
                source.move_to(offset, true);
            } else {
                source.start_drag(offset);
            }
        }
        cx.notify();
    }

    /// Handles mouse move events during dragging on the Source Code pane.
    pub(crate) fn handle_source_mouse_move(
        &mut self,
        pane_id: PaneId,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let is_dragging = self
            .pane_state_ref(pane_id)
            .and_then(|p| p.as_source_code())
            .is_some_and(|s| s.is_dragging);
        if !is_dragging {
            return;
        }

        let Some(pane) = self.pane_state_ref(pane_id) else {
            return;
        };
        let Some(source) = pane.as_source_code() else {
            return;
        };

        let last_bounds = source.last_bounds;
        let total_lines = source.line_count();
        let theme = cx.global::<ThemeManager>().current_arc();
        let font_size = theme.typography.code_size.max(12.0);
        let line_height = (font_size * theme.typography.text_line_height).round();
        let padding = theme.dimensions.editor_padding;
        let font = TypographyStore::default_font(TypographyScope::Code);

        let line_digits = total_lines.to_string().len();
        let gutter_width = (line_digits as f32 * (font_size * 0.6) + 24.0).max(36.0);

        let bounds_origin = last_bounds.map(|b| b.origin).unwrap_or(point(px(0.0), px(0.0)));
        let rel_y = f32::from(event.position.y - bounds_origin.y) - padding;
        let line_idx = (rel_y / line_height).floor().max(0.0) as usize;
        let line_idx = line_idx.min(total_lines.saturating_sub(1));

        let line_str = source.line_str(line_idx);
        let rel_x = f32::from(event.position.x - bounds_origin.x) - gutter_width - 12.0;

        let col = if rel_x <= 0.0 || line_str.is_empty() {
            0
        } else {
            let shaped = window.text_system().shape_line(
                SharedString::new(line_str),
                px(font_size),
                &[TextRun {
                    len: line_str.len(),
                    font,
                    color: theme.colors.text_default,
                    ..Default::default()
                }],
                None,
            );
            shaped.index_for_x(px(rel_x)).unwrap_or(line_str.len())
        };

        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            let offset = source.offset_at_line_col(line_idx, col);
            source.update_drag(offset);
        }
        cx.notify();
    }

    /// Handles mouse up events on the Source Code pane.
    pub(crate) fn handle_source_mouse_up(
        &mut self,
        pane_id: PaneId,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            source.end_drag();
        }
        cx.notify();
    }

    /// Sync changes made in the Source pane buffer back to the shared document AST.
    pub(crate) fn sync_source_edit_to_document(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        let Some(text) = self
            .pane_state_ref(pane_id)
            .and_then(|s| s.as_source_code())
            .map(|s| s.text.clone())
        else {
            return;
        };

        self.rebuild_document_from_markdown(&text, cx);
        self.mark_dirty(cx);

        let synced_hash = self
            .active_doc()
            .map(|d| Self::hash_str(&d.serialize_markdown(cx)))
            .unwrap_or_default();
        let revision = self.active_tab().map(|t| t.document_revision).unwrap_or(0);
        let tab_index = self.session.active_tab_index();

        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            source.synced_doc_hash = synced_hash;
            source.synced_revision = Some(revision);
            source.synced_tab_index = Some(tab_index);
        }
        cx.notify();
    }
}
