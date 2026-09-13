//! Links widget render shell — coordination only.

use gpui::*;

use crate::editor::Editor;
use theme::Theme;

impl Editor {
    pub(crate) fn render_links_widget(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let state = self.links_widget.as_ref()?;
        let host = self.links_host.clone();
        Some(crate::links::render_links_widget(
            state,
            &host,
            theme,
            window,
            cx,
        ))
    }
}
