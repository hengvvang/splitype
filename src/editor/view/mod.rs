//! Editor window rendering — `Editor::render` and panel views.
//!
//! # Render phases
//! 1. **Pre-frame bookkeeping** — pending focus, scroll-into-view,
//!    save / save-as / open-link, window title sync.
//! 2. **Viewport & scroll** — scroll-handle bounds, scrollbar geometry.
//! 3. **Row collection** — iterate `document.blocks()`, group into callout /
//!    footnote / plain rows, compute inter-row gaps.
//! 4. **Windowing** — visible viewport plus overscan; top / bottom spacers.
//! 5. **Scroll content** — windowed rows in scroll container with listeners.
//! 6. **Overlays** — context menu, table-insert dialog, info/drop/unsaved
//!    dialogs.
//!
//! The window chrome (custom titlebar + in-window menu bar) moved to the
//! Shell (`crate::app::window_chrome`); the per-area top bar and bottom
//! status bar live in `crate::editor::topbar` and
//! `crate::editor::bottombar`. This module covers the editor's content
//! render flow and floating overlays.

pub(crate) mod context_menu;
pub(crate) mod context_menu_actions;
pub(crate) mod context_menu_render;
pub(crate) mod dialogs;
pub(crate) mod export;

use std::time::{Duration, Instant};

use gpui::*;

use crate::editor::controller::*;
use crate::editor::tree::document::BlockEntry;
use crate::infra::i18n::{I18nManager, I18nStrings};
use crate::infra::theme::{Theme, ThemeDimensions, ThemeManager};
use crate::model::block::CalloutKind;

// ── Constants ────────────────────────────────────────────────────────────

/// Rows within this many pixels of the viewport stay mounted.
pub const RENDER_OVERDRAW_PX: f32 = 800.0;

/// One inner row inside a planned render row (callout / footnote group).
#[derive(Clone, Debug)]
enum PlannedInnerSegment {
    /// A single block row with its leading `mt` gap.
    Block { gap: f32 },
    /// A footnote subgroup: an outer gap plus per-block row gaps.
    FootnoteSubgroup { gap: f32, row_gaps: Vec<f32> },
}

/// Lightweight plan for one render row, built for every row but materialized
/// into elements only for the windowed range.
struct PlannedRow {
    /// Block range covered by this row, `[start, end)` in visible order.
    start: usize,
    end: usize,
    /// Callout accent variant when this row is a callout group.
    callout_variant: Option<CalloutKind>,
    /// The outer container's leading `mt` gap.
    outer_gap: f32,
    /// Inner rows in order; the sum of their block counts equals `end - start`.
    segments: Vec<PlannedInnerSegment>,
}

pub(crate) const SPLITYPE_REPOSITORY_URL: &str = "https://github.com/hengvvang/splitype";
pub(crate) const SPLITYPE_BUG_REPORT_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=bug_report.yml";
pub(crate) const SPLITYPE_FEATURE_REQUEST_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=feature_request.yml";
pub(crate) const SPLITYPE_DISCUSSIONS_URL: &str =
    "https://github.com/hengvvang/splitype/discussions";
pub(crate) const SPLITYPE_WIKI_URL: &str = "https://github.com/hengvvang/splitype/wiki";
pub(crate) const SPLITYPE_RELEASES_URL: &str = "https://github.com/hengvvang/splitype/releases";

pub(crate) fn open_splitype_repository(cx: &mut App) {
    cx.open_url(SPLITYPE_REPOSITORY_URL);
}

pub(crate) fn open_bug_report(cx: &mut App) {
    cx.open_url(SPLITYPE_BUG_REPORT_URL);
}

pub(crate) fn open_feature_request(cx: &mut App) {
    cx.open_url(SPLITYPE_FEATURE_REQUEST_URL);
}

pub(crate) fn open_discussions(cx: &mut App) {
    cx.open_url(SPLITYPE_DISCUSSIONS_URL);
}

use crate::editor::wysiwyg::render::layout::{
    RowSpacingInfo, callout_colors, callout_row_top_gap, editor_text_font, footnote_row_top_gap,
    row_top_gap,
};

// ── Export methods ────────────────────────────────────────────────────────

