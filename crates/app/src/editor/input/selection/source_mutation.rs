//! Cross-block markdown serialization, text replacement, and source coordinate mapping.

use std::collections::HashMap;
use std::ops::Range;

use gpui::*;

use editor_wysiwyg::document::protocol::UndoCaptureKind;
use crate::editor::engine::controller::{
    CrossBlockSelection, CrossBlockSelectionEndpoint, Editor, EditorSelection,
    SourceTargetMapping, UndoSelectionSnapshot,
};
use crate::editor::input::selection::state::NormalizedCrossBlockSelection;
use editor_wysiwyg::document::block::Block;
use markdown::block::table::serialize_table_markdown_lines;
use markdown::parse::BlockKind;

impl Editor {
    pub(crate) fn cross_block_source_selection_snapshot(
        &self,
        cx: &App,
    ) -> Option<UndoSelectionSnapshot> {
        let normalized = self.normalized_cross_block_selection(cx)?;
        let range = self.cross_block_source_range_for_normalized(normalized, cx)?;
        Some(UndoSelectionSnapshot {
            range,
            reversed: normalized.reversed,
            block_anchor: None,
        })
    }

    pub(crate) fn apply_cross_block_selection_snapshot_if_possible(
        &mut self,
        snapshot: &UndoSelectionSnapshot,
        cx: &mut Context<Self>,
    ) -> bool {
        if snapshot.range.is_empty() {
            return false;
        }

        let mappings = self.build_source_target_mappings(cx);
        let Some(start) = self.endpoint_for_source_offset(snapshot.range.start, &mappings, cx)
        else {
            return false;
        };
        let Some(end) = self.endpoint_for_source_offset(snapshot.range.end, &mappings, cx) else {
            return false;
        };
        let Some(start_index) = self.doc().index_for_entity_id(start.entity_id) else {
            return false;
        };
        let Some(end_index) = self.doc().index_for_entity_id(end.entity_id) else {
            return false;
        };
        if start_index == end_index {
            return false;
        }

        if let Some(selection) = self.active_pane_state().selection_mut() {
            selection.cross_block = Some(if snapshot.reversed {
                CrossBlockSelection {
                    anchor: end,
                    focus: start,
                }
            } else {
                CrossBlockSelection {
                    anchor: start,
                    focus: end,
                }
            });
            selection.cross_block_drag = None;
        }
        self.sync_cross_block_selection_visuals(cx);
        let focus = if snapshot.reversed { start } else { end };
        self.focus_block(focus.entity_id);
        cx.notify();
        true
    }

    pub(crate) fn source_mapping_by_entity_id(
        &self,
        cx: &App,
    ) -> HashMap<EntityId, SourceTargetMapping> {
        self.build_source_target_mappings(cx)
            .into_iter()
            .map(|mapping| (mapping.entity.entity_id(), mapping))
            .collect()
    }

    pub(crate) fn endpoint_source_offset(
        &self,
        endpoint: CrossBlockSelectionEndpoint,
        mappings: &HashMap<EntityId, SourceTargetMapping>,
        cx: &App,
    ) -> Option<usize> {
        let mapping = mappings.get(&endpoint.entity_id)?;
        let block = mapping.entity.read(cx);
        let display_len = block.display_len();
        if endpoint.offset == 0 {
            return Some(mapping.full_source_range.start);
        }
        if endpoint.offset >= display_len {
            return Some(mapping.full_source_range.end);
        }
        let source_offset = block
            .display_range_to_source_range(endpoint.offset..endpoint.offset)
            .start;
        Some(mapping.full_source_range.start + mapping.content_to_source_offset(source_offset))
    }

    pub(crate) fn endpoint_for_source_offset(
        &self,
        offset: usize,
        mappings: &[SourceTargetMapping],
        cx: &App,
    ) -> Option<CrossBlockSelectionEndpoint> {
        let mapping = mappings.iter().min_by_key(|mapping| {
            Self::source_offset_distance(&mapping.full_source_range, offset)
        })?;
        let local = if offset <= mapping.full_source_range.start {
            0
        } else if offset >= mapping.full_source_range.end {
            mapping.full_source_range.len()
        } else {
            offset - mapping.full_source_range.start
        };
        let content_offset = mapping.source_to_content_offset(local);
        let block = mapping.entity.read(cx);
        Some(CrossBlockSelectionEndpoint {
            entity_id: mapping.entity.entity_id(),
            offset: block.source_offset_to_display_offset(content_offset),
        })
    }

