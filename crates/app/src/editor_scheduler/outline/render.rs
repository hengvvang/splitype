//! Outline HUD — Notion-style floating equal-length ticks rail and hover TOC popover.

use std::time::Duration;

use gpui::*;

use crate::editor_scheduler::engine::controller::{Editor, EditorPaneKind, PaneId};
use crate::editor_scheduler::outline::build_outline_headings_from_doc;
use editor_model::outline_headings_from_markdown;
use theme::Theme;

impl Editor {
    /// Rebuilds this editor's outline headings list from the active tab and current pane mode.
    pub(crate) fn sync_editor_outline(
        &mut self,
        pane_id: PaneId,
        kind: EditorPaneKind,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.active_tab() else {
            self.outline.headings.clear();
            self.outline.active_index = None;
            return;
        };

        let tab_idx = self.session.active_tab_index();
        let revision = tab.document_revision;
        let file_path = tab.file.path.clone();

        match kind {
            EditorPaneKind::SourceCode => {
                let source_text = self
                    .pane_state_ref(pane_id)
                    .and_then(|p| p.as_source_code())
                    .map(|s| s.text.as_str())
                    .unwrap_or("");

                let text_to_parse = if source_text.is_empty() {
                    self.serialized_document_text(cx)
                } else {
                    source_text.to_string()
                };

                let hash = Self::hash_str(&text_to_parse);
                if self.outline.synced_tab_index == Some(tab_idx)
                    && self.outline.synced_file_path == file_path
                    && self.outline.synced_revision == Some(revision)
                    && self.outline.synced_hash == hash
                {
                    return;
                }

                self.outline.headings = outline_headings_from_markdown(&text_to_parse);
                self.outline.synced_tab_index = Some(tab_idx);
                self.outline.synced_file_path = file_path;
                self.outline.synced_revision = Some(revision);
                self.outline.synced_hash = hash;
            }
            EditorPaneKind::Wysiwyg => {
                if self.outline.synced_tab_index == Some(tab_idx)
                    && self.outline.synced_file_path == file_path
                    && self.outline.synced_revision == Some(revision)
                {
                    return;
                }

                let mut headings = build_outline_headings_from_doc(self.doc(), cx);
                if headings.is_empty() {
                    headings = outline_headings_from_markdown(&self.doc().serialize_markdown(cx));
                }

                self.outline.headings = headings;
                self.outline.synced_tab_index = Some(tab_idx);
                self.outline.synced_file_path = file_path;
                self.outline.synced_revision = Some(revision);
                self.outline.synced_hash = 0;
            }
            EditorPaneKind::Preview => {
                let text = self.serialized_document_text(cx);
                let hash = Self::hash_str(&text);
                if self.outline.synced_tab_index == Some(tab_idx)
                    && self.outline.synced_file_path == file_path
                    && self.outline.synced_revision == Some(revision)
                    && self.outline.synced_hash == hash
                {
                    return;
                }

                self.outline.headings = outline_headings_from_markdown(&text);
                self.outline.synced_tab_index = Some(tab_idx);
                self.outline.synced_file_path = file_path;
                self.outline.synced_revision = Some(revision);
                self.outline.synced_hash = hash;
            }
        }
    }

