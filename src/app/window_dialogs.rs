//! Window-level overlay dialogs owned by the Shell: the unsaved-changes /
//! drop-replace confirmations and the Help-menu info dialog, plus the
//! background update check.
//!
//! The dialog *state* stays on the documents it concerns (`FileState`
//! flags on the affected tab; `Shell::info_dialog` for the Help dialog),
//! but the dialogs themselves are window chrome: they float over every
//! area, centered in the window. Actions route back to the editor whose
//! tab is showing the dialog.

use gpui::*;

use futures::FutureExt;
use futures::channel::oneshot;

use crate::app::shell::{PanelContent, Shell};
use crate::editor::controller::{Editor, InfoDialogKind};
use crate::editor::view::{SPLITYPE_RELEASES_URL, SPLITYPE_REPOSITORY_URL, SPLITYPE_WIKI_URL};
use crate::infra::i18n::{I18nManager, I18nStrings};
use crate::infra::net::update_checker::{
    self as update_check, UpdateCheckResult, UpdateVersionInfo,
};
use crate::infra::theme::Theme;
use crate::ui::button::{compact_danger_button, compact_primary_button, compact_secondary_button};
use crate::ui::dialog::dialog_card;
use crate::ui::popover::overlay;

impl Shell {
    /// The editor whose active tab satisfies `show` (dialog routing).
    pub(crate) fn editor_with_dialog(
        &self,
        cx: &App,
        show: fn(&crate::editor::controller::FileState) -> bool,
    ) -> Option<Entity<Editor>> {
        self.panel_contents.values().find_map(|content| match content {
            PanelContent::Editor(entity) => entity
                .read(cx)
                .active_editor_tab()
                .filter(|tab| show(&tab.file))
                .map(|_| entity.clone()),
        })
    }

    /// The window-level dialog to render this frame, if any: the info
    /// dialog wins over the per-document confirmations.
    pub(crate) fn render_window_dialogs(
        &self,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        if let Some(kind) = self.info_dialog {
            return Some(
                self.render_info_dialog_overlay(theme, kind, cx)
                    .into_any_element(),
            );
        }
        if self
            .editor_with_dialog(cx, |file| file.show_drop_replace_dialog)
            .is_some()
        {
            return Some(
                self.render_drop_replace_overlay(theme, cx)
                    .into_any_element(),
            );
        }
        if self
            .editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog)
            .is_some()
        {
            return Some(
                self.render_unsaved_changes_overlay(theme, cx)
                    .into_any_element(),
            );
        }
        None
    }

    // ── Unsaved-changes confirmation ────────────────────────────────────

    /// Cancel the unsaved-changes dialog without closing the window.
    pub(crate) fn on_cancel_close_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog)
        else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| editor.cancel_close_dialog(cx));
    }

    /// Save the current document and then close the window.
    pub(crate) fn on_save_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog)
        else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| editor.save_and_close(window, cx));
    }

    /// Discard unsaved changes and close the window immediately (routed
    /// from the Shell's dialog overlay).
    pub(crate) fn on_discard_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog)
        else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| editor.discard_and_close(cx));
        window.remove_window();
    }

    pub(crate) fn render_unsaved_changes_overlay(
        &self,
        theme: &Theme,
        cx: &Context<Self>,
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

    // ── Drop-replace confirmation ───────────────────────────────────────

    pub(crate) fn on_cancel_drop_replace_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor_with_dialog(cx, |file| file.show_drop_replace_dialog) else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| editor.cancel_drop_replace_dialog(cx));
    }

    pub(crate) fn on_discard_and_replace_drop(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor_with_dialog(cx, |file| file.show_drop_replace_dialog) else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| {
            editor.discard_pending_drop_replace(window, cx)
        });
    }

    pub(crate) fn on_save_and_replace_drop(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor_with_dialog(cx, |file| file.show_drop_replace_dialog) else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| {
            editor.save_and_replace_pending_drop(window, cx)
        });
    }

    /// Builds the dropped-file replacement dialog shown when the current
    /// document has unsaved changes.
    pub(crate) fn render_drop_replace_overlay(
        &self,
        theme: &Theme,
        cx: &Context<Self>,
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

    // ── Help-menu info dialog ───────────────────────────────────────────

    pub(crate) fn on_dismiss_info_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hide_info_dialog(cx);
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
                                    img(ImageSource::Resource(Resource::Embedded(
                                        "identity/logo.png".into(),
                                    )))
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

    pub(crate) fn render_info_dialog_overlay(
        &self,
        theme: &Theme,
        kind: InfoDialogKind,
        cx: &Context<Self>,
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
                            .rounded(px(d.panel_tile_radius))
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

    // ── Update check ────────────────────────────────────────────────────

    pub(crate) fn request_check_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.unsaved_dialog_open(cx) {
            return;
        }
        if self.update_check_in_progress {
            self.show_info_dialog(InfoDialogKind::CheckForUpdates, cx);
            return;
        }

        self.update_check_in_progress = true;
        self.show_info_dialog(InfoDialogKind::CheckForUpdates, cx);

        let weak_shell = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let (tx, rx) = oneshot::channel();
        std::thread::spawn(move || {
            let result = update_check::check_latest_version(env!("CARGO_PKG_VERSION"));
            let _ = tx.send(result);
        });

        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = rx
                .map(|result| {
                    result.unwrap_or_else(|_| {
                        Err(update_check::UpdateCheckError::ParseVersion(
                            "update check worker ended before returning a result".to_string(),
                        ))
                    })
                })
                .await;

            let _ = weak_shell.update(cx, |shell, cx| {
                shell.update_check_in_progress = false;
                shell.hide_info_dialog(cx);
            });

            let _ = cx.update_window(
                window_handle,
                move |_view: AnyView, window: &mut Window, cx: &mut App| match result {
                    Ok(UpdateCheckResult::UpdateAvailable(info)) => {
                        show_update_available_prompt(window, cx, &info);
                    }
                    Ok(UpdateCheckResult::UpToDate(info)) => {
                        show_up_to_date_prompt(window, cx, &info);
                    }
                    Err(error) => {
                        show_update_failed_prompt(window, cx, &error.to_string());
                    }
                },
            );
        })
        .detach();
    }
}

