//! Rendering for [`Block`] via GPUI's high-level [`Render`] trait.
//!
//! Each block kind produces a distinct visual style: H1 has a bottom border,
//! list items render a marker column (bullet / ordinal), and raw Markdown
//! fallback renders as plain text. This is the WYSIWYG editing presentation;
//! the read-only preview presentation lives in
//! `crate::editor_model::panes::preview::render`.

pub mod blockquote;
pub mod callout;
pub mod code_ui;
pub mod embedded_preview;
pub mod fenced_code;
pub mod footnote;
pub mod graphic_state;
pub mod heading;
pub mod html_block;
pub mod html_document;
pub mod inline;
pub mod inline_visuals;
pub mod latex_math;
pub mod layout;
pub mod link_cursor;
pub mod list_item;
pub mod list_markers;
pub mod media_placeholder;
pub mod mermaid_diagram;
pub mod paragraph;
pub mod raw_markdown;
pub mod table_block;
pub mod thematic_break;

pub use link_cursor::*;
pub use list_markers::*;
pub use media_placeholder::*;

use gpui::*;

pub const BLOCK_EDITOR_CONTEXT: &str = "BlockEditor";

use crate::document::block::Block;
use crate::render::inline::text_element::BlockTextElement;
use crate::render::{
    blockquote::render_blockquote,
    callout::render_callout,
    embedded_preview::render_graphic_preview_box,
    fenced_code::render_fenced_code,
    footnote::render_footnote_definition,
    heading::render_heading,
    html_block::render_html_block,
    latex_math::render_latex_math,
    list_item::{render_bulleted_list_item, render_numbered_list_item, render_task_list_item},
    mermaid_diagram::render_mermaid_diagram,
    paragraph::render_paragraph,
    raw_markdown::render_raw_markdown,
    table_block::render_table,
    thematic_break::{render_thematic_break_focused, render_thematic_break_unfocused},
};
use config::language::I18nManager;
use theme::{Theme, ThemeDimensions, ThemeManager};
use crate::markdown::parse::BlockKind;

fn wrap_with_quote_guides(content: AnyElement, quote_depth: usize, theme: &Theme) -> AnyElement {
    if quote_depth == 0 {
        return content;
    }

    let c = &theme.colors;
    let d = &theme.dimensions;
    let guide_offset = d.quote_padding_left;
    let total_padding = guide_offset * quote_depth as f32;

    div()
        .w_full()
        .relative()
        .pl(px(total_padding))
        .child(content)
        .children((0..quote_depth).map(|level| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(d.block_padding_x + guide_offset * level as f32))
                .w(px(d.quote_border_width))
                .bg(c.border_quote)
        }))
        .into_any_element()
}

pub fn visible_quote_guides(block: &Block) -> usize {
    if block.kind() == BlockKind::Blockquote && block.children.is_empty() {
        block.visible_quote_depth.max(1)
    } else {
        block.visible_quote_depth
    }
}