    /// Sets whether the outline HUD popover is hovered with a debounce on exit.
    pub(crate) fn set_outline_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.outline.close_token = self.outline.close_token.wrapping_add(1);
        if hovered {
            if !self.outline.is_hovered {
                self.outline.is_hovered = true;
                cx.notify();
            }
        } else {
            let token = self.outline.close_token;
            let weak_editor = cx.entity().downgrade();
            cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                let _ = weak_editor.update(cx, |editor, cx| {
                    if editor.outline.close_token == token && editor.outline.is_hovered {
                        editor.outline.is_hovered = false;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
    }

    /// Navigates the editor to the specified heading in the outline.
    pub(crate) fn navigate_to_outline_index(
        &mut self,
        index: usize,
        kind: EditorPaneKind,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) {
        self.outline.active_index = Some(index);
        let Some(node) = self.outline.headings.get(index).cloned() else {
            return;
        };

        match kind {
            EditorPaneKind::Wysiwyg => {
                if let Some(entity_id) = node.block_id {
                    self.focus_block(entity_id);
                    if let Some(block) = self.doc().block_entity_by_id(entity_id) {
                        Self::reset_block_cursor(&block, 0, cx);
                    }
                    self.request_autoscroll_active_pane(
                        crate::editor_scheduler::engine::controller::AutoscrollStrategy::Top {
                            margin: px(40.0),
                        },
                        cx,
                    );
                } else {
                    let fallback_entity = self.doc().blocks().get(node.block_index).map(|e| e.entity.clone());
                    if let Some(entity) = fallback_entity {
                        let entity_id = entity.entity_id();
                        self.focus_block(entity_id);
                        Self::reset_block_cursor(&entity, 0, cx);
                        self.request_autoscroll_active_pane(
                            crate::editor_scheduler::engine::controller::AutoscrollStrategy::Top {
                                margin: px(40.0),
                            },
                            cx,
                        );
                    }
                }
            }
            EditorPaneKind::SourceCode => {
                let pane_id = self.active_pane_id();
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    let line_start = source.line_start_offset(node.block_index);
                    source.move_to(line_start, false);
                    source.cursor = line_start;
                }

                let font_size = theme.typography.code_size.max(12.0);
                let line_height = (font_size * theme.typography.text_line_height).round().max(18.0);
                let target_y = (node.block_index as f32 * line_height) - 40.0;
                if let Some(state) = self.pane_state_mut(pane_id) {
                    state.scroll.handle.set_offset(point(px(0.0), px(-target_y.max(0.0))));
                }
            }
            EditorPaneKind::Preview => {
                let pane_id = self.active_pane_id();
                self.request_autoscroll(
                    pane_id,
                    crate::editor_scheduler::engine::controller::AutoscrollStrategy::Top {
                        margin: px(40.0),
                    },
                    cx,
                );
            }
        }
        cx.notify();
    }

    /// Updates the active section index based on current focused block or scroll position.
    pub(crate) fn update_active_outline_section(
        &mut self,
        pane_id: PaneId,
        kind: EditorPaneKind,
        theme: &Theme,
    ) {
        if self.outline.headings.is_empty() {
            self.outline.active_index = None;
            return;
        }

        match kind {
            EditorPaneKind::SourceCode => {
                let scroll_offset_y = self
                    .pane_state_ref(pane_id)
                    .map(|s| f32::from(s.scroll.handle.offset().y).abs())
                    .unwrap_or(0.0);

                let font_size = theme.typography.code_size.max(12.0);
                let line_height = (font_size * theme.typography.text_line_height).round().max(18.0);
                let visible_line = ((scroll_offset_y + 30.0) / line_height) as usize;

                let mut active = 0;
                for (idx, heading) in self.outline.headings.iter().enumerate() {
                    if heading.block_index <= visible_line {
                        active = idx;
                    } else {
                        break;
                    }
                }
                self.outline.active_index = Some(active);
            }
            EditorPaneKind::Wysiwyg => {
                if let Some(active_id) = self.active_pane_focus().active_entity {
                    if let Some(active_idx) = self
                        .doc()
                        .blocks()
                        .iter()
                        .position(|b| b.entity.entity_id() == active_id)
                    {
                        let mut best_match = 0;
                        for (idx, heading) in self.outline.headings.iter().enumerate() {
                            if heading.block_index <= active_idx {
                                best_match = idx;
                            } else {
                                break;
                            }
                        }
                        self.outline.active_index = Some(best_match);
                        return;
                    }
                }

                if self.outline.active_index.is_none() {
                    self.outline.active_index = Some(0);
                }
            }
            EditorPaneKind::Preview => {
                if self.outline.active_index.is_none() {
                    self.outline.active_index = Some(0);
                }
            }
        }
    }

    /// Renders the Notion-style Floating Outline HUD (Equal-length ticks rail + Hover Popover TOC card).
    ///
    /// Coordination only: sync the heading list, track the active section,
    /// then hand the data plus an [`OutlineHost`] proxy to the
    /// presentation in `editor_outline`.
    pub(crate) fn render_floating_outline_hud(
        &mut self,
        pane_id: PaneId,
        kind: EditorPaneKind,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_editor_outline(pane_id, kind, cx);
        if self.outline.headings.is_empty() {
            return div().into_any_element();
        }

        self.update_active_outline_section(pane_id, kind, theme);

        let host: std::sync::Arc<dyn editor_outline::OutlineHost> =
            crate::editor_scheduler::engine::pane_host::EditorOutlineHost::new(
                cx.weak_entity(),
                kind,
                theme.clone(),
            );
        editor_outline::render_floating_outline_hud(
            pane_id,
            &self.outline.headings,
            self.outline.active_index,
            self.outline.is_hovered,
            theme,
            &host,
        )
    }
}

