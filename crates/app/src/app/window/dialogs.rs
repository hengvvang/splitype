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

use crate::app::shell::Shell;
use crate::editor::engine::controller::{Editor, InfoDialogKind};
use crate::editor::panes::document_pane::{SPLITYPE_RELEASES_URL, SPLITYPE_REPOSITORY_URL, SPLITYPE_WIKI_URL};
use i18n::{I18nManager, I18nStrings};
use net::update_checker::{
    self as update_check, UpdateCheckResult, UpdateVersionInfo,
};
use theme::Theme;
use ui::button::{compact_danger_button, compact_primary_button, compact_secondary_button};
use ui::dialog::dialog_card;
use ui::popover::overlay;

pub(crate) const ABOUT_EMOJIS: &[&str] = &[
    "icons/emoji/1.svg",
    "icons/emoji/2.svg",
    "icons/emoji/3.svg",
    "icons/emoji/4.svg",
    "icons/emoji/5.svg",
    "icons/emoji/6.svg",
    "icons/emoji/7.svg",
    "icons/emoji/8.svg",
    "icons/emoji/9.svg",
    "icons/emoji/10.svg",
    "icons/emoji/11.svg",
    "icons/emoji/12.svg",
    "icons/emoji/13.svg",
    "icons/emoji/14.svg",
    "icons/emoji/15.svg",
    "icons/emoji/16.svg",
    "icons/emoji/17.svg",
    "icons/emoji/18.svg",
];