impl Block {
    fn on_html_details_toggle_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.html_details_open = !self.html_details_open;
        cx.stop_propagation();
        cx.notify();
    }

    fn render_shell(
        &self,
        block_id: ElementId,
        source_mode: bool,
        cursor_style: CursorStyle,
        padding_left: f32,
        padding_right: f32,
        dimensions: &ThemeDimensions,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let base = div()
            .id(block_id)
            .key_context(BLOCK_EDITOR_CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_newline))
            .on_action(cx.listener(Self::on_delete_backward))
            .on_action(cx.listener(Self::on_delete))
            .on_action(cx.listener(Self::on_word_delete_backward))
            .on_action(cx.listener(Self::on_word_delete_forward))
            .on_action(cx.listener(Self::on_focus_previous))
            .on_action(cx.listener(Self::on_focus_next))
            .on_action(cx.listener(Self::on_move_left))
            .on_action(cx.listener(Self::on_move_right))
            .on_action(cx.listener(Self::on_word_move_left))
            .on_action(cx.listener(Self::on_word_move_right))
            .on_action(cx.listener(Self::on_home))
            .on_action(cx.listener(Self::on_end))
            .on_action(cx.listener(Self::on_block_up))
            .on_action(cx.listener(Self::on_block_down))
            .on_action(cx.listener(Self::on_select_left))
            .on_action(cx.listener(Self::on_select_right))
            .on_action(cx.listener(Self::on_word_select_left))
            .on_action(cx.listener(Self::on_word_select_right))
            .on_action(cx.listener(Self::on_select_home))
            .on_action(cx.listener(Self::on_select_end))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_exit_code_block))
            .on_key_down(cx.listener(Self::on_block_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .w_full()
            .min_w(px(0.0))
            .flex_shrink_0()
            .min_h(px(dimensions.block_min_height))
            .py(if matches!(self.kind(), BlockKind::CodeBlock { .. } | BlockKind::MathBlock | BlockKind::MermaidBlock) {
                px(0.0)
            } else {
                px(dimensions.block_padding_y)
            })
            .pl(px(padding_left))
            .pr(px(padding_right))
            .cursor(cursor_style);

        if source_mode {
            base
        } else {
            base.on_action(cx.listener(Self::on_indent_block))
                .on_action(cx.listener(Self::on_outdent_block))
                .on_action(cx.listener(Self::on_bold_selection))
                .on_action(cx.listener(Self::on_italic_selection))
                .on_action(cx.listener(Self::on_underline_selection))
                .on_action(cx.listener(Self::on_code_selection))
                .on_action(cx.listener(Self::on_strikethrough_selection))
        }
    }
}

impl Focusable for Block {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// The render method builds the full element tree for a block:
/// - Common wrapper: key_context, track_focus, action handlers, mouse events.
/// - Kind-specific styling: headings get size/weight/border, list items get
///   a flex row with marker + content, everything else renders as plain text.
/// - The [`BlockTextElement`] handles text layout, selection, and cursor.
impl Render for Block {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let code_language_focused = self.code_language_focus_handle.is_focused(window);
        let input_active = focused || code_language_focused;

        let showing_rendered_image = self.is_showing_rendered_image() && !focused;
        // Inline math and images stay in the projected view while focused (their
        // Markdown source shows as editable text), so links and other styling in the same
        // block keep their attributes instead of collapsing to raw Markdown, the
        // same way script spans already behave.
        self.sync_inline_projection_for_focus(focused);

        if input_active && self.cursor_blink_task.is_none() {
            self.start_cursor_blink(window, cx);
        } else if !input_active && self.cursor_blink_task.is_some() {
            self.cursor_blink_task = None;
        }
        if !input_active {
            self.reset_code_language_input_layout();
        }

        let block_id = ElementId::Name(format!("block-{}", self.data.id).into());
        let is_placeholder =
            focused && self.display_text().is_empty() && self.marked_range.is_none();

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let depth_padding = d.block_padding_x + d.nested_block_indent * self.render_depth as f32;

        if self.is_table_cell() {
            let is_header = self
                .table_cell_position()
                .map(|position| position.is_header())
                .unwrap_or(false);
            // The header row is only styled distinctly (shaded background, medium
            // weight) when the show-table-headers preference is enabled.
            let style_as_header =
                is_header && config::settings::SettingsStore::get(cx).markdown.show_table_headers;
            let base_bg = if style_as_header {
                c.table_header_bg
            } else {
                c.table_cell_bg
            };
            let cell_base = self
                .render_shell(
                    block_id,
                    false,
                    if showing_rendered_image {
                        CursorStyle::PointingHand
                    } else {
                        CursorStyle::IBeam
                    },
                    0.0,
                    0.0,
                    d,
                    cx,
                )
                .w_full()
                .h_full()
                .min_h(px(d.table_cell_min_height))
                .px(px(d.table_cell_padding_x))
                .py(px(d.table_cell_padding_y))
                .border_r(px(1.0))
                .border_b(px(1.0))
                .border_color(c.table_border)
                .bg(base_bg)
                .text_size(px(t.text_size))
                .text_color(c.text_default)
                .line_height(rems(t.text_line_height));

            let cell_base = if style_as_header {
                cell_base.font_weight(FontWeight::MEDIUM)
            } else {
                cell_base
            };

            let cell_content = if showing_rendered_image && let Some(runtime) = self.image_handle()
            {
                self.render_image_content(
                    runtime,
                    Length::Definite(relative(1.0)),
                    px(d.image_cell_max_height),
                    px(d.image_cell_placeholder_height),
                    &theme,
                    &strings,
                )
            } else if !focused
                && let Some(inline_images) = self.render_table_cell_inline_images(
                    &theme,
                    &strings,
                    if style_as_header {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    },
                    cx,
                )
            {
                inline_images
            } else {
                self.render_text_or_mixed_inline_visuals(
                    &theme,
                    focused,
                    is_placeholder,
                    c.text_default,
                    t.text_size,
                    if style_as_header {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    },
                    cx,
                )
            };

            return cell_base
                .relative()
                .flex()
                .items_center()
                .child(
                    div()
                        .w_full()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .child(cell_content),
                )
                .into_any_element();
        }

