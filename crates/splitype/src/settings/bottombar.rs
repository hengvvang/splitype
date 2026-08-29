//! Bottom bar of a Settings area.
//!
//! Settings panels currently render an empty bottom bar so the panel shell
//! keeps its uniform top-bar / body / bottom-bar layout across all
//! area kinds. When Settings gains its own bottom-bar widgets, add them
//! here.

use gpui::*;

use crate::app::shell::Shell;
use crate::app::window::panels::PanelId;
use splitype_infra::theme::Theme;
use splitype_ui::bottombar::bottombar_container;

impl Shell {
    /// Bottom bar of a Settings area. Renders the shared bar shell with no
    /// content yet, so the area keeps the same layout as Editor / Explorer.
    pub(crate) fn render_settings_bottombar(
        &self,
        panel_id: PanelId,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(("settings-bottombar", panel_id.0))
            .into_any_element()
    }
}