impl Shell {
    /// The editor that holds any tab satisfying `show` (dialog routing).
    pub(crate) fn editor_with_dialog(
        &self,
        cx: &App,
        show: fn(&crate::editor::engine::controller::FileState) -> bool,

    ) -> Option<Entity<Editor>> {
        self.panel_contents
            .values()
            .find_map(|content| {
                let entity = content.as_editor()?;
                let editor = entity.read(cx);
                if editor.session.tabs().any(|tab| show(&tab.file)) {
                    Some(entity.clone())
                } else {
                    None
                }
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
        if self.unsaved_dialog.is_some()
            || self
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

    /// Cancel the unsaved-changes dialog without closing the window or panel.
    pub(crate) fn on_cancel_close_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.unsaved_dialog.take() {
            let target_editor = dialog
                .scope
                .panel_id()
                .and_then(|id| self.editor_for(id))
                .or_else(|| self.active_editor());
            if let Some(editor) = target_editor {
                editor.update(cx, |editor, cx| {
                    if let Some(restore) = dialog.restore_focus {
                        let pane = editor.active_pane_state();
                        if let Some(wysiwyg) = pane.as_wysiwyg_mut() {
                            wysiwyg.focus.active_entity = Some(restore);
                        }
                    }
                    cx.notify();
                });
            }
        }
        if let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog) {
            editor.update(cx, |editor, cx| editor.cancel_close_dialog(cx));
        }
        cx.notify();
    }

    /// Save the target document(s) and then close the window, panel, or tab depending on scope.
    pub(crate) fn on_save_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.unsaved_dialog.take() {
            match dialog.scope {
                crate::app::shell::UnsavedDialogScope::Window => {
                    for content in self.panel_contents.values() {
                        if let Some(editor) = content.as_editor() {
                            editor.update(cx, |ed, cx| {
                                ed.save_all_dirty_tabs(window, cx);
                            });
                        }
                    }
                    window.remove_window();
                }
                crate::app::shell::UnsavedDialogScope::EditorPanel(panel_id) => {
                    if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            ed.save_all_dirty_tabs(window, cx);
                        });
                    }
                    if self.layout_leaf_count() > 1 {
                        self.close_panel(panel_id, cx);
                    } else if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            ed.session.clear_tabs();
                            cx.notify();
                        });
                    }
                }
                crate::app::shell::UnsavedDialogScope::Tab { panel_id, index } => {
                    if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            ed.save_tab_at(index, window, cx);
                            ed.close_tab(index, cx);
                        });
                    }
                }
            }
            cx.notify();
            return;
        }

        if let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog) {
            editor.update(cx, |editor, cx| editor.save_and_close(window, cx));
        }
    }

    /// Discard unsaved changes and close the window, panel, or tab depending on scope.
    pub(crate) fn on_discard_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.unsaved_dialog.take() {
            match dialog.scope {
                crate::app::shell::UnsavedDialogScope::Window => {
                    for session in self.retained_editor_sessions.values_mut() {
                        for tab in session.tabs_mut() {
                            tab.file.dirty = false;
                        }
                    }
                    for content in self.panel_contents.values() {
                        if let Some(editor) = content.as_editor() {
                            editor.update(cx, |ed, cx| {
                                for tab in ed.session.tabs_mut() {
                                    tab.file.dirty = false;
                                }
                                cx.notify();
                            });
                        }
                    }
                    window.remove_window();
                }
                crate::app::shell::UnsavedDialogScope::EditorPanel(panel_id) => {
                    if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            for tab in ed.session.tabs_mut() {
                                tab.file.dirty = false;
                            }
                            cx.notify();
                        });
                    }
                    if self.layout_leaf_count() > 1 {
                        self.close_panel(panel_id, cx);
                    } else if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            ed.session.clear_tabs();
                            cx.notify();
                        });
                    }
                }
                crate::app::shell::UnsavedDialogScope::Tab { panel_id, index } => {
                    if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            if let Some(tab) = ed.session.tab_mut(index) {
                                tab.file.dirty = false;
                            }
                            ed.close_tab(index, cx);
                        });
                    }
                }
            }
            cx.notify();
            return;
        }

        if let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog) {
            editor.update(cx, |editor, cx| editor.discard_and_close(cx));
            if let Some((panel_id, index)) = self.first_dirty_tab(cx) {
                self.prompt_close_tab(panel_id, index, cx);
            } else {
                window.remove_window();
            }
        }
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

        let (title, message) = if let Some(dialog) = &self.unsaved_dialog {
            match &dialog.scope {
                crate::app::shell::UnsavedDialogScope::Window => (
                    strings.unsaved_changes_window_title.clone(),
                    strings.unsaved_changes_window_message.clone(),
                ),
                crate::app::shell::UnsavedDialogScope::EditorPanel(_) => (
                    strings.unsaved_changes_editor_title.clone(),
                    strings.unsaved_changes_editor_message.clone(),
                ),
                crate::app::shell::UnsavedDialogScope::Tab { .. } => (
                    strings.unsaved_changes_tab_title.clone(),
                    strings
                        .unsaved_changes_tab_message_template
                        .replace("{name}", &dialog.document_name),
                ),
            }
        } else {
            (
                strings.unsaved_changes_title.clone(),
                strings.unsaved_changes_message.clone(),
            )
        };

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
                            .w(px(400.0))
                            .border(px(1.0))
                            .border_color(c.dialog_border)
                            .rounded(px(d.dialog_radius))
                            .shadow_xl()
                            .p(px(20.0))
                            .gap(px(16.0))
                            .occlude()
                            .on_click(|_event, _window, _cx| {})
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(16.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(c.dialog_title)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .line_height(rems(1.4))
                                            .text_color(c.dialog_body)
                                            .child(message),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        compact_secondary_button("cancel-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_secondary_button_text)
                                            .child(strings.unsaved_changes_cancel.clone())
                                            .on_click(cx.listener(Self::on_cancel_close_dialog)),
                                    )
                                    .child(
                                        compact_danger_button("discard-and-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_danger_button_text)
                                            .child(strings.unsaved_changes_discard.clone())
                                            .on_click(cx.listener(Self::on_discard_and_close)),
                                    )
                                    .child(
                                        compact_primary_button("save-and-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.unsaved_changes_save.clone())
                                            .on_click(cx.listener(Self::on_save_and_close)),
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
        editor.update(cx, |editor, cx| editor.cancel_drop_replace_dialog(cx));
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
        editor.update(cx, |editor, cx| {
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
        editor.update(cx, |editor, cx| {
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
                            .rounded(px(d.dialog_radius))
                            .shadow_lg()
                            .occlude()
                            .on_click(|_event, _window, _cx| {})
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
        _cx: &Context<Self>,
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
                let link = |id: &'static str, label: String, url: &'static str| {
                    div()
                        .id(id)
                        .cursor_pointer()
                        .text_size(px(17.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c.text_link)
                        .hover(|this| this.underline())
                        .child(label)
                        .on_click(move |_event, _window, cx| cx.open_url(url))
                };

                div()
                    .relative()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(14.0))
                    .pt(px(4.0))
                    .pb(px(4.0))
                    // Section ①: Centered App Logo
                    .child(
                        div()
                            .w(px(96.0))
                            .h(px(96.0))
                            .bg(c.dialog_surface)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                svg()
                                    .path("identity/logo.svg")
                                    .size(px(72.0))
                                    .text_color(c.dialog_title),
                            ),
                    )
                    // Section ②: Splitype title & version badge
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .text_size(px(24.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(c.dialog_title)
                                    .child("Splitype"),
                            )
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(c.dialog_muted)
                                    .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                            ),
                    )
                    // Section ③: Slogan / Tagline (shifted downward)
                    .child(
                        div()
                            .mt(px(14.0))
                            .text_size(px(17.5))
                            .line_height(rems(1.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.dialog_body)
                            .text_align(TextAlign::Center)
                            .child(strings.about_tagline.clone()),
                    )
                    // Section ④: Link row (shifted downward)
                    .child(
                        div()
                            .mt(px(14.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(28.0))
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

        let is_about = kind == InfoDialogKind::About;
        let (dialog_width, dialog_min_height, dialog_padding) = if is_about {
            (px(560.0), px(380.0), px(28.0))
        } else {
            (px(d.dialog_width), px(0.0), px(20.0))
        };

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
                            .relative()
                            .overflow_hidden()
                            .w(dialog_width)
                            .min_h(dialog_min_height)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.dialog_radius))
                            .shadow_2xl()
                            .p(dialog_padding)
                            .occlude()
                            .on_click(|_event, _window, _cx| {})
                            // Background randomized emoji grid for About dialog: subtle soft watermark (0.08)
                            .children(if is_about {
                                Some(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .overflow_hidden()
                                        .flex()
                                        .flex_col()
                                        .opacity(0.08)
                                        .children((0..5).map(|row| {
                                            div()
                                                .flex()
                                                .w_full()
                                                .flex_1()
                                                .children((0..8).map(|col| {
                                                    let idx = self.about_bg_emojis
                                                        .get(row * 8 + col)
                                                        .copied()
                                                        .unwrap_or((row * 8 + col) % ABOUT_EMOJIS.len());
                                                    let path = ABOUT_EMOJIS[idx % ABOUT_EMOJIS.len()];
                                                    div()
                                                        .flex_1()
                                                        .h_full()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(
                                                            svg()
                                                                .path(path)
                                                                .size(px(52.0))
                                                                .text_color(c.text_default),
                                                        )
                                                }))
                                        })),
                                )
                            } else {
                                None
                            })
                            .child(
                                if is_about {
                                    div()
                                } else {
                                    div()
                                        .text_size(px(t.dialog_title_size))
                                        .font_weight(t.dialog_title_weight.to_font_weight())
                                        .text_color(c.dialog_title)
                                        .child(self.info_dialog_title(&strings, kind).to_string())
                                },
                            )
                            .child(self.render_info_dialog_body(theme, &strings, kind, cx))
                            // Top-right close 'x' button (solid c.dialog_title: pure black in light theme, pure white in dark theme)
                            .children(if is_about {
                                Some(
                                    div()
                                        .id("about-close-btn")
                                        .absolute()
                                        .top(px(14.0))
                                        .right(px(14.0))
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .on_click(cx.listener(Self::on_dismiss_info_dialog))
                                        .child(
                                            svg()
                                                .path("icons/editor/topbar/close.svg")
                                                .size(px(16.0))
                                                .text_color(c.dialog_title),
                                        ),
                                )
                            } else {
                                None
                            })
                            .child(
                                if is_about {
                                    div()
                                } else {
                                    div().flex().justify_end().child(
                                        compact_primary_button("dismiss-info-dialog", c, d)
                                            .h(px(26.0))
                                            .px(px(28.0))
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.info_dialog_ok.clone())
                                            .on_click(cx.listener(Self::on_dismiss_info_dialog)),
                                    )
                                },
                            ),
                    ),
            )
    }

    // ── Update check ────────────────────────────────────────────────────

    pub(crate) fn request_check_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_unsaved_dialog_open(cx) {
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

