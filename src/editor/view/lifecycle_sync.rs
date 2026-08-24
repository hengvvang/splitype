//! Pre-frame bookkeeping and lifecycle state synchronization for the editor.

use gpui::*;

use crate::editor::controller::*;
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
        if let Some(entity_id) = self.pane_state(pane_id).focus.pending.take()
            && let Some(block) = self.focusable_entity_by_id(entity_id)
        {
            let focus_handle = block.read(cx).focus_handle.clone();
            focus_handle.focus(window, cx);
        }
    }

    pub(crate) fn ensure_focused_caret_visible(
        &mut self,
        pane_id: PaneId,
        window: &Window,
        cx: &App,
    ) -> bool {
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
            let max_offset_y = scroll.handle.max_offset().y.max(px(0.0));
            offset.y = offset.y.min(px(0.0)).max(-max_offset_y);
            let scroll = &mut self.pane_state(pane_id).scroll;
            scroll.handle.set_offset(offset);
        }

        true
    }

    pub(crate) fn apply_pending_scroll_into_view(
        &mut self,
        pane_id: PaneId,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .pane_state_ref(pane_id)
            .is_none_or(|state| state.scroll.scrollbar_drag.is_some())
        {
            return;
        }

        let is_pending = self
            .pane_state_ref(pane_id)
            .is_some_and(|state| state.focus.pending_scroll_active_block_into_view);
        if !is_pending {
            return;
        }

        let state = self.pane_state(pane_id);
        state.focus.pending_scroll_active_block_into_view = false;
        state.focus.pending_scroll_recheck_after_layout = false;
        state.scroll.scroll_recheck_task = None;

        self.ensure_focused_caret_visible(pane_id, window, cx);
    }


    pub(crate) fn sync_pending_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab().file.pending_save {
            self.tab_mut().file.pending_save = false;
            self.save_document(window, cx);
        }
    }

    pub(crate) fn sync_pending_save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab().file.pending_save_as {
            self.tab_mut().file.pending_save_as = false;
            self.save_document_as(window, cx);
        }
    }

    pub(crate) fn sync_pending_open_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(crate) fn sync_window_edited_state(&mut self, window: &mut Window) {
        if self.tab().file.pending_window_edited {
            self.tab_mut().file.pending_window_edited = false;
            window.set_window_edited(true);
        }
    }

    pub(crate) fn sync_scroll_viewport(
        &mut self,
        pane_id: PaneId,
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

    pub(crate) fn sync_window_title(&mut self, window: &mut Window, strings: &I18nStrings) {
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
