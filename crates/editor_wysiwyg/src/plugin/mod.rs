//! WYSIWYG Pane plugin implementation — PaneView contract and lifecycle.

pub mod actions;
pub mod outline;
pub mod search;
pub mod state;

pub use actions::*;
pub use outline::*;
pub use search::*;
pub use state::*;

use gpui::{
    AnyElement, App, AppContext, Div, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, StatefulInteractiveElement, Styled, Window, div,
};
use editor_model::{
    EditorDocument, PaneKindId, PaneOutlineHost, PaneRenderContext, PaneView,
};
use editor_outline::OutlineNode;
use editor_search::{SearchMatch, SearchQuery};
use theme::Theme;

use crate::document::block::Block;
use crate::document::Document;

/// View state specific to a WYSIWYG editor pane.
#[derive(Default)]
pub struct WysiwygPaneState {
    pub focus: FocusState,
    pub selection: SelectionState,
    pub document: Option<Document>,
    pub tables: TableGrids,
    pub references: ReferenceRegistries,
    pub text_stale: bool,
}

impl PaneView for WysiwygPaneState {
    fn kind(&self) -> PaneKindId {
        PaneKindId::WYSIWYG
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        if let Some(document) = &self.document {
            if self.text_stale {
                return document.serialize_markdown(cx);
            }
        }
        doc.serialize_markdown(cx)
    }

    fn sync_document_text(&mut self, text: &str, _revision: u64, cx: &mut App) {
        if self.document.is_none() {
            let parsed = crate::markdown::parse::parser::parse_wysiwyg_document(text);
            let entities = parsed
                .into_iter()
                .map(|b| cx.new(|cx| Block::with_data(cx, b)))
                .collect();
            let doc = Document::new(entities);
            self.document = Some(doc);
        }
    }

    fn serialize_text(&self, cx: &App) -> Option<String> {
        self.document.as_ref().map(|d| d.serialize_markdown(cx))
    }

