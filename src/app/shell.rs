//! The window shell — the OS window's root entity.
//!
//! Owns the mapping from layout areas to content entities (`AreaContent`),
//! the window-level chrome state (the in-window menu bar), and renders the
//! window chrome (custom titlebar + menu bar) above the primary editor's
//! content. Window-level state that currently lives on the Editor (panel
//! layout, overlays) migrates here incrementally; the Editor keeps only
//! its own editing state.

use std::collections::HashMap;

use gpui::*;

use crate::app::window_chrome::MenuBarState;
use crate::editor::controller::Editor;
use crate::infra::theme::ThemeManager;
use crate::splitter::NodeId;

/// The content of one area in the outer layout tree.
pub enum AreaContent {
    /// An editor with its own tab list and inner panel layout.
    Editor(Entity<Editor>),
}

/// The OS window's root entity: content areas + window lifecycle.
pub struct Shell {
    /// Content entity per outer area id.
    pub(crate) areas: HashMap<NodeId, AreaContent>,
    /// Open/hover state for the in-window titlebar menu bar.
    pub(crate) menu_bar: MenuBarState,
}

impl Shell {
    /// The window's primary (first) editor area content, if any.
    pub(crate) fn primary_editor(&self) -> Option<&Entity<Editor>> {
        self.areas.values().find_map(|area| match area {
            AreaContent::Editor(editor) => Some(editor),
        })
    }

    /// Close-button routing for the custom titlebar: delegate to the
    /// primary editor's unsaved-changes-aware close flow.
    pub(crate) fn on_titlebar_close(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !event.standard_click() {
            return;
        }
        let Some(editor) = self.primary_editor().cloned() else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| {
            editor.request_close_current_window(window, cx);
        });
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeManager>().current_arc();
        let (titlebar, menu_panel, titlebar_height) = self.render_window_chrome(&theme, window, cx);

        let mut base = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.colors.editor_background)
            // A mouse-down anywhere in the window body closes an open
            // menu; titlebar and menu panels are siblings of the body
            // container, so their clicks never reach this listener.
            .on_any_mouse_down(cx.listener(Self::on_body_mouse_down));

        if let Some(titlebar) = titlebar {
            base = base.child(titlebar);
        }

        // The custom titlebar is absolutely positioned over the window top;
        // offset the body by its height so content starts below it.
        let body = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .pt(px(titlebar_height))
            .flex()
            .min_w(px(0.0));
        let body = match self.primary_editor() {
            Some(editor) => body.child(editor.clone().into_any_element()),
            None => body,
        };
        base = base.child(body);

        if let Some(menu_panel) = menu_panel {
            base = base.child(menu_panel);
        }

        base.into_any_element()
    }
}
