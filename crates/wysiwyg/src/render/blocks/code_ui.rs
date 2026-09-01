//! Code block UI — editor section, toolbar, and language picker.

use ui::menu_item::menu_item;

use gpui::*;

use crate::render::BLOCK_EDITOR_CONTEXT;

use crate::model::block::Block;
use crate::render::inline::text_element::{BlockTextElement, CodeLanguageInputElement};
use config::language::I18nStrings;
use syntax_highlighter::language::{code_language_display_name, code_language_options_matching};
use theme::Theme;

impl Block {
    pub fn render_code_editor_section(
        &self,
        show_toolbar: bool,
        is_placeholder: bool,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;

        let current_language = self.code_language_text();
        let language_label: SharedString = if current_language.is_empty() {
            strings.code_language_placeholder.clone().into()
        } else {
            code_language_display_name(current_language)
                .to_string()
                .into()
        };

        let code_content_container = if self.show_code_line_numbers {
            let line_count = self.display_text().split('\n').count().max(1);
            let line_numbers_text = (1..=line_count)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            div()
                .w_full()
                .flex()
                .flex_row()
                .child(
                    div()
                        .flex_none()
                        .pr(px(10.0))
                        .mr(px(8.0))
                        .border_r_1()
                        .border_color(c.table_border)
                        .text_align(TextAlign::Right)
                        .text_size(px(t.code_size))
                        .line_height(rems(t.text_line_height))
                        .text_color(c.dialog_muted)
                        .child(SharedString::from(line_numbers_text)),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .flex_1()
                        .child(BlockTextElement::new(cx.entity(), is_placeholder)),
                )
        } else {
            div()
                .min_w(px(0.0))
                .w_full()
                .child(BlockTextElement::new(cx.entity(), is_placeholder))
        };

        let toolbar = self.render_code_toolbar(show_toolbar, language_label, theme, cx);
        let editor_section = div()
            .relative()
            .w_full()
            .px(px(d.code_block_padding_x))
            .py(px(d.code_block_padding_y))
            .text_size(px(t.code_size))
            .text_color(c.code_text)
            .line_height(rems(t.text_line_height))
            .child(code_content_container)
            .child(toolbar);

        if !self.code_toolbar.picker.is_open {
            editor_section.into_any_element()
        } else {
            let picker = self.render_code_language_picker(current_language, theme, strings, cx);
            editor_section.child(picker).into_any_element()
        }
    }