    pub(crate) fn cross_block_source_range_for_normalized(
        &self,
        selection: NormalizedCrossBlockSelection,
        cx: &App,
    ) -> Option<Range<usize>> {
        let (mapping_list, block_ranges) = self.build_source_target_mappings_with_block_ranges(cx);
        let mappings: HashMap<EntityId, SourceTargetMapping> = mapping_list
            .into_iter()
            .map(|mapping| (mapping.entity.entity_id(), mapping))
            .collect();
        let entries = self.doc().blocks();

        // Resolve an endpoint to a source offset. Atomic blocks (tables, etc.)
        // carry no per-block text mapping, so endpoint_source_offset returns
        // None for them; fall back to the block's own source span, picking the
        // side that keeps the block inside the selection.
        let endpoint_offset =
            |endpoint: CrossBlockSelectionEndpoint, index: usize, at_end: bool| -> Option<usize> {
                if let Some(offset) = self.endpoint_source_offset(endpoint, &mappings, cx) {
                    return Some(offset);
                }
                let entity = entries.get(index)?.entity.clone();
                let range = block_ranges.get(&entity.entity_id())?;
                Some(if at_end { range.end } else { range.start })
            };

        let start = endpoint_offset(selection.start, selection.start_index, false)?;
        let end = endpoint_offset(selection.end, selection.end_index, true)?;
        let (mut lo, mut hi) = (start.min(end), start.max(end));

        // Endpoint offsets can never point *after* a zero-entries-len (atomic)
        // block, so a table at the trailing boundary of the selection would be
        // left behind. Union in the full source range of every atomic block
        // whose entries index falls inside the selection so it is removed whole.
        for index in selection.block_index_range() {
            let entity = entries.get(index)?.entity.clone();
            if entity.read(cx).display_len() == 0 {
                if let Some(range) = block_ranges.get(&entity.entity_id()) {
                    lo = lo.min(range.start);
                    hi = hi.max(range.end);
                }
            }
        }
        Some(lo..hi)
    }

    pub(crate) fn rebuild_after_cross_block_source_edit(
        &mut self,
        source: String,
        cx: &mut Context<Self>,
    ) {
        self.rebuild_document_from_markdown(&source, cx);
    }

    pub(crate) fn apply_marked_source_range(
        &mut self,
        source_range: Range<usize>,
        cx: &mut Context<Self>,
    ) {
        if source_range.is_empty() {
            return;
        }
        let mappings = self.build_source_target_mappings(cx);
        let Some(start) = self.endpoint_for_source_offset(source_range.start, &mappings, cx) else {
            return;
        };
        let Some(end) = self.endpoint_for_source_offset(source_range.end, &mappings, cx) else {
            return;
        };
        if start.entity_id != end.entity_id {
            return;
        }
        let Some(block) = self.focusable_entity_by_id(start.entity_id) else {
            return;
        };
        block.update(cx, |block, cx| {
            block.marked_range = Some(start.offset.min(end.offset)..start.offset.max(end.offset));
            cx.notify();
        });
    }

    pub(crate) fn replace_cross_block_selection_with_text(
        &mut self,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        undo_kind: UndoCaptureKind,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(selection) = self.normalized_cross_block_selection(cx) else {
            return false;
        };
        let Some(source_range) = self.cross_block_source_range_for_normalized(selection, cx) else {
            return false;
        };

        self.prepare_undo_capture(undo_kind, cx);
        let mut source = self.serialized_document_text(cx);
        let (start, end) = (
            source_range.start.min(source_range.end).min(source.len()),
            source_range.start.max(source_range.end).min(source.len()),
        );
        let start = if source.is_char_boundary(start) {
            start
        } else {
            source.floor_char_boundary(start)
        };
        let end = if source.is_char_boundary(end) {
            end
        } else {
            source.ceil_char_boundary(end)
        };
        let end = end.min(source.len());
        source.replace_range(start..end, new_text);
        if let Some(selection) = self.active_pane_state().selection_mut() {
            selection.cross_block = None;
            selection.cross_block_drag = None;
        }

        let inserted_start = start;
        let inserted_end = inserted_start + new_text.len();
        let selected_source_range = selected_range_relative
            .map(|relative| {
                inserted_start + relative.start.min(new_text.len())
                    ..inserted_start + relative.end.min(new_text.len())
            })
            .unwrap_or(inserted_end..inserted_end);
        let marked_source_range =
            (mark_inserted_text && !new_text.is_empty()).then_some(inserted_start..inserted_end);

        self.rebuild_after_cross_block_source_edit(source, cx);
        self.apply_selection_snapshot_in_current_mode(
            &UndoSelectionSnapshot {
                range: selected_source_range,
                reversed: false,
                block_anchor: None,
            },
            cx,
        );
        if let Some(marked_source_range) = marked_source_range {
            self.apply_marked_source_range(marked_source_range, cx);
        }
        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        self.sync_cross_block_selection_visuals(cx);
        self.request_autoscroll_active_pane(
            crate::editor::engine::controller::AutoscrollStrategy::Fit { margin: px(20.0) },
            cx,
        );
        cx.notify();
        true
    }