        // Verbatim-mode rendering: raw text with no formatting.
        if self.is_verbatim_mode()
            && (focused
                || !matches!(
                    self.kind(),
                    BlockKind::HtmlBlock | BlockKind::MathBlock | BlockKind::MermaidBlock
                ))
            && !matches!(self.kind(), BlockKind::MathBlock | BlockKind::MermaidBlock)
        {
            if focused && self.cursor_blink_task.is_none() {
                self.start_cursor_blink(window, cx);
            } else if !focused && self.cursor_blink_task.is_some() {
                self.cursor_blink_task = None;
            }
            let source_base = self
                .render_shell(
                    block_id.clone(),
                    true,
                    CursorStyle::IBeam,
                    d.block_padding_x,
                    d.block_padding_x,
                    d,
                    cx,
                )
                .text_size(px(t.text_size))
                .text_color(c.text_default)
                .line_height(rems(t.text_line_height));

            let source_base = if self.kind() == BlockKind::HtmlComment {
                source_base.bg(c.comment_bg).rounded(px(d.code_block_radius))
            } else if focused {
                source_base.bg(c.source_mode_block_bg).rounded(px(d.code_block_radius))
            } else {
                source_base
            };

            return source_base
                .child(BlockTextElement::new(cx.entity(), is_placeholder))
                .into_any_element();
        }

        let focused_base = self.render_shell(
            block_id.clone(),
            false,
            if showing_rendered_image {
                CursorStyle::PointingHand
            } else {
                CursorStyle::IBeam
            },
            depth_padding,
            d.block_padding_x,
            d,
            cx,
        );