    pub fn render_code_toolbar(
        &self,
        show_toolbar: bool,
        language_label: SharedString,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let toolbar_height = 26.0;

        div()
            .id(ElementId::Name(
                format!("code-toolbar-{}", self.data.id).into(),
            ))
            .absolute()
            .top(px(4.0))
            .right(px(4.0))
            .opacity(if show_toolbar { 1.0 } else { 0.0 })
            .flex()
            .items_center()
            .gap(px(2.0))
            .p(px(2.0))
            .h(px(toolbar_height))
            .rounded(px(d.button_radius))
            .bg(c.code_language_input_bg)
            .border_1()
            .border_color(c.table_border)
            .text_size(px(12.0))
            .text_color(c.code_language_input_text)
            .child(
                div()
                    .id(ElementId::Name(
                        format!("code-language-picker-{}", self.data.id).into(),
                    ))
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .px(px(8.0))
                    .rounded(px(d.button_radius))
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.9))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(Self::on_code_language_picker_toggle),
                    )
                    .child(language_label)
                    .child(
                        svg()
                            .path("icons/editor/wysiwyg/codeblock/select-chevron.svg")
                            .size(px(12.0))
                            .text_color(c.dialog_muted),
                    ),
            )
            .child(div().w(px(1.0)).h(px(14.0)).bg(c.table_border))
            .child(
                div()
                    .id(ElementId::Name(
                        format!("code-line-numbers-{}", self.data.id).into(),
                    ))
                    .h_full()
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(d.icon_button_radius))
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.9))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(Self::on_code_line_numbers_toggle),
                    )
                    .child(
                        svg()
                            .path("icons/editor/wysiwyg/codeblock/line-numbers.svg")
                            .size(px(14.0))
                            .text_color(if self.show_code_line_numbers {
                                c.code_language_input_text
                            } else {
                                c.dialog_muted
                            }),
                    ),
            )
            .child(
                div()
                    .id(ElementId::Name(
                        format!("code-copy-{}", self.data.id).into(),
                    ))
                    .h_full()
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(d.icon_button_radius))
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.9))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(Self::on_code_copy_button_mouse_down),
                    )
                    .child(
                        svg()
                            .path("icons/editor/wysiwyg/codeblock/copy.svg")
                            .size(px(14.0))
                            .text_color(c.code_language_input_text),
                    ),
            )
            .into_any_element()
    }

    pub fn render_code_language_picker(
        &self,
        current_language: &str,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let toolbar_height = 26.0;
        let options = code_language_options_matching(&self.code_toolbar.picker.query);
        let selected_language = current_language.to_string();

        div()
            .id(ElementId::Name(
                format!("code-picker-container-{}", self.data.id).into(),
            ))
            .absolute()
            .top(px(toolbar_height + 8.0))
            .right(px(4.0))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .w(px(230.0))
            .max_h(px(320.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(6.0))
            .rounded(px(d.menu_panel_radius))
            .border_1()
            .border_color(c.dialog_border)
            .bg(c.dialog_surface)
            .shadow_lg()
            .child(
                div()
                    .key_context(BLOCK_EDITOR_CONTEXT)
                    .track_focus(&self.code_language_focus_handle)
                    .on_action(cx.listener(Self::on_code_language_newline))
                    .on_action(cx.listener(Self::on_code_language_dismiss))
                    .on_action(cx.listener(Self::on_code_language_delete_backward))
                    .on_action(cx.listener(Self::on_code_language_delete))
                    .on_action(cx.listener(Self::on_code_language_focus_content))
                    .on_action(cx.listener(Self::on_code_language_focus_next))
                    .on_action(cx.listener(Self::on_code_language_move_left))
                    .on_action(cx.listener(Self::on_code_language_move_right))
                    .on_action(cx.listener(Self::on_code_language_home))
                    .on_action(cx.listener(Self::on_code_language_end))
                    .on_action(cx.listener(Self::on_code_language_select_left))
                    .on_action(cx.listener(Self::on_code_language_select_right))
                    .on_action(cx.listener(Self::on_code_language_select_all))
                    .on_action(cx.listener(Self::on_code_language_copy))
                    .on_action(cx.listener(Self::on_code_language_cut))
                    .on_action(cx.listener(Self::on_code_language_paste))
                    .on_action(cx.listener(Self::on_code_language_indent))
                    .on_action(cx.listener(Self::on_code_language_outdent))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(Self::on_code_language_mouse_down),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(Self::on_code_language_mouse_up),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(Self::on_code_language_mouse_up_out),
                    )
                    .on_mouse_move(cx.listener(Self::on_code_language_mouse_move))
                    .w_full()
                    .h(px(28.0))
                    .px(px(8.0))
                    .py(px(3.0))
                    .rounded(px(d.code_language_input_radius))
                    .border_1()
                    .border_color(c.dialog_border)
                    .bg(c.dialog_secondary_button_bg)
                    .flex()
                    .items_center()
                    .text_size(px(11.5))
                    .cursor(CursorStyle::IBeam)
                    .child(CodeLanguageInputElement::new(
                        cx.entity(),
                        SharedString::from(strings.code_language_search_placeholder.clone()),
                    )),
            )
            .child(
                div()
                    .id(ElementId::Name(
                        format!("code-language-list-{}", self.data.id).into(),
                    ))
                    .track_scroll(&self.code_toolbar.picker.scroll_handle)
                    .w_full()
                    .max_h(px(250.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_y_scroll()
                    .scrollbar_width(px(4.0))
                    .children(options.into_iter().enumerate().map(|(index, option)| {
                        let option_block = cx.entity();
                        let value = option.value;
                        let is_selected =
                            code_language_display_name(&selected_language) == option.label;
                        menu_item(
                            ElementId::Name(
                                format!("code-language-option-{}-{index}", self.data.id).into(),
                            ),
                            c,
                            d,
                        )
                        .w_full()
                        .flex_shrink_0()
                        .justify_between()
                        .bg(if is_selected {
                            c.panel_row_selected
                        } else {
                            c.dialog_surface
                        })
                        .active(|this| this.opacity(0.9))
                        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            option_block.update(cx, |block, block_cx| {
                                block_cx.stop_propagation();
                                block.choose_code_language(value, block_cx);
                                block.focus_handle.focus(window, block_cx);
                            });
                        })
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(if is_selected {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if is_selected {
                                    c.text_default
                                } else {
                                    c.dialog_body
                                })
                                .child(option.label),
                        )
                        .children(if is_selected {
                            Some(
                                svg()
                                    .path("icons/editor/wysiwyg/codeblock/select-checkmark.svg")
                                    .size(px(14.0))
                                    .text_color(c.text_default),
                            )
                        } else {
                            None
                        })
                    })),
            )
            .into_any_element()
    }
}