impl Editor {
    /// Apply the pane's pending focus to the window keyboard focus. Only
    /// the active pane's pending focus is applied — the window focus sits
    /// in exactly one pane, and other panes' pending targets stay queued
    /// until their pane becomes active.
    fn apply_pending_focus(&mut self, pane_id: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entity_id) = self.pane_state(pane_id).focus.pending.take()
            && let Some(block) = self.focusable_entity_by_id(entity_id)
        {
            block.read(cx).focus_handle.focus(window);
        }
    }

    fn ensure_focused_caret_visible(&mut self, pane_id: usize, window: &Window, cx: &App) -> bool {
        let Some(focused_block) = self.focused_edit_target(window, cx) else {
            return false;
        };
        let Some(active_bounds) =
            focused_block.read_with(cx, |block, _cx| block.active_range_or_cursor_bounds())
        else {
            return false;
        };

        let scroll = &self
            .pane_state_ref(pane_id)
            .expect("pane state exists")
            .scroll;
        let viewport = scroll.handle.bounds();
        let padding = px(20.0);
        let top_limit = viewport.top() + padding;
        let bottom_limit = viewport.bottom() - padding;
        let mut offset = scroll.handle.offset();
        let mut changed = false;

        if active_bounds.top() < top_limit {
            offset.y += top_limit - active_bounds.top();
            changed = true;
        } else if active_bounds.bottom() > bottom_limit {
            offset.y -= active_bounds.bottom() - bottom_limit;
            changed = true;
        }

        if changed {
            let max_offset_y = scroll.handle.max_offset().height.max(px(0.0));
            offset.y = offset.y.min(px(0.0)).max(-max_offset_y);
            let scroll = &mut self.pane_state(pane_id).scroll;
            scroll.handle.set_offset(offset);
        }

        true
    }

    fn apply_pending_scroll_into_view(
        &mut self,
        pane_id: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .pane_state_ref(pane_id)
            .is_none_or(|state| state.scroll.scrollbar_drag.is_some())
        {
            return;
        }

        if !self
            .pane_state_ref(pane_id)
            .is_some_and(|state| state.focus.pending_scroll_active_block_into_view)
        {
            return;
        }

        // scroll_to_item indexed children by position, which the spacers break;
        // the focused block is always mounted, so pixel math on its bounds works.
        let has_bounds = self.ensure_focused_caret_visible(pane_id, window, cx);
        if self
            .pane_state_ref(pane_id)
            .is_some_and(|state| state.focus.pending_scroll_recheck_after_layout)
        {
            let state = self.pane_state(pane_id);
            state.focus.pending_scroll_recheck_after_layout = false;
            self.schedule_scroll_recheck(pane_id, cx);
            return;
        }

        if !has_bounds {
            self.schedule_scroll_recheck(pane_id, cx);
            return;
        }

        let state = self.pane_state(pane_id);
        state.focus.pending_scroll_active_block_into_view = false;
        state.scroll.scroll_recheck_task = None;
    }

    /// Requests a repaint one frame out so a still-pending scroll-into-view can
    /// retry once the target block has been laid out. `cx.notify()` is swallowed
    /// when called from within `render`, so without this the retry would wait
    /// for the next external notify (e.g. the cursor blink, ~0.5s later).
    fn schedule_scroll_recheck(&mut self, pane_id: usize, cx: &mut Context<Self>) {
        let state = self.pane_state(pane_id);
        state.scroll.scroll_recheck_task =
            Some(cx.spawn(async move |this: WeakEntity<Self>, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let _ = this.update(cx, |_this, cx| cx.notify());
            }));
    }

    fn sync_pending_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab().file.pending_save {
            self.tab_mut().file.pending_save = false;
            self.save_document(window, cx);
        }
    }

    fn sync_pending_save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab().file.pending_save_as {
            self.tab_mut().file.pending_save_as = false;
            self.save_document_as(window, cx);
        }
    }

    fn sync_pending_open_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.tab_mut().file.pending_open_link.take() else {
            return;
        };

        let strings = cx.global::<I18nManager>().strings_arc();
        let buttons = [
            strings.open_link_open.as_str(),
            strings.open_link_cancel.as_str(),
        ];
        let prompt = window.prompt(
            PromptLevel::Info,
            &strings.open_link_title,
            Some(&link.prompt_target),
            &buttons,
            cx,
        );
        let window_handle = window.window_handle();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let Ok(choice) = prompt.await else {
                return;
            };
            if choice == 0 {
                let _ = cx.update_window(window_handle, |_view: AnyView, _window, cx| {
                    cx.open_url(&link.open_target);
                });
            }
        })
        .detach();
    }

    fn sync_window_edited_state(&mut self, window: &mut Window) {
        if self.tab().file.pending_window_edited {
            self.tab_mut().file.pending_window_edited = false;
            window.set_window_edited(true);
        }
    }

    fn sync_scroll_viewport(
        &mut self,
        pane_id: usize,
        viewport_size: Size<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let previous = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.scroll.last_viewport_size);
        match previous {
            Some(previous) if Self::viewport_size_changed(previous, viewport_size) => {
                let state = self.pane_state(pane_id);
                state.scroll.last_viewport_size = Some(viewport_size);
                self.request_active_block_scroll_into_view(pane_id, cx);
            }
            Some(_) => {}
            None => {
                let state = self.pane_state(pane_id);
                state.scroll.last_viewport_size = Some(viewport_size);
            }
        }
    }

    fn sync_window_title(&mut self, window: &mut Window, strings: &I18nStrings) {
        if self.tab().file.pending_window_title_refresh {
            self.tab_mut().file.pending_window_title_refresh = false;
            let title = Self::window_title(
                self.tab().file.path.as_deref(),
                self.tab().file.dirty,
                strings,
            );
            window.set_window_title(&title);
        }
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // One Editor entity serves one area, so tab()/doc() always resolve
        // to this editor's own session.

        if self.has_active_tab() {
            // The window keyboard focus sits in exactly one pane; pending
            // focus and scroll-into-view apply to the active pane here (and
            // again inside its document view once layout is measurable).
            let active_pane = self.active_pane_id();
            self.apply_pending_focus(active_pane, window, cx);
            self.apply_pending_scroll_into_view(active_pane, window, cx);
            self.tab_mut().undo.last_selection_snapshot =
                self.capture_source_selection_snapshot(cx);
            self.sync_pending_save(window, cx);
            self.sync_pending_save_as(window, cx);
            self.sync_pending_open_link(window, cx);
            self.sync_window_edited_state(window);
        }

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        if self.has_active_tab() {
            self.sync_window_title(window, &strings);
        }

        let _editor = cx.entity().downgrade();

        // Repaint when the Cmd/Ctrl follow modifier toggles so a hovered link's
        // hand cursor updates without moving the pointer. `ModifiersChanged` is
        // dispatched along the focused element's path to the root, and this root
        // is an ancestor of every block, so one listener here covers a link in any
        // block while editing. Gated to the secondary modifier so Shift during
        // selection does not repaint.
        let follow_modifier_active = window.modifiers().secondary();

        let d = &theme.dimensions;
        let c = &theme.colors;
        let panel_id = self.panel_id;
        // Pushed by the Shell every frame: how many panels the outer layout
        // holds (maximize/close controls hide for a single area) and
        // whether this tile is maximized.
        let leaf_count = self.leaf_count;
        let is_maximized = self.is_maximized;

        // One Editor entity renders its own panel tile: top bar, panes
        // layout, and bottom status bar. The outer split tree (rendered by
        // the Shell) embeds this tile as one leaf.
        let base = div()
            .id(("editor-area-tile", panel_id))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .rounded(px(d.panel_tile_radius))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .shadow_lg()
            .font(editor_text_font())
            .on_modifiers_changed(move |event, window, _| {
                if event.modifiers.secondary() != follow_modifier_active {
                    window.refresh();
                }
            })
            .capture_action(cx.listener(Self::on_copy_capture))
            .capture_action(cx.listener(Self::on_cut_capture))
            .capture_action(cx.listener(Self::on_delete_capture))
            .capture_action(cx.listener(Self::on_delete_back_capture))
            .capture_key_down(cx.listener(Self::on_editor_key_down_capture))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_save_document))
            .on_action(cx.listener(Self::on_save_document_as))
            .on_action(cx.listener(Self::on_export_html))
            .on_action(cx.listener(Self::on_export_pdf))
            .on_action(cx.listener(Self::on_toggle_view_mode_action))
            .on_action(cx.listener(Self::on_page_up))
            .on_action(cx.listener(Self::on_page_down))
            .on_action(cx.listener(Self::on_jump_to_top))
            .on_action(cx.listener(Self::on_jump_to_bottom))
            .on_action(cx.listener(Self::on_dismiss_transient_ui))
            .child(self.render_editor_topbar(
                WindowPanelKind::Editor,
                &theme,
                leaf_count,
                is_maximized,
                cx,
            ))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h(px(0.0))
                    .relative()
                    .child(self.render_editor_body(&theme, &strings, window, cx)),
            )
            .child(self.render_editor_bottombar(&theme, &strings, cx));
        let base = if let Some(context_menu) = self.render_context_menu_overlay(&theme, cx) {
            base.child(context_menu)
        } else {
            base
        };
        let base = if let Some(footnote_tooltip) = self.render_footnote_tooltip(&theme, window, cx)
        {
            base.child(footnote_tooltip)
        } else {
            base
        };
        let base = if let Some(table_dialog) = self.render_table_insert_dialog_overlay(&theme, cx) {
            base.child(table_dialog)
        } else {
            base
        };
        // Window-level dialogs (unsaved changes, drop-replace, Help-menu
        // info) render on the Shell at the window root.
        base.into_any_element()
    }
}

