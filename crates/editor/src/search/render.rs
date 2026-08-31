//! Search panel render shell — coordination only.
//!
//! The presentation lives in `core_contracts::render_search_panel_overlay`;
//! this shell wires the panel state, the snapshot/IME proxies and the
//! search host into it.

use std::sync::Arc;

use gpui::*;

use crate::editor::Editor;
use core_contracts::SearchHost;
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
            crate::editor::host_bridge::EditorSearchHost::new(cx.weak_entity());
        core_contracts::render_search_panel_overlay(
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

