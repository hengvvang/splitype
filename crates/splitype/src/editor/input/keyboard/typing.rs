//! Typing behavior: newline handling, table extension, and shortcut
//! transformations applied as text is entered.

use gpui::*;

use crate::editor::engine::controller::*;
use markdown::block::table::*;
use markdown::inline::text::BlockText;
use markdown::parse::BlockKind;

impl Editor {
    pub(crate) fn jump_to_footnote_definition(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(binding) = self.tab().references.footnotes.binding(id) else {
            return false;
        };
        let Some(block) = self.focusable_entity_by_id(binding.definition_entity_id) else {
            return false;
        };
        self.focus_block_range(&block, 0..0, cx);
        true
    }

    pub(crate) fn jump_to_footnote_backref(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(first_reference) = self
            .tab()
            .references
            .footnotes
            .binding(id)
            .and_then(|binding| binding.first_reference.clone())
        else {
            return false;
        };
        let Some(block) = self.focusable_entity_by_id(first_reference.entity_id) else {
            return false;
        };
        self.focus_block(block.entity_id());
        block.update(cx, |block, cx| {
            let plain_selected = block
                .data
                .text
                .fragments
                .iter()
                .take_while(|f| {
                    !f.footnote()
                        .is_some_and(|fn_ref| fn_ref.occurrence_index == first_reference.occurrence_index)
                })
                .map(|f| f.text.len())
                .sum::<usize>();
            let footnote_len = block
                .data
                .text
                .fragments
                .iter()
                .find(|f| {
                    f.footnote()
                        .is_some_and(|fn_ref| fn_ref.occurrence_index == first_reference.occurrence_index)
                })
                .map(|f| f.text.len())
                .unwrap_or(0);
            let plain_range = plain_selected..plain_selected + footnote_len;

            block.selected_range = plain_range.clone();
            block.sync_inline_projection_for_focus(true);
            let range = block
                .display_range_for_footnote_occurrence(first_reference.occurrence_index)
                .unwrap_or(0..0);
            block.selected_range = range;
            block.selection_reversed = false;
            block.marked_range = None;
            block.vertical_motion_x = None;
            block.cursor_blink_epoch = std::time::Instant::now();
            let supports_projection = block.edit_mode.supports_inline_projection();
            let kind_key = match block.kind() {
                markdown::parse::BlockKind::Heading { level } => Some(level),
                markdown::parse::BlockKind::Callout(variant) => Some(10 + variant as u8),
                _ => None,
            };
            block.projection_cache_key = Some((supports_projection, kind_key, plain_range, None));
            cx.notify();
        });
        true
    }

    pub(crate) fn insert_list_group_separator_before(
        &mut self,
        entity_id: EntityId,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(location) = self.doc().find_block_location(entity_id) else {
            return false;
        };

        let separator = Self::new_block(cx, BlockData::paragraph(String::new()));
        self.doc_mut()
            .insert_blocks_at(location.parent, location.index, vec![separator], cx);
        true
    }

    pub(crate) fn set_block_text_and_kind(
        block: &Entity<editor_wysiwyg::document::block::Block>,
        kind: BlockKind,
        text: BlockText,
        cursor: usize,
        cx: &mut Context<Self>,
    ) {
        let (kind, text, cursor) = Self::apply_paragraph_shortcuts(kind, text, cursor);
        block.update(cx, move |block, cx| {
            block.data.kind = kind;
            block.data.set_text(text.clone());
            block.sync_edit_mode_from_kind();
            block.sync_render_cache();
            let plain_cursor = cursor.min(block.data.text.plain_len());
            block.selected_range = block.plain_to_display_range(plain_cursor..plain_cursor);
            block.selection_reversed = false;
            block.marked_range = None;
            block.vertical_motion_x = None;
            block.cursor_blink_epoch = Instant::now();
            cx.notify();
        });
    }

    /// A block that a setext underline below it can promote into a heading: a
    /// non-empty, single-line, plain paragraph with no children.
    pub(crate) fn is_setext_heading_target(
        block: &Entity<editor_wysiwyg::document::block::Block>,
        cx: &App,
    ) -> bool {
        let block = block.read(cx);
        if block.kind() != BlockKind::Paragraph || !block.children.is_empty() {
            return false;
        }
        let text = block.data.text.plain_text();
        !text.trim().is_empty() && !text.contains('\n')
    }

