//! Search panel render shell — coordination only.
//!
//! The presentation lives in `editor_search::render_search_panel_overlay`;
//! this shell wires the panel state, the snapshot/IME proxies and the
//! search host into it.

use std::sync::Arc;

use gpui::*;

use crate::editor_scheduler::engine::controller::Editor;
use editor_search::SearchHost;
use theme::Theme;

impl Editor {
    pub(crate) fn render_search_panel_overlay(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.search.visible {
            return None;
        }
        let host: Arc<dyn SearchHost> =
            crate::editor_scheduler::engine::pane_host::EditorSearchHost::new(cx.weak_entity());
        editor_search::render_search_panel_overlay(
            &self.search,
            &self.search_view,
            &self.search_ime,
            &host,
            theme,
            window,
            cx,
        )
    }
}
