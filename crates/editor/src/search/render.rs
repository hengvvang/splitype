//! Search panel render shell — coordination only.
//!
//! The presentation lives in `ui::render_search_panel_overlay`;
//! this shell wires the panel state, the snapshot/IME proxies and the
//! search host into it.

use std::sync::Arc;

use gpui::*;

use crate::editor::Editor;
use editor_contracts::SearchHost;
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
            crate::editor::search_host::EditorSearchHost::new(cx.weak_entity());
        ui::render_search_panel_overlay(
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