impl Editor {
    /// Builds this editor area's WYSIWYG document view: the scrollable block
    /// editor. One Editor entity serves one area, so every document-state
    /// access hits this editor's own tab set.
    pub(crate) fn render_document_view(
        &mut self,
        pane_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let panel_id = self.panel_id;
        // A tab that has never been rendered has an unbound scroll handle
        // (0×0 bounds). Window the first frame against the window viewport
        // instead of a 1px sliver, so the switch shows a full screen of rows
        // immediately; `track_scroll` binds the real bounds during layout
        // and later frames use them.
        let viewport_bounds = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.bounds())
            .unwrap_or_default();
        let viewport_size =
            if viewport_bounds.size.width == px(0.0) || viewport_bounds.size.height == px(0.0) {
                window.viewport_size()
            } else {
                viewport_bounds.size
            };
        // The window keyboard focus sits in exactly one pane, so pending
        // focus and scroll-into-view only apply to the active pane; other
        // panes keep theirs queued until they become active.
        if pane_id == self.active_pane_id() {
            self.apply_pending_focus(pane_id, window, cx);
            self.apply_pending_scroll_into_view(pane_id, window, cx);
        }
        self.sync_scroll_viewport(pane_id, viewport_size, cx);

        let theme = cx.global::<ThemeManager>().current_arc();
        let d = &theme.dimensions;
        let blocks = self.doc().blocks().to_vec();
        let editor = cx.entity().downgrade();
        let scroll_trigger_padding = (d.block_min_height * 0.75).max(16.0);
        let max_scroll_y = self
            .pane_state_ref(pane_id)
            .map(|state| f32::from(state.scroll.handle.max_offset().height.max(px(0.0))))
            .unwrap_or(0.0);
        let viewport_height = f32::from(viewport_bounds.size.height.max(px(1.0)));
        // Extra room below the last block so the lowest line can be scrolled up
        // to the viewport center instead of being pinned to the bottom edge.
        let scroll_beyond_bottom = viewport_height * 0.5;
        let viewport_width = f32::from(viewport_bounds.size.width.max(px(1.0)));
        let has_overflow = max_scroll_y > 0.5;

