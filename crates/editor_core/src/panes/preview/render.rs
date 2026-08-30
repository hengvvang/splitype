//! Preview pane render shell — coordination only.
//!
//! The presentation lives in `editor_preview::render`; this shell
//! refreshes the preview tree, applies pending focus/autoscroll and hands
//! the pane state plus the `PaneRenderContext` (id, scroll, host) to the
//! mode crate.

use gpui::*;

use crate::engine::controller::{Editor, PaneId};
use editor_model::PaneRenderContext;
use config::language::I18nStrings;
use theme::Theme;

impl Editor {
    pub(crate) fn render_preview_pane(
        &mut self,
        pane_id: PaneId,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.refresh_preview_blocks(pane_id, cx);

        if pane_id == self.active_pane_id() {
            self.apply_pending_focus(pane_id, window, cx);
            self.apply_pending_autoscroll(pane_id, window, cx);
        }

        let scroll = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.clone())
            .unwrap_or_default();
        let host = self.pane_host.clone();
        let Some(state) = self.pane_state_ref(pane_id).and_then(|s| s.as_preview()) else {
            return div().w_full().h_full().into_any_element();
        };

        let is_focused = self.focused_pane_id == Some(pane_id);
        let view = PaneRenderContext {
            pane_id,
            is_focused,
            scroll: &scroll,
            host: &host,
        };
        let preview_body = editor_preview::render_preview_pane(state, &view, theme, strings, window, cx);
        let outline_hud = self.render_floating_outline_hud(pane_id, crate::engine::session::PaneKindId::PREVIEW, theme, cx);
        div()
            .relative()
            .w_full()
            .h_full()
            .child(preview_body)
            .child(outline_hud)
            .into_any_element()
    }
}
