//! Pre-frame bookkeeping and lifecycle state synchronization for the editor.

use gpui::*;

use std::path::Path;

use crate::editor::Editor;
use config::language::{I18nManager, I18nStrings};
use editor_contracts::{AutoscrollStrategy, PaneId};

impl Editor {
    /// Apply the pane's pending focus to the window keyboard focus.
    pub fn apply_pending_focus(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(handle) = state.pane.focus_handle(cx) {
                if !handle.is_focused(window) {
                    handle.focus(window, cx);
                }
            }
        }
    }

    pub fn apply_pending_autoscroll(&mut self, pane_id: PaneId, window: &Window, cx: &App) {
        if self
            .pane_state_ref(pane_id)
            .is_none_or(|state| state.scroll.scrollbar_drag.is_some())
        {
            return;
        }

        let strategy = self
            .pane_state_mut(pane_id)
            .and_then(|state| state.scroll.pending_autoscroll.take());
        let Some(strategy) = strategy else {
            return;
        };

        self.execute_autoscroll(pane_id, strategy, window, cx);
    }

    pub fn execute_autoscroll(
        &mut self,
        pane_id: PaneId,
        strategy: AutoscrollStrategy,
        _window: &Window,
        _cx: &App,
    ) -> bool {
        let active_bounds: Option<Bounds<Pixels>> = None;

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

    pub fn sync_pending_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().is_some_and(|t| t.file.pending_save) {
            if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_save = false;
            }
            self.save_document(window, cx);
        }
    }

    pub fn sync_pending_save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().is_some_and(|t| t.file.pending_save_as) {
            if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_save_as = false;
            }
            self.save_document_as(window, cx);
        }
    }

    pub fn sync_pending_open_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self
            .active_tab_mut()
            .and_then(|t| t.file.pending_open_link.take())
        else {
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

    pub fn sync_window_edited_state(&mut self, window: &mut Window) {
        if self
            .active_tab()
            .is_some_and(|t| t.file.pending_window_edited)
        {
            if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_window_edited = false;
            }
            window.set_window_edited(true);
        }
    }

    pub fn sync_scroll_viewport(
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

    /// Builds the OS window title, including the dirty marker when the
    /// document has unsaved changes.
    pub fn window_title(file_path: Option<&Path>, is_dirty: bool, strings: &I18nStrings) -> String {
        let base_title = file_path
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if is_dirty {
            format!("{}{} - Splitype", strings.dirty_title_marker, base_title)
        } else if base_title.is_empty() {
            "Splitype".to_string()
        } else {
            format!("{} - Splitype", base_title)
        }
    }

    pub fn sync_window_title(&mut self, window: &mut Window, strings: &I18nStrings) {
        if self
            .active_tab()
            .is_some_and(|t| t.file.pending_window_title_refresh)
        {
            let (path, dirty) = if let Some(tab) = self.active_tab_mut() {
                tab.file.pending_window_title_refresh = false;
                (tab.file.path.clone(), tab.file.dirty)
            } else {
                (None, false)
            };
            let title = Self::window_title(path.as_deref(), dirty, strings);
            window.set_window_title(&title);
        }
    }
}