    pub(crate) fn cross_block_selected_markdown(&self, cx: &App) -> Option<String> {
        let selection = self.normalized_cross_block_selection(cx)?;
        let source = self.serialized_document_text(cx);
        let mappings = self.source_mapping_by_entity_id(cx);
        let entries = self.doc().blocks();

        // Join blocks with the same spacing the document serializer uses
        // (collect_root_markdown_lines): a blank line between blocks, but tight
        // list items stay on consecutive lines. A flat single-newline join used
        // to silently fuse separate paragraphs on paste, and once setext pairs
        // are recognized it could even fabricate a heading from two paragraphs.
        let mut result = String::new();
        let mut wrote_chunk = false;

        for index in selection.block_index_range() {
            let entity = entries.get(index)?.entity.clone();
            let block = entity.read(cx);
            let len = block.display_len();
            let range = if selection.is_single_block() {
                selection.start.offset.min(len)..selection.end.offset.min(len)
            } else if index == selection.start_index {
                selection.start.offset.min(len)..len
            } else if index == selection.end_index {
                0..selection.end.offset.min(len)
            } else {
                0..len
            };
            let full_block =
                range.start == 0 && range.end == len && (!selection.is_single_block() || len > 0);
            let include_atomic = len == 0 && !selection.is_single_block();
            if range.is_empty() && !include_atomic && !Editor::is_empty_root_paragraph(block) {
                continue;
            }

            if wrote_chunk {
                result.push('\n');
            }
            result.push_str(&self.markdown_chunk_for_block(
                &entity,
                range,
                full_block || include_atomic,
                &source,
                &mappings,
                cx,
            ));
            wrote_chunk = true;
        }

        Some(result)
    }

    pub(crate) fn safe_source_slice(source: &str, range: Range<usize>) -> &str {
        let (start, end) = (range.start.min(range.end), range.start.max(range.end));
        let start = start.min(source.len());
        let end = end.min(source.len());
        let start = if source.is_char_boundary(start) {
            start
        } else {
            source.floor_char_boundary(start)
        };
        let end = if source.is_char_boundary(end) {
            end
        } else {
            source.ceil_char_boundary(end)
        };
        let end = end.min(source.len());
        if start <= end {
            &source[start..end]
        } else {
            ""
        }
    }

    pub(crate) fn markdown_chunk_for_block(
        &self,
        entity: &Entity<Block>,
        range: Range<usize>,
        full_block: bool,
        source: &str,
        mappings: &HashMap<EntityId, SourceTargetMapping>,
        cx: &App,
    ) -> String {
        if let Some(mapping) = mappings.get(&entity.entity_id()) {
            if full_block {
                return Self::safe_source_slice(source, mapping.full_source_range.clone())
                    .to_string();
            }

            let start = self
                .endpoint_source_offset(
                    CrossBlockSelectionEndpoint {
                        entity_id: entity.entity_id(),
                        offset: range.start,
                    },
                    mappings,
                    cx,
                )
                .unwrap_or(mapping.full_source_range.start);
            let end = self
                .endpoint_source_offset(
                    CrossBlockSelectionEndpoint {
                        entity_id: entity.entity_id(),
                        offset: range.end,
                    },
                    mappings,
                    cx,
                )
                .unwrap_or(mapping.full_source_range.end);
            return Self::safe_source_slice(source, start.min(end)..start.max(end)).to_string();
        }

        let block = entity.read(cx);
        if full_block {
            return match block.kind() {
                BlockKind::Table => block
                    .data
                    .table
                    .as_ref()
                    .map(serialize_table_markdown_lines)
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_default(),
                _ => block
                    .data
                    .serialize_markdown_line(block.render_depth, block.list_ordinal),
            };
        }

        let markdown = block.data.text.serialize_markdown();
        let source_range = block.display_range_to_source_range(range);
        markdown
            .get(source_range)
            .map(ToOwned::to_owned)
            .unwrap_or_default()
    }