fn show_update_available_prompt(window: &mut Window, cx: &mut App, info: &UpdateVersionInfo) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let detail = format_update_message(
        &strings.update_available_message_template,
        &info.current_version,
        &info.latest_version,
    );
    let buttons = [
        strings.update_open_release.as_str(),
        strings.update_later.as_str(),
    ];
    let prompt = window.prompt(
        PromptLevel::Info,
        &strings.update_available_title,
        Some(&detail),
        &buttons,
        cx,
    );
    let window_handle = window.window_handle();
    cx.spawn(async move |cx| {
        let Ok(choice) = prompt.await else {
            return;
        };
        if choice == 0 {
            let _ = cx.update_window(window_handle, |_view: AnyView, _window, cx| {
                cx.open_url(update_check::RELEASES_URL);
            });
        }
    })
    .detach();
}

fn show_up_to_date_prompt(window: &mut Window, cx: &mut App, info: &UpdateVersionInfo) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let detail = format_update_message(
        &strings.update_up_to_date_message_template,
        &info.current_version,
        &info.latest_version,
    );
    let buttons = [strings.info_dialog_ok.as_str()];
    let _ = window.prompt(
        PromptLevel::Info,
        &strings.update_up_to_date_title,
        Some(&detail),
        &buttons,
        cx,
    );
}

fn show_update_failed_prompt(window: &mut Window, cx: &mut App, detail: &str) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let message = strings
        .update_failed_message_template
        .replace("{error}", detail);
    let buttons = [strings.info_dialog_ok.as_str()];
    let _ = window.prompt(
        PromptLevel::Critical,
        &strings.update_failed_title,
        Some(&message),
        &buttons,
        cx,
    );
}

fn format_update_message(template: &str, current_version: &str, latest_version: &str) -> String {
    template
        .replace("{current}", current_version)
        .replace("{latest}", latest_version)
}

#[cfg(test)]
mod tests {
    use super::format_update_message;

    #[test]
    pub(crate) fn update_message_templates_replace_versions() {
        assert_eq!(
            format_update_message("Current {current}, latest {latest}.", "0.2.1", "0.2.2"),
            "Current 0.2.1, latest 0.2.2."
        );
    }
}
