//! WysiwygDocumentController — sync_commit handlers.

use std::sync::Arc;

use editor_contracts::{CursorHint, EditTransaction};
use gpui::{App, Context};

use super::WysiwygDocumentController;
impl WysiwygDocumentController {
    pub fn sync_document(
        &mut self,
        document: &editor_contracts::DocumentSnapshot,
        cx: &mut Context<Self>,
    ) {
        let next_base_dir = document
            .base_dir
            .as_deref()
            .map(std::path::Path::to_path_buf);
        let base_dir_changed = self.references.base_dir != next_base_dir;
        self.references.base_dir = next_base_dir;

        if self.synced_revision == Some(document.revision) && self.document.is_some() {
            if base_dir_changed {
                self.sync_reference_context(cx);
            }
            return;
        }
        if self.pending_edit {
            self.synced_revision = Some(document.revision);
            self.pending_edit = false;
            if base_dir_changed {
                self.sync_reference_context(cx);
            }
            return;
        }
        // The buffer's shared block projection always matches the
        // revision (it is maintained synchronously with each edit), so the
        // pane patches its entities against it without ever parsing.
        self.patch_from_blocks(
            document.blocks.as_slice(),
            document.revision,
            document.text.len(),
            cx,
        );
        if let Some(hint) = document.restore_cursor {
            self.restore_cursor_hint(hint, cx);
        }
    }

    pub fn document_text(&self, cx: &App) -> Option<String> {
        self.document.as_ref().map(|d| d.serialize_markdown(cx))
    }

    /// Current caret position of the active block as a document-level
    /// cursor hint (1-based line/column in the serialized Markdown).
    pub fn cursor_hint(&self, cx: &App) -> CursorHint {
        let Some(doc) = &self.document else {
            return CursorHint::new(1, 1);
        };
        let (lines, mappings) = doc.serialize_markdown_lines_with_mapping(cx);
        self.cursor_hint_from(&lines, &mappings, cx)
    }

    /// Cursor hint from an already-serialized document, so commits pay for
    /// one serialization instead of two.
    fn cursor_hint_from(
        &self,
        lines: &[String],
        mappings: &[crate::model::serialize::BlockLineMapping],
        cx: &App,
    ) -> CursorHint {
        let Some(active) = &self.active_entity else {
            return CursorHint::new(1, 1);
        };
        if lines.is_empty() || mappings.is_empty() {
            return CursorHint::new(1, 1);
        }
        let Some(mapping) = mappings.iter().find(|m| m.entity_id == active.entity_id()) else {
            return CursorHint::new(1, 1);
        };
        if mapping.own_start_line >= mapping.own_end_line || mapping.own_start_line >= lines.len() {
            return CursorHint::new((mapping.own_start_line + 1).min(lines.len()) as u32, 1);
        }

        let block = active.read(cx);
        let caret = block.cursor_offset();
        let intra = block.display_range_to_source_range(caret..caret);
        let markdown_text = block.data.text.serialize_markdown();
        let intra_offset = markdown_parser::inline::serialize::clamp_to_char_boundary(
            &markdown_text,
            intra.start.min(markdown_text.len()),
        );
        let before = &markdown_text[..intra_offset];
        let line_in_block = before.matches('\n').count();
        let col_in_block = before.rsplit('\n').next().unwrap_or("").chars().count();

        let num_own_lines = mapping.own_end_line - mapping.own_start_line;
        let line_offset_in_own = if block.kind().is_code_block() {
            (line_in_block + 1).min(num_own_lines.saturating_sub(1))
        } else {
            line_in_block.min(num_own_lines.saturating_sub(1))
        };
        let doc_line = mapping.own_start_line + line_offset_in_own;
        if doc_line >= lines.len() {
            return CursorHint::new(lines.len() as u32, 1);
        }

        let line_str = &lines[doc_line];
        let text_line = markdown_text.split('\n').nth(line_in_block).unwrap_or("");
        let prefix_bytes = if line_str.ends_with(text_line) {
            line_str.len() - text_line.len()
        } else {
            markdown_parser::inline::serialize::clamp_to_char_boundary(
                line_str,
                line_str.len().saturating_sub(text_line.len()),
            )
        };
        let prefix_chars = line_str[..prefix_bytes].chars().count();
        let col_chars = prefix_chars + col_in_block;

        CursorHint::new((doc_line + 1) as u32, (col_chars + 1) as u32)
    }