    fn focus_handle(&self, cx: &App) -> Option<gpui::FocusHandle> {
        if let Some(entity_id) = self.focus.active_entity {
            if let Some(doc) = &self.document {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    return Some(block.read(cx).focus_handle.clone());
                }
            }
        }
        None
    }

    fn outline_headings(&self, cx: &App) -> Vec<OutlineNode> {
        if let Some(doc) = &self.document {
            crate::plugin::outline::extract_outline_headings(doc, cx)
        } else {
            Vec::new()
        }
    }

    fn navigate_to_outline(&mut self, index: usize, theme: &Theme, cx: &mut App) {
        let headings = self.outline_headings(cx);
        if let Some(node) = headings.get(index) {
            if let Some(entity_id) = node.block_id {
                self.focus.active_entity = Some(entity_id);
                if let Some(doc) = &self.document {
                    if let Some(block) = doc.block_entity_by_id(entity_id) {
                        block.update(cx, |block, cx| {
                            block.assign_collapsed_selection_offset(
                                0,
                                crate::document::block::CollapsedCaretAffinity::Default,
                                None,
                            );
                            cx.notify();
                        });
                    }
                }
            }
            let font_size = theme.typography.text_size.max(14.0);
            let line_height = (font_size * theme.typography.text_line_height).round().max(24.0);
            let _target_y = (node.block_index as f32 * line_height * 1.5) - 40.0;
        }
    }

    fn search_matches(&self, query: &SearchQuery, cx: &App) -> Vec<SearchMatch> {
        if let Some(doc) = &self.document {
            crate::plugin::search::search_in_document(doc, query, cx)
        } else {
            Vec::new()
        }
    }

    fn replace_match(
        &mut self,
        match_item: &SearchMatch,
        replace_with: &str,
        cx: &mut App,
    ) {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                crate::plugin::search::replace_in_block_entity(
                    doc,
                    entity_id,
                    match_item.byte_range.clone(),
                    replace_with,
                    cx,
                );
            }
        }
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, cx: &mut App) {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                self.focus.active_entity = Some(entity_id);
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    block.update(cx, |block, cx| {
                        block.selected_range = match_item.byte_range.clone();
                        block.selection_reversed = false;
                        block.start_cursor_blink(cx);
                        cx.notify();
                    });
                }
            }
        }
    }

    fn apply_line_prefix(&mut self, prefix: &str, cx: &mut App) {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = self.focus.active_entity {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    block.update(cx, |block, cx| {
                        if !block.edits_verbatim_text() {
                            block.data.kind = crate::markdown::parse::BlockKind::Paragraph;
                        }
                        let cursor = block.cursor_offset();
                        let text = block.display_text();
                        let line_start = text[..cursor.min(text.len())]
                            .rfind('\n')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let line_end = text[cursor.min(text.len())..]
                            .find('\n')
                            .map(|i| cursor + i)
                            .unwrap_or(text.len());
                        let line = &text[line_start..line_end];
                        let stripped = line.trim_start_matches(|c| {
                            c == '#' || c == '>' || c == '-' || c == '*' || c == '+' || c == ' ' || c == '\t'
                        });
                        let new_line = format!("{prefix}{stripped}");
                        let prefix_len = prefix.len();
                        block.replace_text_in_display_range(
                            line_start..line_end,
                            &new_line,
                            Some(prefix_len..prefix_len),
                            false,
                            cx,
                        );
                    });
                }
            }
        }
    }

    fn apply_snippet(&mut self, snippet: &str, caret_offset: usize, cx: &mut App) {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = self.focus.active_entity {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    block.update(cx, |block, cx| {
                        let cursor = block.cursor_offset();
                        let range = block.selected_range.clone();
                        let len = snippet.len();
                        let offset = caret_offset.min(len);
                        if range.is_empty() {
                            block.replace_text_in_display_range(
                                cursor..cursor,
                                snippet,
                                Some(offset..offset),
                                false,
                                cx,
                            );
                        } else {
                            block.replace_text_in_display_range(
                                range,
                                snippet,
                                Some(offset..offset),
                                false,
                                cx,
                            );
                        }
                    });
                }
            }
        }
    }

    fn apply_wrapped_or_template(
        &mut self,
        empty_template: &str,
        caret_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        cx: &mut App,
    ) {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = self.focus.active_entity {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    block.update(cx, |block, cx| {
                        let range = block.selected_range.clone();
                        if range.is_empty() {
                            let cursor = block.cursor_offset();
                            block.replace_text_in_display_range(
                                cursor..cursor,
                                empty_template,
                                Some(caret_offset_in_empty..caret_offset_in_empty),
                                false,
                                cx,
                            );
                        } else {
                            let text = block.selected_text();
                            let inner_len = text.len();
                            let replacement = format!("{wrap_prefix}{text}{wrap_suffix}");
                            let prefix_len = wrap_prefix.len();
                            block.replace_text_in_display_range(
                                range,
                                &replacement,
                                Some(prefix_len..prefix_len + inner_len),
                                false,
                                cx,
                            );
                        }
                    });
                }
            }
        }
    }

    fn apply_clear_format(&mut self, cx: &mut App) {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = self.focus.active_entity {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    block.update(cx, |b, cx| {
                        let range = b.selected_range.clone();
                        if !range.is_empty() {
                            let (target_range, plain) = {
                                let text = b.display_text();
                                let start = range.start.min(text.len());
                                let end = range.end.min(text.len());
                                let selected = &text[start..end];
                                let plain = selected.trim_matches(|c| {
                                    c == '*' || c == '_' || c == '~' || c == '`' || c == '=' || c == '$'
                                }).to_string();
                                (range, plain)
                            };
                            let plain_len = plain.len();
                            b.replace_text_in_display_range(
                                target_range,
                                &plain,
                                Some(0..plain_len),
                                false,
                                cx,
                            );
                        }
                    });
                }
            }
        }
    }

    fn render(
        &mut self,
        ctx: &PaneRenderContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let d = &theme.dimensions;
        let c = &theme.colors;
        let pane_id = ctx.pane_id;

        if let Some(doc) = &self.document {
            let blocks = doc.blocks();
            let plans = crate::render::viewport::plan_document_rows(&blocks, d, cx);
            let centered_width = crate::render::layout::centered_column_width(
                f32::from(ctx.scroll.bounds().size.width).max(600.0),
                d,
            );

            let row_elements: Vec<AnyElement> = plans
                .iter()
                .map(|plan| {
                    crate::render::viewport::build_planned_row_element(
                        plan,
                        &blocks,
                        centered_width,
                        &theme,
                        d,
                        |row: Div, _id| row,
                    )
                })
                .collect();

            let outline_host: std::sync::Arc<dyn editor_outline::OutlineHost> =
                std::sync::Arc::new(PaneOutlineHost {
                    pane_id: ctx.pane_id,
                    host: ctx.host.clone(),
                });

            let outline_hud = editor_outline::render_floating_outline_hud(
                ctx.pane_id.0,
                &self.outline_headings(cx),
                None,
                false,
                &theme,
                &outline_host,
            );

            let host_for_click = ctx.host.clone();
            div()
                .id(ElementId::Name(
                    format!("tiled-wysiwyg-editor-{pane_id}").into(),
                ))
                .key_context("Wysiwyg")
                .w_full()
                .h_full()
                .relative()
                .bg(c.editor_background)
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    host_for_click.focus_pane(pane_id, window, cx);
                })
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("tiled-wysiwyg-scroll-{pane_id}").into(),
                        ))
                        .w_full()
                        .h_full()
                        .overflow_y_scroll()
                        .track_scroll(ctx.scroll)
                        .children(row_elements),
                )
                .child(outline_hud)
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
