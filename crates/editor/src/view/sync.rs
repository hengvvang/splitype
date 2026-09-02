//! Pre-frame bookkeeping and lifecycle state synchronization for the editor.

use gpui::*;

use std::path::Path;

use crate::editor::Editor;
use config::language::{I18nManager, I18nStrings};
use editor_contracts::PaneId;

impl Editor {
    /// Apply the pane's pending focus to the window keyboard focus.
    pub fn apply_pending_focus(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.search.visible {
            if self.search.search_focus_handle.is_focused(window)
                || self.search.replace_focus_handle.is_focused(window)
            {
                return;
            }
        }

        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(handle) = state.pane.focus_handle(cx) {
                if !handle.is_focused(window) && self.focused_pane_id == Some(pane_id) {
                    handle.focus(window, cx);
                }
            }
        }
    }

    /// Applies a pane-computed content Y offset to the pane scroll handle.
    pub fn scroll_pane_to(&mut self, pane_id: PaneId, target_y: f32, _window: &Window, _cx: &App) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            state
                .scroll
                .handle
                .set_offset(point(px(0.0), px(-target_y.max(0.0))));
        }
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

    fn viewport_size_changed(previous: Size<Pixels>, current: Size<Pixels>) -> bool {
        const EPSILON: f32 = 0.5;

        (f32::from(previous.width) - f32::from(current.width)).abs() > EPSILON
            || (f32::from(previous.height) - f32::from(current.height)).abs() > EPSILON
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