        let centered_width = Self::centered_column_width(viewport_width, &theme.dimensions);
        let current_scroll_y = self
            .pane_state_ref(pane_id)
            .map(|state| (-f32::from(state.scroll.handle.offset().y)).clamp(0.0, max_scroll_y))
            .unwrap_or(0.0);
        let scrollbar_geometry =
            Self::scrollbar_geometry(viewport_height, max_scroll_y, current_scroll_y);
        let track_height = scrollbar_geometry.track_height;
        let thumb_height = scrollbar_geometry.thumb_height;
        let thumb_top = scrollbar_geometry.thumb_top;

        let show_custom_scrollbar = has_overflow
            && self.pane_state_ref(pane_id).is_some_and(|state| {
                state.scroll.scrollbar_drag.is_some()
                    || state.scroll.scrollbar_hovered
                    || Instant::now() <= state.scroll.scrollbar_visible_until
            });

        // Spacing metadata is read on demand instead of pre-collected into a
        // Vec<RowSpacingInfo> sized to all visible blocks. For long
        // documents this skips a ~tens-of-KB allocation per frame; per-block
        // entity.read_with is a cheap immutable lock + 7-field struct copy.
        let spacing_for = |index: usize| -> RowSpacingInfo {
            blocks[index]
                .entity
                .read_with(cx, |block, _cx| RowSpacingInfo::from_block(block))
        };
        let mut previous_row_spacing = None;
        // One lightweight plan entry per render row, covering the whole
        // document; elements are only built for the windowed range below, so
        // off-screen rows cost nothing but the spacing reads.
        let mut rows: Vec<PlannedRow> = Vec::new();
        let mut row_starts: Vec<usize> = Vec::new();
        // Each row's leading `mt` gap; the top spacer subtracts the first mounted
        // row's, since that row re-applies it.
        let mut row_top_gaps: Vec<f32> = Vec::new();
        let mut index = 0usize;
        while index < blocks.len() {
            let first_spacing = spacing_for(index);
            let top_gap = row_top_gap(previous_row_spacing, first_spacing, d.block_gap);

            if let (Some(callout_group_id), Some(callout_variant)) = (
                first_spacing.callout_group_id,
                first_spacing.callout_variant,
            ) {
                let mut segments = Vec::new();
                let mut group_end = index;
                let mut previous_callout_row = None;
                while group_end < blocks.len()
                    && spacing_for(group_end).callout_group_id == Some(callout_group_id)
                {
                    let row_spacing = spacing_for(group_end);
                    if let Some(footnote_group_id) = row_spacing.footnote_group_id {
                        let mut footnote_end = group_end;
                        let mut previous_footnote_row = None;
                        let mut row_gaps = Vec::new();
                        while footnote_end < blocks.len()
                            && spacing_for(footnote_end).callout_group_id == Some(callout_group_id)
                            && spacing_for(footnote_end).footnote_group_id
                                == Some(footnote_group_id)
                        {
                            let footnote_spacing = spacing_for(footnote_end);
                            row_gaps.push(footnote_row_top_gap(previous_footnote_row, d.block_gap));
                            previous_footnote_row = Some(footnote_spacing);
                            footnote_end += 1;
                        }

                        segments.push(PlannedInnerSegment::FootnoteSubgroup {
                            gap: callout_row_top_gap(previous_callout_row, row_spacing, d),
                            row_gaps,
                        });
                        previous_callout_row = Some(spacing_for(footnote_end - 1));
                        group_end = footnote_end;
                        continue;
                    }

                    segments.push(PlannedInnerSegment::Block {
                        gap: callout_row_top_gap(previous_callout_row, row_spacing, d),
                    });
                    previous_callout_row = Some(row_spacing);
                    group_end += 1;
                }

                row_starts.push(index);
                row_top_gaps.push(top_gap);
                rows.push(PlannedRow {
                    start: index,
                    end: group_end,
                    callout_variant: Some(callout_variant),
                    outer_gap: top_gap,
                    segments,
                });
                previous_row_spacing = Some(spacing_for(group_end - 1));
                index = group_end;
                continue;
            }

            if let Some(footnote_group_id) = first_spacing.footnote_group_id {
                let mut segments = Vec::new();
                let mut group_end = index;
                let mut previous_footnote_row = None;
                while group_end < blocks.len()
                    && spacing_for(group_end).footnote_group_id == Some(footnote_group_id)
                {
                    let row_spacing = spacing_for(group_end);
                    segments.push(PlannedInnerSegment::Block {
                        gap: footnote_row_top_gap(previous_footnote_row, d.block_gap),
                    });
                    previous_footnote_row = Some(row_spacing);
                    group_end += 1;
                }

                row_starts.push(index);
                row_top_gaps.push(top_gap);
                rows.push(PlannedRow {
                    start: index,
                    end: group_end,
                    callout_variant: None,
                    outer_gap: top_gap,
                    segments,
                });
                previous_row_spacing = Some(spacing_for(group_end - 1));
                index = group_end;
                continue;
            }

            row_starts.push(index);
            row_top_gaps.push(top_gap);
            rows.push(PlannedRow {
                start: index,
                end: index + 1,
                callout_variant: None,
                outer_gap: top_gap,
                segments: Vec::new(),
            });
            previous_row_spacing = Some(first_spacing);
            index += 1;
        }