    /// Handles Enter pressed on a paragraph that is a pure setext underline.
    /// When a matching paragraph precedes it at the root, the two collapse into
    /// a heading; a lone dash run still falls back to a thematic break. Returns
    /// true when it consumed the newline.
    pub(crate) fn try_form_setext_heading_on_newline(
        &mut self,
        block: &Entity<editor_wysiwyg::document::block::Block>,
        cx: &mut Context<Self>,
    ) -> bool {
        let text = block.read(cx).display_text().to_string();
        let Some(level) = BlockKind::parse_setext_underline(&text) else {
            return false;
        };
        if block.read(cx).kind() != BlockKind::Paragraph {
            return false;
        }
        let Some(location) = self.doc().find_block_location(block.entity_id()) else {
            return false;
        };

        // Only root paragraphs auto-form headings; nested contexts (quotes,
        // lists) keep their existing newline behavior.
        let target = if location.parent.is_none() {
            self.doc()
                .previous_sibling(block.entity_id(), cx)
                .filter(|prev| Self::is_setext_heading_target(prev, cx))
        } else {
            None
        };

        // A `=` underline with no heading target is ordinary text: defer to the
        // normal newline split. A dash run still has to become a separator.
        if target.is_none() && !BlockKind::parse_thematic_break_line(&text) {
            return false;
        }

        // The newline's own capture was already finalized by the block's Changed
        // event (nothing had changed yet), so start a fresh one here that spans
        // the heading/separator conversion. prepare is a no-op if one is pending.
        self.prepare_undo_capture(
            editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        );

        if let Some(prev) = target {
            let heading_text = prev.read(cx).data.text.clone();
            let cursor = heading_text.plain_len();
            let removed_id = block.entity_id();
            let new_paragraph = Self::new_block(cx, BlockData::paragraph(String::new()));

            Self::set_block_text_and_kind(
                &prev,
                BlockKind::Heading { level },
                heading_text,
                cursor,
                cx,
            );
            self.doc_mut().with_structure_mutation(cx, |document, cx| {
                let _ = document.remove_block_unindexed(removed_id, cx);
            });
            if let Some(heading_location) = self.doc().find_block_location(prev.entity_id()) {
                self.doc_mut().insert_blocks_at(
                    heading_location.parent,
                    heading_location.index + 1,
                    vec![new_paragraph.clone()],
                    cx,
                );
            }
            self.focus_block(new_paragraph.entity_id());
        } else {
            block.update(cx, |block, _cx| block.make_separator());
            let new_paragraph = Self::new_block(cx, BlockData::paragraph(String::new()));
            self.doc_mut().insert_blocks_at(
                location.parent,
                location.index + 1,
                vec![new_paragraph.clone()],
                cx,
            );
            self.focus_block(new_paragraph.entity_id());
        }

        self.rebuild_reference_registries(cx);
        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        cx.notify();
        true
    }

    /// Handles Enter pressed on a paragraph that is a pipe-table row. A
    /// delimiter row under a header paragraph forms a native table; a body row
    /// directly under an existing table is absorbed into it. After either, the
    /// caret lands in a fresh paragraph below the table so consecutive rows can
    /// be typed. Returns true when it consumed the newline.
    pub(crate) fn try_form_or_extend_table_on_newline(
        &mut self,
        block: &Entity<editor_wysiwyg::document::block::Block>,
        cx: &mut Context<Self>,
    ) -> bool {
        let text = block.read(cx).display_text().to_string();
        if block.read(cx).kind() != BlockKind::Paragraph || !is_table_row_candidate(&text) {
            return false;
        }
        let Some(location) = self.doc().find_block_location(block.entity_id()) else {
            return false;
        };
        if location.parent.is_some() {
            return false;
        }
        let Some(prev) = self.doc().previous_sibling(block.entity_id(), cx) else {
            return false;
        };

        if prev.read(cx).kind() == BlockKind::Table {
            // A multi-column row typed directly under a table is meant as a row,
            // so absorb it and let the table normalize ragged cell counts the
            // same way pasted rows are padded or truncated to the header width.
            return self.extend_table_with_typed_row(&prev, block, &text, cx);
        }

        if prev.read(cx).kind() != BlockKind::Paragraph {
            return false;
        }
        let header_text = prev.read(cx).display_text().to_string();
        if !is_table_row_candidate(&header_text) {
            return false;
        }
        let Some(table) = parse_table_region(&[header_text, text]) else {
            return false;
        };

        self.prepare_undo_capture(
            editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        );
        // Remove the lower (delimiter) block first so the header index is stable.
        let header_index = location.index - 1;
        let removed_delimiter = block.entity_id();
        let removed_header = prev.entity_id();
        let table_block = Self::new_table_block(cx, table);
        let new_paragraph = Self::new_block(cx, BlockData::paragraph(String::new()));
        self.doc_mut().with_structure_mutation(cx, |document, cx| {
            let _ = document.remove_block_unindexed(removed_delimiter, cx);
            let _ = document.remove_block_unindexed(removed_header, cx);
        });
        self.doc_mut().insert_blocks_at(
            None,
            header_index,
            vec![table_block.clone(), new_paragraph.clone()],
            cx,
        );
        self.rebuild_table_grids(cx);
        self.focus_block(new_paragraph.entity_id());
        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        cx.notify();
        true
    }