    /// Moves the active caret to the document position described by a
    /// cursor hint (used to apply `restore_cursor` after undo/redo).
    pub fn restore_cursor_hint(&mut self, hint: CursorHint, cx: &mut Context<Self>) {
        let Some(doc) = &self.document else {
            return;
        };
        let (lines, mappings) = doc.serialize_markdown_lines_with_mapping(cx);
        if mappings.is_empty() || lines.is_empty() {
            return;
        }

        let target_line = hint.line.saturating_sub(1) as usize;
        let target_col = hint.column.saturating_sub(1) as usize;

        let mut best = &mappings[0];
        for m in &mappings {
            if target_line >= m.own_start_line {
                best = m;
            }
            if target_line >= m.own_start_line && target_line < m.own_end_line {
                best = m;
                break;
            }
        }

        let Some(target_entity) = doc.block_entity_by_id(best.entity_id) else {
            return;
        };

        let block_ref = target_entity.read(cx);
        let markdown_text = block_ref.data.text.serialize_markdown();
        let num_own_lines = best.own_end_line.saturating_sub(best.own_start_line);
        let own_line_offset = target_line
            .saturating_sub(best.own_start_line)
            .min(num_own_lines.saturating_sub(1));
        let doc_line = (best.own_start_line + own_line_offset).min(lines.len().saturating_sub(1));
        let line_str = &lines[doc_line];

        let text_line_idx = if block_ref.kind().is_code_block() {
            own_line_offset.saturating_sub(1)
        } else {
            own_line_offset
        };

        let text_lines: Vec<&str> = markdown_text.split('\n').collect();
        let text_line = text_lines.get(text_line_idx).copied().unwrap_or("");
        let prefix_bytes = if line_str.ends_with(text_line) {
            line_str.len() - text_line.len()
        } else {
            markdown_parser::inline::serialize::clamp_to_char_boundary(
                line_str,
                line_str.len().saturating_sub(text_line.len()),
            )
        };
        let prefix_chars = line_str[..prefix_bytes].chars().count();
        let col_in_text_line = target_col
            .saturating_sub(prefix_chars)
            .min(text_line.chars().count());

        let mut source_offset = 0usize;
        for &prev in &text_lines[..text_line_idx.min(text_lines.len())] {
            source_offset += prev.len() + 1;
        }
        for ch in text_line.chars().take(col_in_text_line) {
            source_offset += ch.len_utf8();
        }
        let source_offset = markdown_parser::inline::serialize::clamp_to_char_boundary(
            &markdown_text,
            source_offset.min(markdown_text.len()),
        );

        self.active_entity = Some(target_entity.clone());
        target_entity.update(cx, |b, cx| {
            let display = b.source_range_to_display_range(source_offset..source_offset);
            let caret = display.start.min(b.display_len());
            b.selected_range = caret..caret;
            b.selection_reversed = false;
            b.marked_range = None;
            b.start_cursor_blink(cx);
            cx.notify();
        });
        cx.notify();
    }

    /// Commits the current document as one edit transaction.
    ///
    /// `merge` marks continuation of the previous undo transaction (typing
    /// runs). The caret hints anchor the buffer-level undo/redo restore.
    pub fn commit_document_edit(&mut self, merge: bool, cx: &mut App) {
        if let Some(edit) = self.take_edit_transaction(merge, cx) {
            if let Some(host) = self.host.clone() {
                host.commit_edit(edit, cx);
            }
        }
    }