        // The focused row is always kept mounted so its caret is not blurred; a
        // table cell maps to its containing table block's row.
        let focus_row = self
            .focused_edit_target_entity_id(window, cx)
            .and_then(|id| {
                self.doc().index_for_entity_id(id).or_else(|| {
                    self.table_cell_binding(id).and_then(|binding| {
                        self.doc()
                            .index_for_entity_id(binding.table_block.entity_id())
                    })
                })
            })
            .map(|visible_index| {
                row_starts
                    .partition_point(|&start| start <= visible_index)
                    .saturating_sub(1)
            });

        // A row's first block keys its cached height; its painted top (from last
        // frame) feeds the footprints below.
        let row_first_ids: Vec<EntityId> = row_starts
            .iter()
            .map(|&start| blocks[start].entity.entity_id())
            .collect();
        let row_tops: Vec<Option<f32>> = row_starts
            .iter()
            .map(|&start| {
                blocks[start].entity.read_with(cx, |block, _cx| {
                    block
                        .last_paint()
                        .map(|paint| f32::from(paint.bounds.top()))
                })
            })
            .collect();

        // On a structural edit the row indices no longer match last frame, so the
        // cache refresh below is skipped; its block-keyed entries still hold.
        let structural_change = self
            .pane_state_ref(pane_id)
            .map(|state| {
                blocks.len() != state.scroll.prev_block_ids.len()
                    || blocks
                        .iter()
                        .zip(&state.scroll.prev_block_ids)
                        .any(|(visible, prev)| visible.entity.entity_id() != *prev)
            })
            .unwrap_or(true);
        if structural_change {
            let state = self.pane_state(pane_id);
            state.scroll.prev_block_ids = blocks.iter().map(|v| v.entity.entity_id()).collect();
        }