    pub(crate) fn extend_table_with_typed_row(
        &mut self,
        table_block: &Entity<editor_wysiwyg::document::block::Block>,
        row_block: &Entity<editor_wysiwyg::document::block::Block>,
        text: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        // Capture any in-progress cell edits before mutating the table data.
        self.sync_table_data_from_grid(table_block, cx);
        let Some(mut table) = table_block.read(cx).data.table.clone() else {
            return false;
        };
        let Some(row) = parse_table_body_row(text, table.column_count()) else {
            return false;
        };

        self.prepare_undo_capture(
            editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        );
        table.rows.push(row);
        table_block.update(cx, |block, cx| {
            block.data.table = Some(table);
            cx.notify();
        });

        let removed_id = row_block.entity_id();
        self.doc_mut().with_structure_mutation(cx, |document, cx| {
            let _ = document.remove_block_unindexed(removed_id, cx);
        });
        let new_paragraph = Self::new_block(cx, BlockData::paragraph(String::new()));
        if let Some(table_location) = self.doc().find_block_location(table_block.entity_id()) {
            self.doc_mut().insert_blocks_at(
                table_location.parent,
                table_location.index + 1,
                vec![new_paragraph.clone()],
                cx,
            );
        }
        self.rebuild_table_grids(cx);
        self.focus_block(new_paragraph.entity_id());
        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        cx.notify();
        true
    }

    /// Inserts an empty paragraph after `block` when it renders as a
    /// self-contained structure the caret cannot move past (table, code, math,
    /// separator, quote, callout, footnote definition, standalone image, ...)
    /// and nothing currently follows it in its container. This keeps a rendered
    /// document from ending on such a block, so a rendered-first user can keep
    /// typing past it rather than being stranded. No-op when something already
    /// follows the block or it is not a stranding structure.
    pub(crate) fn ensure_trailing_paragraph_after_structural(
        &mut self,
        block: &Entity<editor_wysiwyg::document::block::Block>,
        cx: &mut Context<Self>,
    ) {
        let strands = {
            let block = block.read(cx);
            let kind = block.kind();
            kind.is_atomic_structural()
                || kind.is_quote_container()
                || kind.is_footnote_definition()
                || block.is_standalone_image()
        };
        if !strands {
            return;
        }
        let Some(location) = self.doc().find_block_location(block.entity_id()) else {
            return;
        };
        let sibling_count = match location.parent.as_ref() {
            Some(parent) => parent.read(cx).children.len(),
            None => self.doc().root_count(),
        };
        if location.index + 1 < sibling_count {
            return;
        }
        let trailing = Self::new_block(cx, BlockData::paragraph(String::new()));
        self.doc_mut()
            .insert_blocks_at(location.parent, location.index + 1, vec![trailing], cx);
    }

    pub(crate) fn apply_paragraph_shortcuts(
        kind: BlockKind,
        mut text: BlockText,
        cursor: usize,
    ) -> (BlockKind, BlockText, usize) {
        if kind == BlockKind::Paragraph {
            let plain_text = text.plain_text();
            if let Some((detected_kind, prefix_len)) =
                BlockKind::detect_markdown_shortcut(&plain_text)
            {
                text.remove_plain_prefix(prefix_len);
                return (detected_kind, text, cursor.saturating_sub(prefix_len));
            }
        }

        (kind, text, cursor)
    }
}
