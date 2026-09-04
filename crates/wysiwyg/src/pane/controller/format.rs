//! WysiwygDocumentController — format handlers.

use gpui::{ClipboardItem, Context, EntityId, Window};

use crate::model::block::state::InlineFormat;
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::BlockKind;

use super::WysiwygDocumentController;
impl WysiwygDocumentController {
    pub fn cut_active_selection(&mut self, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            if !b.selected_range.is_empty() {
                let text = b.selected_text();
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                b.apply_source_space_text_edit(b.selected_range.clone(), "", None, false, cx);
            } else {
                let text = b.data.text.plain_text();
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                b.data.text = BlockText::plain(String::new());
                b.mark_changed(cx);
            }
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn copy_active_selection(&self, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.read_with(cx, |b, cx| {
            let text = if !b.selected_range.is_empty() {
                b.selected_text()
            } else {
                b.data.text.plain_text()
            };
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        });
    }

    pub fn paste_into_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            b.on_paste(&platform_contracts::actions::Paste, window, cx);
        });
    }

    pub fn toggle_active_format(&mut self, format: InlineFormat, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            b.toggle_inline_format(format, cx);
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn wrap_active_selection(
        &mut self,
        left_delim: &str,
        right_delim: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            if !b.selected_range.is_empty() {
                let text = b.selected_text();
                let wrapped = format!("{left_delim}{text}{right_delim}");
                b.apply_source_space_text_edit(b.selected_range.clone(), &wrapped, None, false, cx);
            } else {
                let wrapped = format!("{left_delim}{right_delim}");
                let cur = b.selected_range.start;
                b.apply_source_space_text_edit(cur..cur, &wrapped, None, false, cx);
            }
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn clear_active_selection_format(&mut self, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            if !b.selected_range.is_empty() {
                let text = b.selected_text();
                let cleaned = text
                    .replace("**", "")
                    .replace("~~", "")
                    .replace("==", "")
                    .replace(['*', '`', '$'], "");
                b.apply_source_space_text_edit(b.selected_range.clone(), &cleaned, None, false, cx);
            }
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn convert_target_block(
        &mut self,
        target_id: EntityId,
        kind: BlockKind,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(target_id) else {
            return;
        };
        target.update(cx, |b, cx| {
            b.data.kind = kind;
            b.mark_changed(cx);
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }
}