        // Rows mounted together last frame shared one scroll offset, so their
        // adjacent painted-top differences are scroll-free heights. Caching those,
        // not raw positions, is what keeps the window stable while scrolling.
        if !structural_change {
            if let Some((prev_start, prev_end)) = self
                .pane_state_ref(pane_id)
                .and_then(|state| state.scroll.prev_row_band)
            {
                let prev_end = prev_end.min(row_first_ids.len());
                for row in prev_start..prev_end.saturating_sub(1) {
                    if let (Some(top), Some(next_top)) = (row_tops[row], row_tops[row + 1]) {
                        let stride = next_top - top;
                        if stride > 0.0 && stride.is_finite() {
                            let state = self.pane_state(pane_id);
                            state
                                .scroll
                                .row_stride_cache
                                .insert(row_first_ids[row], stride);
                        }
                    }
                }
            }
        }

        // Unmeasured rows use the minimum block height: a lower bound, so the
        // window over-mounts rather than ever landing on a spacer.
        let estimate = d.block_min_height.max(1.0);
        let strides: Vec<f32> = row_first_ids
            .iter()
            .map(|id| {
                self.pane_state_ref(pane_id)
                    .and_then(|state| state.scroll.row_stride_cache.get(id))
                    .copied()
                    .unwrap_or(estimate)
            })
            .collect();

        // Bound the cache against block churn, only when it outgrows the live rows.
        if self.pane_state_ref(pane_id).is_some_and(|state| {
            state.scroll.row_stride_cache.len() > row_first_ids.len().saturating_mul(2)
        }) {
            let live: std::collections::HashSet<EntityId> = row_first_ids.iter().copied().collect();
            let state = self.pane_state(pane_id);
            state
                .scroll
                .row_stride_cache
                .retain(|id, _| live.contains(id));
        }

        let band = Self::visible_row_band(
            &strides,
            current_scroll_y,
            viewport_height,
            RENDER_OVERDRAW_PX,
            focus_row,
        );
        let state = self.pane_state(pane_id);
        state.scroll.prev_row_band = Some((band.run_start, band.run_end));

        // The first mounted row re-applies its `mt`, so drop it from the top
        // spacer to avoid shifting content down by a gap.
        let top_h = match row_top_gaps.get(band.run_start) {
            Some(gap) => (band.top_h - gap).max(0.0),
            None => band.top_h,
        };
        let mut block_rows: Vec<AnyElement> = Vec::with_capacity(band.run_end - band.run_start + 2);
        if top_h > 0.5 {
            block_rows.push(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .h(px(top_h))
                    .into_any_element(),
            );
        }
        for (row_index, plan) in rows.iter().enumerate() {
            if row_index < band.run_start || row_index >= band.run_end {
                continue;
            }
            block_rows.push(self.build_planned_row_element(
                plan,
                &blocks,
                editor.clone(),
                panel_id,
                centered_width,
                &theme,
                d,
            ));
        }
        if band.bottom_h > 0.5 {
            block_rows.push(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .h(px(band.bottom_h))
                    .into_any_element(),
            );
        }

