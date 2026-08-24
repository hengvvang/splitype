//! Shortcuts settings tab: Document Actions, Interface & View Controls.

use gpui::*;

use crate::infra::theme::Theme;
use crate::settings::tabs::common::{make_row, make_section};
use crate::settings::window::SettingsWindow;

impl SettingsWindow {
    pub(crate) fn render_shortcuts_tab(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;
        let toggle_section_ed = cx.entity().downgrade();

        let mut sections: Vec<AnyElement> = Vec::new();

        // Section 1: Document Actions
        let sec1_key = "doc_actions";
        let mut sec1_items = Vec::new();

        for item in crate::settings::shortcuts_data::doc_action_shortcuts() {
            let ctrl_sc = div()
                .px(px(8.0))
                .py(px(2.0))
                .rounded(px(d.badge_radius))
                .bg(c.dialog_secondary_button_hover)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(c.text_default)
                .child(item.shortcut)
                .into_any_element();

            sec1_items.push(make_row(inner_border_color, c, d, item.name, item.description, ctrl_sc));
        }

        sections.push(make_section(
            c,
            d,
            "win-sec-doc-actions",
            sec1_key,
            "Document Actions",
            self.expanded_sections.contains(sec1_key),
            toggle_section_ed.clone(),
            sec1_items,
        ));

        // Section 2: Interface & View Controls
        let sec2_key = "view_controls";
        let mut sec2_items = Vec::new();

        for item in crate::settings::shortcuts_data::interface_view_shortcuts() {
            let ctrl_sc = div()
                .px(px(8.0))
                .py(px(2.0))
                .rounded(px(d.badge_radius))
                .bg(c.dialog_secondary_button_hover)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(c.text_default)
                .child(item.shortcut)
                .into_any_element();

            sec2_items.push(make_row(inner_border_color, c, d, item.name, item.description, ctrl_sc));
        }

        sections.push(make_section(
            c,
            d,
            "win-sec-view-controls",
            sec2_key,
            "Interface & View Controls",
            self.expanded_sections.contains(sec2_key),
            toggle_section_ed,
            sec2_items,
        ));

        sections
    }
}
