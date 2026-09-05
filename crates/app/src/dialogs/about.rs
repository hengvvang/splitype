//! About dialog overlay and background watermark rendering.

use gpui::*;

use super::{ABOUT_EMOJIS, InfoDialogKind};
use crate::links::{RELEASES_URL, REPOSITORY_URL, WIKI_URL};
use crate::shell::Shell;
use config::language::{I18nManager, I18nStrings};
use theme::Theme;
use ui::button::compact_primary_button;
use ui::dialog::dialog_card;
use ui::popover::overlay;

impl Shell {
    pub(crate) fn about_dialog_body_lines(strings: &I18nStrings) -> Vec<String> {
        vec![
            format!("Splitype {}", env!("CARGO_PKG_VERSION")),
            strings.help_about_message.clone(),
            format!("{}: {}", strings.help_about_github_label, REPOSITORY_URL),
            strings.help_about_star_message.clone(),
        ]
    }

    pub(crate) fn render_about_dialog_body(
        &self,
        theme: &Theme,
        strings: &I18nStrings,
    ) -> AnyElement {
        let c = &theme.colors;
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
            // Section ③: Slogan / Tagline
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
            // Section ④: Link row
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
                        REPOSITORY_URL,
                    ))
                    .child(link(
                        "about-releases-link",
                        strings.about_releases_label.clone(),
                        RELEASES_URL,
                    ))
                    .child(link(
                        "about-website-link",
                        strings.about_website_label.clone(),
                        REPOSITORY_URL,
                    ))
                    .child(link(
                        "about-wiki-link",
                        strings.about_wiki_label.clone(),
                        WIKI_URL,
                    )),
            )
            .into_any_element()
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
                                            div().flex().w_full().flex_1().children((0..8).map(
                                                |col| {
                                                    let idx = self
                                                        .about_bg_emojis
                                                        .get(row * 8 + col)
                                                        .copied()
                                                        .unwrap_or(
                                                            (row * 8 + col) % ABOUT_EMOJIS.len(),
                                                        );
                                                    let path =
                                                        ABOUT_EMOJIS[idx % ABOUT_EMOJIS.len()];
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
                                                },
                                            ))
                                        })),
                                )
                            } else {
                                None
                            })
                            .child(if is_about {
                                div()
                            } else {
                                div()
                                    .text_size(px(t.dialog_title_size))
                                    .font_weight(t.dialog_title_weight.to_font_weight())
                                    .text_color(c.dialog_title)
                                    .child(self.info_dialog_title(&strings, kind).to_string())
                            })
                            .child(self.render_info_dialog_body(theme, &strings, kind, cx))
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
                                                .path("icons/titlebar/chrome/close.svg")
                                                .size(px(16.0))
                                                .text_color(c.dialog_title),
                                        ),
                                )
                            } else {
                                None
                            })
                            .child(if is_about {
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
                            }),
                    ),
            )
    }
}
