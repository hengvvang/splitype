//! Bottom bar of a Settings area.
//!
//! Settings areas currently render an empty bottom bar so the area shell
//! keeps its uniform top-bar / midcontainer / bottom-bar layout across all
//! area kinds. When Settings gains its own bottom-bar widgets, add them
//! here.

use gpui::*;

use crate::infra::theme::Theme;
use crate::ui::components::bottombar::bottombar_container;

impl crate::editor::controller::Editor {
    /// Bottom bar of a Settings area. Renders the shared bar shell with no
    /// content yet, so the area keeps the same layout as Editor / Explorer.
    pub(crate) fn render_settings_bottombar(
        &self,
        area_id: usize,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(("settings-bottombar", area_id))
            .into_any_element()
    }
}
