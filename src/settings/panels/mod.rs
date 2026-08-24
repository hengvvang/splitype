//! Settings panel rendered inside the editor's tiled layout.

pub(crate) mod editing;
pub(crate) mod interface;
pub(crate) mod shortcuts;

use gpui::*;

use crate::app::shell::Shell;
use crate::app::window_panels::PanelId;
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;
use crate::settings::state::SettingsTab;
use crate::ui::tab::nav_tab;

impl Shell {
    pub(crate) fn render_settings_body(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        _strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let active_tab = self.panels.settings.tab;

        // --- Left Sidebar (3 Main Tabs: Interface, Editing, Keymap) ---
        let mut left_nav_items = Vec::new();
        for (tab_idx, tab) in SettingsTab::all().iter().enumerate() {
            let is_active = active_tab == *tab;
            let shell = cx.entity().downgrade();
            let tab_item = *tab;

            left_nav_items.push(
                nav_tab(
                    ElementId::Name(format!("pref-tab-{panel_id}-{tab_idx}").into()),
                    c,
                    d,
                )
                .id(ElementId::Name(
                    format!("pref-tab-{panel_id}-{tab_idx}").into(),
                ))
                .cursor_pointer()
                .flex()
                .items_center()
                .bg(if is_active {
                    c.panel_row_selected
                } else {
                    c.dialog_surface
                })
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(if is_active {
                            gpui::FontWeight::BOLD
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .text_color(if is_active {
                            c.text_default
                        } else {
                            c.dialog_muted
                        })
                        .child(tab.name()),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = shell.update(cx, |shell, cx| {
                        shell.panels.settings.tab = tab_item;
                        cx.notify();
                    });
                })
                .into_any_element(),
            );
        }

        let left_nav = div()
            .w(px(160.0))
            .h_full()
            .flex_shrink_0()
            .p(px(8.0))
            .border_r_1()
            .border_color(c.dialog_border)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(left_nav_items);

        // --- Right Content Area ---
        let sections = match active_tab {
            SettingsTab::Interface => self.render_panel_interface_tab(panel_id, theme, cx),
            SettingsTab::Editing => self.render_panel_editing_tab(panel_id, theme, cx),
            SettingsTab::Keymap => self.render_panel_shortcuts_tab(panel_id, theme, cx),
        };

        let right_content = div()
            .id(("pref-right-content", panel_id.0))
            .relative()
            .flex_1()
            .h_full()
            .p(px(14.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(sections);

        // --- Main Root Layout ---
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_row()
            .bg(c.editor_background)
            .child(left_nav)
            .child(right_content)
            .into_any_element()
    }

    pub(crate) fn toggle_settings_section_handler(
        &self,
        cx: &mut Context<Self>,
        key: &'static str,
    ) -> crate::settings::common::SettingsClickHandler {
        let handle = cx.entity().downgrade();
        Box::new(move |_event, _window, cx| {
            let _ = handle.update(cx, |this, cx| {
                if this.panels.settings.expanded_sections.contains(key) {
                    this.panels.settings.expanded_sections.remove(key);
                } else {
                    this.panels.settings.expanded_sections.insert(key.to_string());
                }
                cx.notify();
            });
        })
    }
}
