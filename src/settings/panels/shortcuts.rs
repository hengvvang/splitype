//! Shortcuts settings tab for the in-editor slide-over panel.

use gpui::*;

use crate::app::shell::Shell;
use crate::infra::theme::Theme;
use crate::settings::panels::common::{make_row, make_section};

impl Shell {
    pub(crate) fn render_panel_shortcuts_tab(
        &mut self,
        panel_id: usize,
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
            let doc_shortcuts = [
                (
                    "Save Document",
                    "Save active file changes to disk",
                    "Ctrl + S",
                ),
                (
                    "Save Document As",
                    "Save active document with a new name",
                    "Ctrl + Shift + S",
                ),
                (
                    "New Window",
                    "Open a new editor window instance",
                    "Ctrl + N",
                ),
                (
                    "Close Window",
                    "Close the currently focused editor window",
                    "Ctrl + W",
                ),
            ];

            for (name, desc, sc) in doc_shortcuts.iter() {
                let ctrl_sc = div()
                    .px(px(8.0))
                    .py(px(2.0))
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_secondary_button_hover)
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(c.text_default)
                    .child(*sc)
                    .into_any_element();

                sec1_items.push(make_row(name, desc, ctrl_sc, theme, inner_border_color));
            }
        }

        let sec1_shell = cx.entity().downgrade();
        sections.push(make_section(
            "pref-sec-doc-actions",
            "Document Actions",
            is_sec1_expanded,
            Box::new(move |_event, _window, cx| {
                let _ = sec1_shell.update(cx, |shell, cx| {
                    shell.panels.settings.toggle_section(sec1_key);
                    cx.notify();
                });
            }),
            sec1_items,
            theme,
            panel_id,
        ));

        // Section 2: Interface & View Controls
        let sec2_key = "view_controls";
        let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
        let mut sec2_items = Vec::new();

        if is_sec2_expanded {
            let view_shortcuts = [
                (
                    "Toggle View Mode",
                    "Switch between Edit, Preview, and Dual view layouts",
                    "Ctrl + M",
                ),
                (
                    "Toggle ExplorerState Tree",
                    "Show or collapse the left file navigation sidebar",
                    "Ctrl + E",
                ),
                (
                    "Quit Application",
                    "Safely exit application and save session",
                    "Ctrl + Q",
                ),
            ];

            for (name, desc, sc) in view_shortcuts.iter() {
                let ctrl_sc = div()
                    .px(px(8.0))
                    .py(px(2.0))
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_secondary_button_hover)
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(c.text_default)
                    .child(*sc)
                    .into_any_element();

                sec2_items.push(make_row(name, desc, ctrl_sc, theme, inner_border_color));
            }
        }

        let sec2_shell = cx.entity().downgrade();
        sections.push(make_section(
            "pref-sec-view-controls",
            "Interface & View Controls",
            is_sec2_expanded,
            Box::new(move |_event, _window, cx| {
                let _ = sec2_shell.update(cx, |shell, cx| {
                    shell.panels.settings.toggle_section(sec2_key);
                    cx.notify();
                });
            }),
            sec2_items,
            theme,
            panel_id,
        ));

        sections
    }
}
