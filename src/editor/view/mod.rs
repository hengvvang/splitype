//! Editor window rendering — `Editor::render` and panel views.
//!
//! # Render phases
//! 1. **Pre-frame bookkeeping** — pending focus, scroll-into-view,
//!    save / save-as / open-link, window title sync.
//! 2. **Viewport & scroll** — scroll-handle bounds, scrollbar geometry.
//! 3. **Row collection** — iterate `document.blocks()`, group into callout /
//!    footnote / plain rows, compute inter-row gaps.
//! 4. **Windowing** — visible viewport plus overscan; top / bottom spacers.
//! 5. **Scroll content** — windowed rows in scroll container with listeners.
//! 6. **Overlays** — context menu, table-insert dialog, info/drop/unsaved
//!    dialogs.

pub(crate) mod context_menu;
pub(crate) mod context_menu_actions;
pub(crate) mod context_menu_render;
pub(crate) mod dialogs;
pub(crate) mod document_viewport;
pub(crate) mod export;
pub(crate) mod lifecycle_sync;

use gpui::*;

use crate::editor::controller::*;
use crate::editor::wysiwyg::render::layout::editor_text_font;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::ThemeManager;

pub(crate) const SPLITYPE_REPOSITORY_URL: &str = "https://github.com/hengvvang/splitype";
pub(crate) const SPLITYPE_BUG_REPORT_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=bug_report.yml";
pub(crate) const SPLITYPE_FEATURE_REQUEST_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=feature_request.yml";
pub(crate) const SPLITYPE_DISCUSSIONS_URL: &str =
    "https://github.com/hengvvang/splitype/discussions";
pub(crate) const SPLITYPE_WIKI_URL: &str = "https://github.com/hengvvang/splitype/wiki";
pub(crate) const SPLITYPE_RELEASES_URL: &str = "https://github.com/hengvvang/splitype/releases";

pub(crate) fn open_splitype_repository(cx: &mut App) {
    cx.open_url(SPLITYPE_REPOSITORY_URL);
}

pub(crate) fn open_bug_report(cx: &mut App) {
    cx.open_url(SPLITYPE_BUG_REPORT_URL);
}

pub(crate) fn open_feature_request(cx: &mut App) {
    cx.open_url(SPLITYPE_FEATURE_REQUEST_URL);
}

pub(crate) fn open_discussions(cx: &mut App) {
    cx.open_url(SPLITYPE_DISCUSSIONS_URL);
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // One Editor entity serves one area, so tab()/doc() always resolve
        // to this editor's own session.

        if self.has_active_tab() {
            // The window keyboard focus sits in exactly one pane; pending
            // focus and scroll-into-view apply to the active pane here (and
            // again inside its document view once layout is measurable).
            let active_pane = self.active_pane_id();
            self.apply_pending_focus(active_pane, window, cx);
            self.apply_pending_scroll_into_view(active_pane, window, cx);
            self.tab_mut().undo.last_selection_snapshot =
                self.capture_source_selection_snapshot(cx);
            self.sync_pending_save(window, cx);
            self.sync_pending_save_as(window, cx);
            self.sync_pending_open_link(window, cx);
            self.sync_window_edited_state(window);
        }

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        if self.has_active_tab() {
            self.sync_window_title(window, &strings);
        }

        let _editor = cx.entity().downgrade();

        // Repaint when the Cmd/Ctrl follow modifier toggles so a hovered link's
        // hand cursor updates without moving the pointer. `ModifiersChanged` is
        // dispatched along the focused element's path to the root, and this root
        // is an ancestor of every block, so one listener here covers a link in any
        // block while editing. Gated to the secondary modifier so Shift during
        // selection does not repaint.
        let follow_modifier_active = window.modifiers().secondary();

        let d = &theme.dimensions;
        let c = &theme.colors;
        let panel_id = self.panel_id;
        // Pushed by the Shell every frame: how many panels the outer layout
        // holds (maximize/close controls hide for a single area) and
        // whether this tile is maximized.
        let leaf_count = self.leaf_count;
        let is_maximized = self.is_maximized;

        // One Editor entity renders its own panel tile: top bar, panes
        // layout, and bottom status bar. The outer split tree (rendered by
        // the Shell) embeds this tile as one leaf.
        let base = div()
            .id(("editor-panel-tile", panel_id))
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
            .font(editor_text_font())
            .on_modifiers_changed(move |event, window, _| {
                if event.modifiers.secondary() != follow_modifier_active {
                    window.refresh();
                }
            })
            .capture_action(cx.listener(Self::on_copy_capture))
            .capture_action(cx.listener(Self::on_cut_capture))
            .capture_action(cx.listener(Self::on_delete_capture))
            .capture_action(cx.listener(Self::on_delete_backward_capture))
            .capture_key_down(cx.listener(Self::on_editor_key_down_capture))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_save_document))
            .on_action(cx.listener(Self::on_save_document_as))
            .on_action(cx.listener(Self::on_export_html))
            .on_action(cx.listener(Self::on_export_pdf))
            .on_action(cx.listener(Self::on_toggle_view_mode_action))
            .on_action(cx.listener(Self::on_page_up))
            .on_action(cx.listener(Self::on_page_down))
            .on_action(cx.listener(Self::on_jump_to_top))
            .on_action(cx.listener(Self::on_jump_to_bottom))
            .on_action(cx.listener(Self::on_dismiss_transient_ui))
            .child(self.render_editor_topbar(
                WindowPanelKind::Editor,
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
        let base = if let Some(context_menu) = self.render_context_menu_overlay(&theme, cx) {
            base.child(context_menu)
        } else {
            base
        };
        let base = if let Some(footnote_tooltip) = self.render_footnote_tooltip(&theme, window, cx)
        {
            base.child(footnote_tooltip)
        } else {
            base
        };
        let base = if let Some(table_dialog) = self.render_table_insert_dialog_overlay(&theme, cx) {
            base.child(table_dialog)
        } else {
            base
        };
        // Window-level dialogs (unsaved changes, drop-replace, Help-menu
        // info) render on the Shell at the window root.
        base.into_any_element()
    }
}