        let scroll_handle = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.clone())
            .unwrap_or_default();
        let scroll_content = div()
            .id(ElementId::Name(
                format!("editor-scroll-inner-{panel_id}-{pane_id}").into(),
            ))
            .flex()
            .flex_col()
            .flex_grow()
            .h_full()
            .items_center()
            .bg(theme.colors.editor_background)
            .overflow_y_scroll()
            .scrollbar_width(px(0.0))
            .track_scroll(&scroll_handle)
            .can_drop(|dragged, _window, _cx| dragged.is::<ExternalPaths>())
            .on_drop::<ExternalPaths>(cx.listener(move |this, paths, window, cx| {
                // Dropping into this editor activates it and routes the
                // replace flow to ITS tab set.
                {
                    this.defer_shell_action(cx, move |shell, cx| {
                        shell.activate_panel(panel_id, cx)
                    });
                    this.on_external_paths_drop(paths, window, cx);
                }
            }))
            .on_hover(cx.listener(move |this, hovered, window, cx| {
                this.on_editor_hover(pane_id, hovered, window, cx);
            }))
            .capture_any_mouse_down(cx.listener(move |this, event, window, cx| {
                this.on_editor_capture_mouse_down(pane_id, event, window, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event, window, cx| {
                    this.defer_shell_action(cx, move |shell, cx| {
                        shell.activate_panel(panel_id, cx)
                    });
                    this.on_editor_mouse_down(event, window, cx);
                }),
            )
            .on_mouse_move(cx.listener(move |this, event, window, cx| {
                this.on_editor_mouse_move(pane_id, event, window, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, event, window, cx| {
                    this.on_editor_mouse_up(pane_id, event, window, cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(move |this, event, window, cx| {
                    this.on_editor_mouse_up(pane_id, event, window, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(move |this, event, window, cx| {
                this.on_editor_scroll_wheel(pane_id, event, window, cx);
            }))
            .p(px(d.editor_padding))
            .pb(px(d.editor_padding
                + scroll_trigger_padding
                + scroll_beyond_bottom))
            .children(block_rows);
        let scroll_content = if self.tab().mode == EditorMode::Wysiwyg {
            scroll_content.on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event, window, cx| {
                    this.defer_shell_action(cx, move |shell, cx| {
                        shell.activate_panel(panel_id, cx)
                    });
                    this.on_editor_context_menu_mouse_down(event, window, cx);
                }),
            )
        } else {
            scroll_content
        };

        let content_area = div()
            .id(ElementId::Name(
                format!("editor-scroll-{panel_id}-{pane_id}").into(),
            ))
            .w_full()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .bg(theme.colors.editor_background)
            .relative()
            .child(scroll_content);

        let content_area = if show_custom_scrollbar {
            let scrollbar_editor = editor.clone();
            let track_origin_y = f32::from(viewport_bounds.origin.y);
            content_area.child(
                div()
                    .id(ElementId::Name(
                        format!("editor-scrollbar-thumb-{panel_id}-{pane_id}").into(),
                    ))
                    .absolute()
                    .occlude()
                    .top(px(thumb_top))
                    .right(px(d.scrollbar_right))
                    .w(px(d.scrollbar_width))
                    .h(px(thumb_height))
                    .rounded(px(999.0))
                    .bg(theme.colors.scrollbar_thumb)
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, hovered, window, cx| {
                        this.on_editor_hover(pane_id, hovered, window, cx);
                    }))
                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                        let pointer_offset_y =
                            f32::from(event.position.y) - track_origin_y - thumb_top;
                        let _ = scrollbar_editor.update(cx, |editor, cx| {
                            cx.stop_propagation();
                            {
                                editor.defer_shell_action(cx, move |shell, cx| {
                                    shell.activate_panel(panel_id, cx);
                                });
                                editor.start_scrollbar_drag(
                                    pane_id,
                                    pointer_offset_y,
                                    track_height,
                                    thumb_height,
                                    max_scroll_y,
                                    cx,
                                );
                            }
                        });
                    })
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |_thumb_bounds, _, window, _| {
                                window.on_mouse_event({
                                    let editor = editor.clone();
                                    move |_event: &MouseUpEvent, phase, _window, cx| {
                                        if !phase.bubble() {
                                            return;
                                        }
                                        let _ = editor.update(cx, |editor, cx| {
                                            editor.end_scrollbar_drag(pane_id, cx);
                                        });
                                    }
                                });

                                window.on_mouse_event({
                                    let editor = editor.clone();
                                    move |event: &MouseMoveEvent, phase, _window, cx| {
                                        if !phase.bubble() || !event.dragging() {
                                            return;
                                        }

                                        let pointer_y_in_track =
                                            f32::from(event.position.y) - track_origin_y;
                                        let _ = editor.update(cx, |editor, cx| {
                                            editor.update_scrollbar_drag(
                                                pane_id,
                                                pointer_y_in_track,
                                                cx,
                                            );
                                        });
                                    }
                                });
                            },
                        )
                        .size_full(),
                    ),
            )
        } else {
            content_area
        };

        content_area.into_any_element()
    }

    /// Materializes one planned render row into its element tree. Only rows
    /// inside the windowed range are built, so off-screen rows cost nothing
    /// but their plan (spacing reads).
    fn build_planned_row_element(
        &self,
        plan: &PlannedRow,
        blocks: &[BlockEntry],
        editor: WeakEntity<Self>,
        panel_id: usize,
        centered_width: f32,
        theme: &Theme,
        d: &ThemeDimensions,
    ) -> AnyElement {
        debug_assert_eq!(
            plan.start
                + if plan.segments.is_empty() {
                    // Single block rows carry no inner segments.
                    1
                } else {
                    plan.segments
                        .iter()
                        .map(|segment| match segment {
                            PlannedInnerSegment::Block { .. } => 1,
                            PlannedInnerSegment::FootnoteSubgroup { row_gaps, .. } => {
                                row_gaps.len()
                            }
                        })
                        .sum::<usize>()
                },
            plan.end,
            "planned row segment block counts must match its block range"
        );

        let attach_context_menu = |row: Div, entity_id: EntityId| -> Div {
            if self.tab().mode == EditorMode::Wysiwyg {
                let row_editor = editor.clone();
                row.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    let _ = row_editor.update(cx, |editor, cx| {
                        if let Some(shell) = editor.shell.clone() {
                            let _ =
                                shell.update(cx, |shell, cx| shell.activate_panel(panel_id, cx));
                        }
                        editor.on_block_context_menu_mouse_down(entity_id, event, window, cx);
                    });
                })
            } else {
                row
            }
        };

        match (plan.callout_variant, plan.segments.is_empty()) {
            // Single block row.
            (None, true) => {
                let entity = blocks[plan.start].entity.clone();
                let entity_id = entity.entity_id();
                let row = div()
                    .w(px(centered_width))
                    .max_w(relative(1.0))
                    .flex_shrink_0()
                    .mt(px(plan.outer_gap))
                    .child(entity.clone());
                attach_context_menu(row, entity_id).into_any_element()
            }
            // Plain footnote group: footnote definitions render as simple text
            // rows without the former card shell.
            (None, false) => {
                let mut children = Vec::new();
                let mut block_offset = plan.start;
                for segment in &plan.segments {
                    let PlannedInnerSegment::Block { gap } = segment else {
                        debug_assert!(false, "plain footnote group cannot contain subgroups");
                        continue;
                    };
                    let entity = blocks[block_offset].entity.clone();
                    let entity_id = entity.entity_id();
                    let row = div()
                        .w_full()
                        .flex_shrink_0()
                        .mt(px(*gap))
                        .child(entity.clone());
                    children.push(attach_context_menu(row, entity_id).into_any_element());
                    block_offset += 1;
                }
                div()
                    .w(px(centered_width))
                    .max_w(relative(1.0))
                    .flex_shrink_0()
                    .mt(px(plan.outer_gap))
                    .children(children)
                    .into_any_element()
            }
            // Callout group (possibly with footnote subgroups inside).
            (Some(variant), _) => {
                let (accent, _background) = callout_colors(variant, theme);
                let mut group_children = Vec::new();
                let mut block_offset = plan.start;
                for segment in &plan.segments {
                    match segment {
                        PlannedInnerSegment::Block { gap } => {
                            let entity = blocks[block_offset].entity.clone();
                            let entity_id = entity.entity_id();
                            let row = div()
                                .w_full()
                                .flex_shrink_0()
                                .mt(px(*gap))
                                .child(entity.clone());
                            group_children
                                .push(attach_context_menu(row, entity_id).into_any_element());
                            block_offset += 1;
                        }
                        PlannedInnerSegment::FootnoteSubgroup { gap, row_gaps } => {
                            let mut footnote_children = Vec::new();
                            for row_gap in row_gaps {
                                let entity = blocks[block_offset].entity.clone();
                                let entity_id = entity.entity_id();
                                let row = div()
                                    .w_full()
                                    .flex_shrink_0()
                                    .mt(px(*row_gap))
                                    .child(entity.clone());
                                footnote_children
                                    .push(attach_context_menu(row, entity_id).into_any_element());
                                block_offset += 1;
                            }
                            group_children.push(
                                div()
                                    .w_full()
                                    .flex_shrink_0()
                                    .mt(px(*gap))
                                    .children(footnote_children)
                                    .into_any_element(),
                            );
                        }
                    }
                }
                div()
                    .w(px(centered_width))
                    .max_w(relative(1.0))
                    .flex_shrink_0()
                    .mt(px(plan.outer_gap))
                    .flex()
                    .flex_col()
                    .gap(px(0.0))
                    .px(px(d.callout_padding_x))
                    .py(px(d.callout_padding_y))
                    .rounded_r(px(d.callout_radius))
                    .border_l(px(d.callout_border_width))
                    .border_color(accent)
                    .children(group_children)
                    .into_any_element()
            }
        }
    }
}
