//! In-window overlay dialogs and their action handlers: unsaved changes,
//! drop-replace, and info/about overlays.

use crate::ui::popover::overlay;

use crate::ui::dialog::dialog_card;

use crate::ui::button::{compact_danger_button, compact_primary_button, compact_secondary_button};

use gpui::*;


use crate::editor::controller::{Editor, InfoDialogKind};
use crate::editor::window::context_menu::TableInsertTarget;
use crate::editor::window::{SPLITYPE_RELEASES_URL, SPLITYPE_REPOSITORY_URL, SPLITYPE_WIKI_URL};
use crate::infra::i18n::{I18nManager, I18nStrings};
use crate::infra::theme::Theme;

/// State for the table insertion dialog opened from the context menu.
pub(crate) struct TableInsertDialogState {
    pub target: TableInsertTarget,
    pub body_rows: usize,
    pub columns: usize,
}

impl Editor {
    /// Dismiss the unsaved-changes dialog without closing the window.
    pub(crate) fn on_cancel_close_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab_mut().file.show_unsaved_changes_dialog = false;
        self.tab_mut().file.pending_close_after_save = false;
        if let Some(restore) = self.tab_mut().file.close_dialog_restore_focus.take() {
            self.tab_mut().focus.active_entity = Some(restore);
        }
        cx.notify();
    }

    /// Save the current document and then close the window.
    pub(crate) fn on_save_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab_mut().file.show_unsaved_changes_dialog = false;
        self.tab_mut().file.pending_close_after_save = true;
        self.save_document(window, cx);
    }

    /// Discard unsaved changes and close the window immediately.
    pub(crate) fn on_discard_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.tab_mut().file.show_unsaved_changes_dialog = false;
        self.tab_mut().file.pending_close_after_save = false;
        self.tab_mut().file.close_dialog_restore_focus = None;
        window.remove_window();
    }

    /// Initiate window-close flow, showing the unsaved-changes prompt when
    /// the document is dirty.
    pub(crate) fn request_close_current_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // No dirty document in this editor: nothing to save, close
        // immediately. (Window-wide aggregation across every editor area
        // moves to the Shell together with the window-lifecycle flow.)
        let Some((_, index)) = self.first_dirty_tab() else {
            window.remove_window();
            return;
        };
        self.panels.layout.activate_area(self.area_id);
        let tab = &mut self.session.tab_list.tabs[index];
        tab.file.show_unsaved_changes_dialog = true;
        tab.file.close_dialog_restore_focus = tab.focus.active_entity;
        cx.notify();
    }

    /// Called by the GPUI `Window::on_window_should_close` guard.
    /// Returns `true` when the window is safe to close (clean document).
    /// Returns `false` and shows the unsaved-changes prompt when dirty.
    pub(crate) fn on_window_should_close(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // No dirty document in this editor: safe to close. (Window-wide
        // aggregation across every editor area moves to the Shell together
        // with the window-lifecycle flow.)
        let Some((_, index)) = self.first_dirty_tab() else {
            return true;
        };
        self.panels.layout.activate_area(self.area_id);
        let tab = &mut self.session.tab_list.tabs[index];
        tab.file.show_unsaved_changes_dialog = true;
        tab.file.close_dialog_restore_focus = tab.focus.active_entity;
        cx.notify();
        false
    }

    /// Cancel the pending-close-after-save flag (called when save fails or is
    /// cancelled, or when the save completes but close is no longer desired).
    pub(crate) fn abort_pending_close_after_save(&mut self, cx: &mut Context<Self>) {
        self.tab_mut().file.pending_close_after_save = false;
        self.tab_mut().file.close_dialog_restore_focus = None;
        cx.notify();
    }
    pub(crate) fn render_unsaved_changes_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings();

        overlay()
            .id("unsaved-changes-overlay")
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(Self::on_cancel_close_dialog))
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        dialog_card(c, d)
                            .id("unsaved-changes-dialog")
                            .w(px(d.dialog_width))
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.menu_panel_radius))
                            .shadow_lg()
                            .occlude()
                            .on_click(|_, _, _| {})
                            .child(
                                div()
                                    .text_size(px(t.dialog_title_size))
                                    .font_weight(t.dialog_title_weight.to_font_weight())
                                    .text_color(c.dialog_title)
                                    .child(strings.unsaved_changes_title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(t.dialog_body_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .line_height(rems(t.text_line_height))
                                    .text_color(c.dialog_body)
                                    .child(strings.unsaved_changes_message.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(d.dialog_button_gap))
                                    .child(
                                        compact_primary_button("save-and-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.unsaved_changes_save_and_close.clone())
                                            .on_click(cx.listener(Self::on_save_and_close)),
                                    )
                                    .child(
                                        compact_danger_button("discard-and-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_danger_button_text)
                                            .child(
                                                strings.unsaved_changes_discard_and_close.clone(),
                                            )
                                            .on_click(cx.listener(Self::on_discard_and_close)),
                                    )
                                    .child(
                                        compact_secondary_button("cancel-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_secondary_button_text)
                                            .child(strings.unsaved_changes_cancel.clone())
                                            .on_click(cx.listener(Self::on_cancel_close_dialog)),
                                    ),
                            ),
                    ),
            )
    }

    /// Builds the dropped-file replacement dialog shown when the current
    /// document has unsaved changes.
    pub(crate) fn render_drop_replace_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings();

        overlay()
            .id("drop-replace-overlay")
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(Self::on_cancel_drop_replace_dialog))
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        dialog_card(c, d)
                            .id("drop-replace-dialog")
                            .w(px(d.dialog_width))
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.menu_panel_radius))
                            .shadow_lg()
                            .occlude()
                            .on_click(|_, _, _| {})
                            .child(
                                div()
                                    .text_size(px(t.dialog_title_size))
                                    .font_weight(t.dialog_title_weight.to_font_weight())
                                    .text_color(c.dialog_title)
                                    .child(strings.drop_replace_title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(t.dialog_body_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .line_height(rems(t.text_line_height))
                                    .text_color(c.dialog_body)
                                    .child(strings.drop_replace_message.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(d.dialog_button_gap))
                                    .child(
                                        compact_primary_button(
                                            "save-and-replace-drop-dialog",
                                            c,
                                            d,
                                        )
                                        .text_size(px(13.0))
                                        .font_weight(t.dialog_button_weight.to_font_weight())
                                        .text_color(c.dialog_primary_button_text)
                                        .child(strings.drop_replace_save_and_replace.clone())
                                        .on_click(cx.listener(Self::on_save_and_replace_drop)),
                                    )
                                    .child(
                                        div()
                                            .id("discard-and-replace-drop-dialog")
                                            .h(px(32.0))
                                            .px(px(14.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(d.menu_item_radius))
                                            .border(px(d.dialog_border_width))
                                            .border_color(c.dialog_border)
                                            .bg(c.dialog_danger_button_bg)
                                            .hover(|this| this.bg(c.dialog_danger_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_danger_button_text)
                                            .child(strings.drop_replace_discard_and_replace.clone())
                                            .on_click(
                                                cx.listener(Self::on_discard_and_replace_drop),
                                            ),
                                    )
                                    .child(
                                        compact_secondary_button(
                                            "cancel-drop-replace-dialog",
                                            c,
                                            d,
                                        )
                                        .text_size(px(13.0))
                                        .font_weight(t.dialog_button_weight.to_font_weight())
                                        .text_color(c.dialog_secondary_button_text)
                                        .child(strings.drop_replace_cancel.clone())
                                        .on_click(cx.listener(Self::on_cancel_drop_replace_dialog)),
                                    ),
                            ),
                    ),
            )
    }

    pub(crate) fn info_dialog_title<'a>(
        &self,
        strings: &'a I18nStrings,
        kind: InfoDialogKind,
    ) -> &'a str {
        match kind {
            InfoDialogKind::CheckForUpdates => &strings.help_check_updates_title,
            InfoDialogKind::About => &strings.help_about_title,
        }
    }

    pub(crate) fn about_dialog_body_lines(strings: &I18nStrings) -> Vec<String> {
        vec![
            format!("Splitype {}", env!("CARGO_PKG_VERSION")),
            strings.help_about_message.clone(),
            format!(
                "{}: {}",
                strings.help_about_github_label, SPLITYPE_REPOSITORY_URL
            ),
            strings.help_about_star_message.clone(),
        ]
    }

    pub(crate) fn info_dialog_body(&self, strings: &I18nStrings, kind: InfoDialogKind) -> String {
        match kind {
            InfoDialogKind::CheckForUpdates => strings.help_check_updates_message.clone(),
            InfoDialogKind::About => Self::about_dialog_body_lines(strings).join("\n"),
        }
    }

    pub(crate) fn render_info_dialog_body(
        &self,
        theme: &Theme,
        strings: &I18nStrings,
        kind: InfoDialogKind,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let body_style = |this: Div| {
            this.text_size(px(t.dialog_body_size))
                .font_weight(t.dialog_body_weight.to_font_weight())
                .line_height(rems(t.text_line_height))
                .text_color(c.dialog_body)
        };

        match kind {
            InfoDialogKind::CheckForUpdates => div()
                .flex()
                .flex_col()
                .gap(px(d.dialog_gap * 0.5))
                .child(
                    body_style(div()).children(
                        self.info_dialog_body(strings, kind)
                            .lines()
                            .map(|line| div().child(line.to_string())),
                    ),
                )
                .into_any_element(),
            InfoDialogKind::About => {
                // About panel — three stacked sections:
                //   section ① top: logo rect (left) beside the name rect
                //                   (right); the name rect splits into the
                //                   title row (Splitype v0.0.1) above the
                //                   tagline row
                //   section ②      link row (GitHub / Releases / Website /
                //                   Wiki)
                //   section ③      the dismiss button lives in the shared
                //                   dialog footer below this body
                let meta_line = |this: Div| {
                    this.text_size(px(t.dialog_body_size + 2.0))
                        .font_weight(t.dialog_body_weight.to_font_weight())
                        .line_height(rems(t.text_line_height))
                        .text_color(c.dialog_body)
                };
                let link = |id: &'static str, label: String, url: &'static str| {
                    div()
                        .id(id)
                        .cursor_pointer()
                        .text_color(c.text_link)
                        .underline()
                        .child(label)
                        .on_click(move |_, _, cx| cx.open_url(url))
                };

                div()
                    .flex()
                    .flex_col()
                    .gap(px(d.dialog_gap * 1.5))
                    .child(
                        // Section ①: horizontal top rect — logo rect on the
                        // left, name rect on the right.
                        div()
                            .flex()
                            .items_center()
                            .gap(px(28.0))
                            .child(
                                // Rect ①-left: the logo container. The PNG
                                // keeps the true white-on-black look; the SVG
                                // source renders as a monochrome mask and
                                // loses the line art.
                                div().child(
                                    img("identity/logo.png")
                                        .w(px(56.0))
                                        .h(px(64.0))
                                        .object_fit(ObjectFit::Contain),
                                ),
                            )
                            .child(
                                // Rect ①-right: the name rect, split into
                                // the title row above the tagline row.
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        // Rect ①-right-top: title row
                                        // (Splitype + inline grey version).
                                        div()
                                            .flex()
                                            .items_baseline()
                                            .gap(px(6.0))
                                            .child(
                                                div()
                                                    .text_size(px(t.dialog_title_size - 2.0))
                                                    .font_weight(
                                                        t.dialog_title_weight.to_font_weight(),
                                                    )
                                                    .text_color(c.dialog_title)
                                                    .child("Splitype"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(t.dialog_body_size))
                                                    .font_weight(
                                                        t.dialog_body_weight.to_font_weight(),
                                                    )
                                                    .text_color(c.dialog_muted)
                                                    .child(format!(
                                                        "v{}",
                                                        env!("CARGO_PKG_VERSION")
                                                    )),
                                            ),
                                    )
                                    .child(
                                        // Rect ①-right-bottom: tagline row.
                                        div()
                                            .text_size(px(t.dialog_body_size))
                                            .font_weight(t.dialog_body_weight.to_font_weight())
                                            .text_color(c.dialog_muted)
                                            .child(strings.about_tagline.clone()),
                                    ),
                            ),
                    )
                    .child(
                        // Section ②: the link row, left-aligned.
                        meta_line(div())
                            .flex()
                            .gap(px(16.0))
                            .child(link(
                                "about-github-link",
                                strings.help_about_github_label.clone(),
                                SPLITYPE_REPOSITORY_URL,
                            ))
                            .child(link(
                                "about-releases-link",
                                strings.about_releases_label.clone(),
                                SPLITYPE_RELEASES_URL,
                            ))
                            .child(link(
                                "about-website-link",
                                strings.about_website_label.clone(),
                                SPLITYPE_REPOSITORY_URL,
                            ))
                            .child(link(
                                "about-wiki-link",
                                strings.about_wiki_label.clone(),
                                SPLITYPE_WIKI_URL,
                            )),
                    )
                    .into_any_element()
            }
        }
    }

    pub(crate) fn on_dismiss_info_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hide_info_dialog(cx);
    }

    pub(crate) fn render_info_dialog_overlay(
        &self,
        theme: &Theme,
        kind: InfoDialogKind,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings().clone();

        overlay()
            .id("info-dialog-overlay")
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(Self::on_dismiss_info_dialog))
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        dialog_card(c, d)
                            .id("info-dialog")
                            .w(px(d.dialog_width))
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.area_tile_radius))
                            .shadow_lg()
                            .occlude()
                            .on_click(|_, _, _| {})
                            .child(
                                // The About panel has no title bar; other
                                // info dialogs keep one.
                                if kind == InfoDialogKind::About {
                                    div()
                                } else {
                                    div()
                                        .text_size(px(t.dialog_title_size))
                                        .font_weight(t.dialog_title_weight.to_font_weight())
                                        .text_color(c.dialog_title)
                                        .child(self.info_dialog_title(&strings, kind).to_string())
                                },
                            )
                            .child(self.render_info_dialog_body(theme, &strings, kind))
                            .child(
                                div().flex().justify_end().child(
                                    compact_primary_button("dismiss-info-dialog", c, d)
                                        .h(px(26.0))
                                        .px(px(28.0))
                                        .text_size(px(13.0))
                                        .font_weight(t.dialog_button_weight.to_font_weight())
                                        .text_color(c.dialog_primary_button_text)
                                        .child(strings.info_dialog_ok.clone())
                                        .on_click(cx.listener(Self::on_dismiss_info_dialog)),
                                ),
                            ),
                    ),
            )
    }
}
