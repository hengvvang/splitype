//! Window-level overlay dialogs owned by the Shell: the unsaved-changes /
//! drop-replace confirmations and the Help-menu info dialog, plus the
//! background update check.

pub mod about;
pub mod drop_replace;
pub mod unsaved;
pub mod update;

use gpui::*;

use crate::shell::Shell;
use config::language::I18nStrings;
use editor::{Editor, FileState};
use theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfoDialogKind {
    CheckForUpdates,
    About,
}

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
    pub(crate) fn editor_with_dialog(
        &self,
        cx: &App,
        show: fn(&FileState) -> bool,
    ) -> Option<Entity<Editor>> {
        self.panel_views.values().find_map(|view| {
            let panel = view.as_any().downcast_ref::<editor::EditorPanelView>()?;
            let editor = panel.editor.read(cx);
            if editor.session.tabs().any(|tab| show(&tab.file)) {
                Some(panel.editor.clone())
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
            InfoDialogKind::About => self.render_about_dialog_body(theme, strings),
        }
    }
}