    pub(crate) fn delete_cross_block_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(selection) = self.normalized_cross_block_selection(cx) else {
            return false;
        };
        let Some(source_range) = self.cross_block_source_range_for_normalized(selection, cx) else {
            return false;
        };
        if source_range.is_empty() {
            return false;
        }

        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        let mut source = self.serialized_document_text(cx);
        let (start, end) = (
            source_range.start.min(source_range.end).min(source.len()),
            source_range.start.max(source_range.end).min(source.len()),
        );
        let start = if source.is_char_boundary(start) {
            start
        } else {
            source.floor_char_boundary(start)
        };
        let end = if source.is_char_boundary(end) {
            end
        } else {
            source.ceil_char_boundary(end)
        };
        let end = end.min(source.len());
        source.replace_range(start..end, "");
        if let Some(selection) = self.active_pane_state().selection_mut() {
            selection.cross_block = None;
            selection.cross_block_drag = None;
        }

        self.rebuild_after_cross_block_source_edit(source, cx);

        self.apply_selection_snapshot_in_current_mode(
            &UndoSelectionSnapshot {
                range: start..start,
                reversed: false,
                block_anchor: None,
            },
            cx,
        );
        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        self.sync_cross_block_selection_visuals(cx);
        cx.notify();
        true
    }

    /// Returns the markdown text of the current selection, whether cross-block,
    /// intra-block, or source mode. Returns `None` when nothing is selected.
    pub(crate) fn selected_markdown_text(&self, cx: &App) -> Option<String> {
        if self.is_source_code() {
            let pane_id = self.active_pane_id();
            return self
                .pane_state_ref(pane_id)
                .and_then(|p| p.as_source_code())
                .and_then(|p| p.selected_text().map(String::from));
        }

        if self.is_preview() {
            return self.preview_selected_text(cx);
        }

        match self.active_selection(cx) {
            EditorSelection::CrossBlock(_) => self.cross_block_selected_markdown(cx),
            EditorSelection::IntraBlock { block_id, range, .. } => {
                let block = self.doc().block_entity_by_id(block_id)?.read(cx);
                let source_range = block.display_range_to_source_range(range);
                let full_markdown = block.data.text.serialize_markdown();
                let start = source_range.start.min(full_markdown.len());
                let end = source_range.end.min(full_markdown.len());
                let start = if full_markdown.is_char_boundary(start) {
                    start
                } else {
                    full_markdown.floor_char_boundary(start)
                };
                let end = if full_markdown.is_char_boundary(end) {
                    end
                } else {
                    full_markdown.ceil_char_boundary(end)
                };
                if start < end {
                    Some(full_markdown[start..end].to_owned())
                } else {
                    None
                }
            }
            EditorSelection::TableAxis(_) | EditorSelection::None => None,
        }
    }

    /// Deletes the current active selection across WYSIWYG and Source Code modes.
    pub(crate) fn delete_active_selection(&mut self, cx: &mut Context<Self>) -> bool {
        if self.is_source_code() {
            let pane_id = self.active_pane_id();
            let mut deleted = false;
            if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                if source.selection.is_some() {
                    source.delete_backward();
                    deleted = true;
                }
            }
            if deleted {
                self.sync_source_edit_to_document(pane_id, cx);
            }
            return deleted;
        }

        match self.active_selection(cx) {
            EditorSelection::CrossBlock(_) => self.delete_cross_block_selection(cx),
            EditorSelection::IntraBlock { block_id, range, .. } => {
                let Some(block) = self.focusable_entity_by_id(block_id) else {
                    return false;
                };
                self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
                block.update(cx, |b, cx| {
                    b.replace_text_in_display_range(
                        range,
                        "",
                        Some(0..0),
                        false,
                        cx,
                    );
                });
                self.mark_dirty(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
                true
            }
            EditorSelection::TableAxis(_) | EditorSelection::None => false,
        }
    }
}
