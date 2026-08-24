use gpui::*;

use crate::app::shell::Shell;
use crate::app::window_panels::PanelId;
use crate::infra::theme::Theme;
use crate::settings::common::{make_row, make_section};

impl Shell {
    pub(crate) fn render_panel_shortcuts_tab(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;

        let mut sections: Vec<AnyElement> = Vec::new();

        // Section 1: Document Actions
        let sec1_key = "doc_actions";
        let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
        let mut sec1_items = Vec::new();

        if is_sec1_expanded {
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
        }

        sections.push(make_section(
            c,
            d,
            ("pref-sec-doc-actions", panel_id.0),
            "Document Actions",
            is_sec1_expanded,
            self.toggle_settings_section_handler(cx, sec1_key),
            sec1_items,
        ));

        // Section 2: Interface & View Controls
        let sec2_key = "view_controls";
        let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
        let mut sec2_items = Vec::new();

        if is_sec2_expanded {
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
        }

        sections.push(make_section(
            c,
            d,
            ("pref-sec-view-controls", panel_id.0),
            "Interface & View Controls",
            is_sec2_expanded,
            self.toggle_settings_section_handler(cx, sec2_key),
            sec2_items,
        ));

        sections
    }
}
