//! Editor view components — UI rendering, topbar, bottombar, scrollbar, and frame synchronization.

pub mod bottombar;
pub mod scrollbar;
pub mod sync;
pub mod topbar;
pub mod words;

use gpui::*;

use crate::editor::Editor;
use config::language::I18nManager;
use theme::ThemeManager;

pub const SPLITYPE_REPOSITORY_URL: &str = "https://github.com/hengvvang/splitype";
pub const SPLITYPE_BUG_REPORT_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=bug_report.yml";
pub const SPLITYPE_FEATURE_REQUEST_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=feature_request.yml";
pub const SPLITYPE_DISCUSSIONS_URL: &str = "https://github.com/hengvvang/splitype/discussions";
pub const SPLITYPE_WIKI_URL: &str = "https://github.com/hengvvang/splitype/wiki";
pub const SPLITYPE_RELEASES_URL: &str = "https://github.com/hengvvang/splitype/releases";

pub fn open_splitype_repository(cx: &mut App) {
    cx.open_url(SPLITYPE_REPOSITORY_URL);
}

pub fn open_bug_report(cx: &mut App) {
    cx.open_url(SPLITYPE_BUG_REPORT_URL);
}

pub fn open_feature_request(cx: &mut App) {
    cx.open_url(SPLITYPE_FEATURE_REQUEST_URL);
}

pub fn open_discussions(cx: &mut App) {
    cx.open_url(SPLITYPE_DISCUSSIONS_URL);
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.has_tabs() {
            let active_pane = self.active_pane_id();
            self.apply_pending_focus(active_pane, window, cx);
            self.apply_pending_autoscroll(active_pane, window, cx);
            self.sync_pending_save(window, cx);
            self.sync_pending_save_as(window, cx);
            self.sync_pending_open_link(window, cx);
            self.sync_window_edited_state(window);
        }

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        if self.has_tabs() {
            self.sync_window_title(window, &strings);
        }

        let follow_modifier_active = window.modifiers().secondary();
        let d = &theme.dimensions;
        let c = &theme.colors;
        let panel_id = self.panel_id;
        let leaf_count = self.leaf_count;
        let is_maximized = self.is_maximized;

        let base = div()
            .id(("editor-panel-tile", panel_id.0))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .rounded(px(d.panel_tile_radius))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .shadow_lg()
            .font(theme::TypographyStore::prose_font(cx))
            .on_modifiers_changed(move |event, window, _| {
                if event.modifiers.secondary() != follow_modifier_active {
                    window.refresh();
                }
            })
            .capture_key_down(cx.listener(Self::on_editor_key_down_capture))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_save_document))
            .on_action(cx.listener(Self::on_save_document_as))
            .on_action(cx.listener(Self::on_export_html))
            .on_action(cx.listener(Self::on_export_pdf))
            .on_action(cx.listener(Self::on_toggle_pane_kind))
            .on_action(cx.listener(Self::on_toggle_maximize_pane))
            .on_action(cx.listener(Self::on_toggle_search))
            .on_action(cx.listener(Self::on_toggle_replace))
            .on_action(cx.listener(Self::on_find_next))
            .on_action(cx.listener(Self::on_find_previous))
            .on_action(cx.listener(Self::on_replace_current))
            .on_action(cx.listener(Self::on_replace_all))
            .on_action(cx.listener(Self::on_page_up))
            .on_action(cx.listener(Self::on_page_down))
            .on_action(cx.listener(Self::on_jump_to_top))
            .on_action(cx.listener(Self::on_jump_to_bottom))
            .child(self.render_editor_topbar(
                crate::plugin::TOPBAR_ICON_PREFIX,
                &theme,
                leaf_count,
                is_maximized,
                cx,
            ))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h(px(0.0))
                    .relative()
                    .child(self.render_editor_pane_layout(&theme, &strings, window, cx)),
            )
            .child(self.render_editor_bottombar(&theme, &strings, cx));

        let base =
            if let Some(search_overlay) = self.render_search_panel_overlay(&theme, window, cx) {
                base.child(search_overlay)
            } else {
                base
            };

        base.into_any_element()
    }
}
