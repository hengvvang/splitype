//! Pre-frame bookkeeping and lifecycle state synchronization for the editor.

use gpui::*;

use crate::editor::engine::controller::*;
use crate::infra::i18n::{I18nManager, I18nStrings};

impl Editor {
    /// Apply the pane's pending focus to the window keyboard focus. Only
    /// the active pane's pending focus is applied — the window focus sits
    /// in exactly one pane, and other panes' pending targets stay queued
    /// until their pane becomes active.
    pub(crate) fn apply_pending_focus(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let kind = self.pane_kind(pane_id).unwrap_or(EditorPaneKind::Wysiwyg);
        match kind {
            EditorPaneKind::SourceCode => {
                if let Some(state) = self.pane_state_mut(pane_id) {
                    if state.source_code.focus_handle.is_none() {
                        state.source_code.focus_handle = Some(cx.focus_handle());
                    }
                    if let Some(ref handle) = state.source_code.focus_handle {
                        if !handle.is_focused(window) {
                            handle.focus(window, cx);
                        }
                    }
                }
            }
            EditorPaneKind::Wysiwyg => {
                if let Some(state) = self.pane_state_mut(pane_id) {
                    if let Some(entity_id) = state.focus.pending.take()
                        && let Some(block) = self.focusable_entity_by_id(entity_id)
                    {
                        let focus_handle = block.read(cx).focus_handle.clone();
                        focus_handle.focus(window, cx);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn apply_pending_autoscroll(
        &mut self,
        pane_id: PaneId,
        window: &Window,
        cx: &App,
    ) {
        if self
            .pane_state_ref(pane_id)
            .is_none_or(|state| state.scroll.scrollbar_drag.is_some())
        {
            return;
        }

        let strategy = self
            .pane_state_mut(pane_id)
            .and_then(|s| s.scroll.pending_autoscroll.take());

        if let Some(strategy) = strategy {
            self.execute_autoscroll(pane_id, strategy, window, cx);
        }
    }

    pub(crate) fn execute_autoscroll(
        &mut self,
        pane_id: PaneId,
        strategy: crate::editor::engine::controller::AutoscrollStrategy,
        window: &Window,
        cx: &App,
    ) -> bool {
        use crate::editor::engine::controller::{AutoscrollStrategy, EditorPaneKind};

        let kind = self
            .session
            .root
            .tree
            .find_leaf_kind(pane_id.0)
            .unwrap_or(EditorPaneKind::Wysiwyg);

        let active_bounds: Option<Bounds<Pixels>> = (|| {
            match kind {
                EditorPaneKind::Wysiwyg => {
                    let target_block = self
                        .focused_edit_target(window, cx)
                        .or_else(|| {
                            let active_entity_id =
                                self.pane_state_ref(pane_id)?.focus.active_entity?;
                            self.focusable_entity_by_id(active_entity_id)
                        })
                        .or_else(|| {
                            let doc = self.active_doc()?;
                            doc.blocks().iter().find_map(|entry| {
                                let block = entry.entity.read(cx);
                                if block.search_matches.iter().any(|(_, is_active)| *is_active) {
                                    Some(entry.entity.clone())
                                } else {
                                    None
                                }
                            })
                        })?;
                    target_block.read_with(cx, |block, _cx| block.active_range_or_cursor_bounds())
                }
                EditorPaneKind::SourceCode => None,
                EditorPaneKind::Preview => None,
                EditorPaneKind::Outline => None,
            }
        })();

        let Some(active_bounds) = active_bounds else {
            return false;
        };

        let scroll = &self
            .pane_state_ref(pane_id)
            .expect("pane state exists")
            .scroll;
        let viewport = scroll.handle.bounds();
        let mut offset = scroll.handle.offset();
        let mut changed = false;

        match strategy {
            AutoscrollStrategy::Fit { margin } => {
                let top_limit = viewport.top() + margin;
                let bottom_limit = viewport.bottom() - margin;
                if active_bounds.top() < top_limit {
                    offset.y += top_limit - active_bounds.top();
                    changed = true;
                } else if active_bounds.bottom() > bottom_limit {
                    offset.y -= active_bounds.bottom() - bottom_limit;
                    changed = true;
                }
            }
            AutoscrollStrategy::Center => {
                let target_center = (active_bounds.top() + active_bounds.bottom()) / 2.0;
                let viewport_center = (viewport.top() + viewport.bottom()) / 2.0;
                offset.y += viewport_center - target_center;
                changed = true;
            }
            AutoscrollStrategy::Top { margin } => {
                let top_limit = viewport.top() + margin;
                offset.y += top_limit - active_bounds.top();
                changed = true;
            }
            AutoscrollStrategy::Bottom { margin } => {
                let bottom_limit = viewport.bottom() - margin;
                offset.y -= active_bounds.bottom() - bottom_limit;
                changed = true;
            }
        }

        if changed {
            let max_offset_y = scroll.handle.max_offset().y.max(px(0.0));
            offset.y = offset.y.min(px(0.0)).max(-max_offset_y);
            if let Some(state) = self.pane_state_mut(pane_id) {
                state.scroll.handle.set_offset(offset);
            }
        }

        true
    }

    pub(crate) fn sync_pending_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().is_some_and(|t| t.file.pending_save) {
            if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_save = false;
            }
            self.save_document(window, cx);
        }
    }

    pub(crate) fn sync_pending_save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().is_some_and(|t| t.file.pending_save_as) {
            if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_save_as = false;
            }
            self.save_document_as(window, cx);
        }
    }

    pub(crate) fn sync_pending_open_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.active_tab_mut().and_then(|t| t.file.pending_open_link.take()) else {
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

    pub(crate) fn sync_window_edited_state(&mut self, window: &mut Window) {
        if self.active_tab().is_some_and(|t| t.file.pending_window_edited) {
            if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_window_edited = false;
            }
            window.set_window_edited(true);
        }
    }

    pub(crate) fn sync_scroll_viewport(
        &mut self,
        pane_id: PaneId,
        viewport_size: Size<Pixels>,
        _cx: &mut Context<Self>,
    ) {
        let previous = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.scroll.last_viewport_size);
        match previous {
            Some(previous) if Self::viewport_size_changed(previous, viewport_size) => {
                if let Some(state) = self.pane_state_mut(pane_id) {
                    state.scroll.last_viewport_size = Some(viewport_size);
                }
            }
            Some(_) => {}
            None => {
                if let Some(state) = self.pane_state_mut(pane_id) {
                    state.scroll.last_viewport_size = Some(viewport_size);
                }
            }
        }
    }

    pub(crate) fn sync_window_title(&mut self, window: &mut Window, strings: &I18nStrings) {
        if self.active_tab().is_some_and(|t| t.file.pending_window_title_refresh) {
            let (path, dirty) = if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_window_title_refresh = false;
                (tab.file.path.clone(), tab.file.dirty)
            } else {
                (None, false)
            };
            let title = Self::window_title(
                path.as_deref(),
                dirty,
                strings,
            );
            window.set_window_title(&title);
        }
    }
}