        if self.is_showing_rendered_image() && self.kind() == BlockKind::Paragraph {
            let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
            let max_width = px(effective_image_width(self, viewport_width, d));
            if let Some(runtime) = self.image_handle() {
                let image_preview = self.render_image_content(
                    runtime,
                    max_width.into(),
                    px(d.image_root_max_height),
                    px(d.image_root_placeholder_height),
                    &theme,
                    &strings,
                );

                if !focused {
                    let outer = div()
                        .w_full()
                        .p(relative(0.005))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(image_preview);

                    return focused_base.w_full().child(outer).into_any_element();
                } else {
                    let editor_input = BlockTextElement::new(cx.entity(), is_placeholder);
                    let editor_section = div()
                        .w_full()
                        .px(px(d.code_block_padding_x))
                        .py(px(d.code_block_padding_y))
                        .text_size(px(t.text_size))
                        .text_color(c.text_default)
                        .line_height(rems(t.text_line_height))
                        .child(editor_input);

                    let container = div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .w_full()
                                .bg(c.code_bg)
                                .rounded(px(d.code_block_radius))
                                .child(editor_section),
                        )
                        .child(render_graphic_preview_box(image_preview, &theme));

                    return focused_base
                        .relative()
                        .on_hover(cx.listener(Self::on_code_block_hover))
                        .w_full()
                        .flex()
                        .flex_col()
                        .child(container)
                        .into_any_element();
                }
            }
        }
        let content = match self.kind() {
            BlockKind::ThematicBreak => {
                if !focused {
                    render_thematic_break_unfocused(focused_base, &theme)
                } else {
                    render_thematic_break_focused(
                        self,
                        focused,
                        is_placeholder,
                        focused_base,
                        &theme,
                        cx,
                    )
                }
            }
            BlockKind::Heading { level } => render_heading(
                self,
                level,
                focused,
                is_placeholder,
                focused_base,
                &theme,
                cx,
            ),
            BlockKind::BulletListItem => render_bulleted_list_item(
                self,
                focused,
                is_placeholder,
                showing_rendered_image,
                focused_base,
                &theme,
                window,
                cx,
            ),
            BlockKind::TaskListItem { checked } => render_task_list_item(
                self,
                checked,
                focused,
                is_placeholder,
                showing_rendered_image,
                focused_base,
                &theme,
                window,
                cx,
            ),
            BlockKind::NumberedListItem => render_numbered_list_item(
                self,
                focused,
                is_placeholder,
                showing_rendered_image,
                focused_base,
                &theme,
                window,
                cx,
            ),
            BlockKind::Blockquote => {
                render_blockquote(self, focused, is_placeholder, focused_base, &theme, cx)
            }
            BlockKind::Callout(variant) => render_callout(
                self,
                variant,
                focused,
                is_placeholder,
                focused_base,
                &theme,
                cx,
            ),
            BlockKind::FootnoteDefinition => {
                render_footnote_definition(self, focused, is_placeholder, focused_base, &theme, cx)
            }
            BlockKind::CodeBlock { ref language } => {
                if crate::markdown::block::mermaid::is_mermaid_info_string(language.as_deref()) {
                    render_mermaid_diagram(
                        self,
                        focused,
                        is_placeholder,
                        code_language_focused,
                        focused_base,
                        &theme,
                        &strings,
                        window,
                        cx,
                    )
                } else if language.as_deref().map_or(false, |l| {
                    l.eq_ignore_ascii_case("math") || l.eq_ignore_ascii_case("latex")
                }) {
                    render_latex_math(
                        self,
                        focused,
                        is_placeholder,
                        code_language_focused,
                        focused_base,
                        &theme,
                        &strings,
                        cx,
                    )
                } else {
                    render_fenced_code(
                        self,
                        is_placeholder,
                        code_language_focused,
                        focused_base,
                        &theme,
                        &strings,
                        cx,
                    )
                }
            }
            BlockKind::Table => render_table(
                self,
                focused,
                is_placeholder,
                focused_base,
                &theme,
                window,
                cx,
            ),
            BlockKind::HtmlBlock => render_html_block(self, focused_base, &theme, cx),
            BlockKind::MathBlock => render_latex_math(
                self,
                focused,
                is_placeholder,
                code_language_focused,
                focused_base,
                &theme,
                &strings,
                cx,
            ),
            BlockKind::MermaidBlock => render_mermaid_diagram(
                self,
                focused,
                is_placeholder,
                code_language_focused,
                focused_base,
                &theme,
                &strings,
                window,
                cx,
            ),
            BlockKind::RawMarkdown => {
                render_raw_markdown(self, focused, is_placeholder, focused_base, &theme, cx)
            }
            BlockKind::Paragraph | BlockKind::HtmlComment => {
                render_paragraph(self, focused, is_placeholder, focused_base, &theme, cx)
            }
        };

        wrap_with_quote_guides(content, visible_quote_guides(self), &theme)
    }
}

/// Break a styled inline text run into wrap-friendly chunks for the mixed
/// inline-visual layout. Runs that carry their own box (inline code, background
/// highlight) stay a single chunk so their padding/background is continuous;
/// everything else is split on whitespace with each word keeping its trailing
/// space, so the `flex_wrap` row can break between words instead of pushing the
/// next inline visual onto its own line.
pub fn inline_word_chunks(text: &str, code: bool, has_background: bool) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    if code || has_background {
        return vec![text];
    }
    text.split_inclusive(char::is_whitespace).collect()
}