    /// Commits a block-level text change, deriving the typing-run merge
    /// flag from the serialized text. One serialization serves the merge
    /// check, the caret hint, and the transaction itself.
    pub fn commit_typing_edit(&mut self, cx: &mut App) {
        let Some((lines, mappings)) = self
            .document
            .as_ref()
            .map(|doc| doc.serialize_markdown_lines_with_mapping(cx))
        else {
            return;
        };
        let text = lines.join("\n");
        let merge = self.is_typing_continuation(&text, cx);
        if let Some(edit) = self.build_transaction(merge, text, &lines, &mappings, cx) {
            if let Some(host) = self.host.clone() {
                host.commit_edit(edit, cx);
            }
        }
    }

    /// Builds the edit transaction for the current document state and
    /// updates the typing-run bookkeeping. Used both by direct commits and
    /// by contract methods that hand the transaction to the editor.
    pub fn take_edit_transaction(&mut self, merge: bool, cx: &App) -> Option<EditTransaction> {
        let (lines, mappings) = self
            .document
            .as_ref()?
            .serialize_markdown_lines_with_mapping(cx);
        let text = lines.join("\n");
        self.build_transaction(merge, text, &lines, &mappings, cx)
    }

    /// Assembles the transaction from one already-performed serialization:
    /// the caret hint reuses `lines`/`mappings` instead of re-serializing.
    fn build_transaction(
        &mut self,
        merge: bool,
        text: String,
        lines: &[String],
        mappings: &[crate::model::serialize::BlockLineMapping],
        cx: &App,
    ) -> Option<EditTransaction> {
        let cursor_after = self.cursor_hint_from(lines, mappings, cx);
        let cursor_before = if merge {
            self.typing_run_start_hint.unwrap_or(cursor_after)
        } else {
            self.last_cursor_hint.unwrap_or(cursor_after)
        };

        if merge {
            if self.typing_run_start_hint.is_none() {
                self.typing_run_start_hint = self.last_cursor_hint;
            }
            self.last_typing_insert_at = self.single_char_insert_at(&text);
        } else {
            self.typing_run_start_hint = self.last_cursor_hint;
            self.last_typing_insert_at = None;
        }

        self.last_cursor_hint = Some(cursor_after);
        self.last_committed_text = Some(text.clone());
        // The wysiwyg reserializes the whole document; report the edit as
        // one full-text replacement (the buffer applies it as a degenerate
        // whole-range edit).
        let old_len = self
            .last_committed_text
            .as_ref()
            .map(|text| text.len())
            .unwrap_or(self.last_synced_len);
        Some(EditTransaction::new(
            0..old_len,
            Arc::from(text),
            merge,
            cursor_before,
            cursor_after,
        ))
    }

    /// Whether committing `new_text` continues the previous typing run: a
    /// single-character insertion at exactly the previous insertion point,
    /// or an update while an IME composition is active.
    fn is_typing_continuation(&self, new_text: &str, cx: &App) -> bool {
        if let Some(active) = &self.active_entity {
            if active.read(cx).marked_range.is_some() {
                return true;
            }
        }
        let Some(insert_at) = self.last_typing_insert_at else {
            return false;
        };
        self.single_char_insert_at(new_text) == Some(insert_at)
    }

    /// Insert position when `new_text` is a single-character insertion into
    /// the previously committed text.
    fn single_char_insert_at(&self, new_text: &str) -> Option<usize> {
        let old_text = self.last_committed_text.as_ref()?;
        if new_text.len() != old_text.len() + 1 {
            return None;
        }
        let old = old_text.as_bytes();
        let new = new_text.as_bytes();
        let mut prefix = 0;
        while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
            prefix += 1;
        }
        if new_text.is_char_boundary(prefix)
            && prefix < new.len()
            && old[prefix..] == new[prefix + 1..]
        {
            Some(prefix)
        } else {
            None
        }
    }
}
