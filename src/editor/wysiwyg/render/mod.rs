//! Rendering for [`Block`] via GPUI's high-level [`Render`] trait.
//!
//! Each block kind produces a distinct visual style: H1 has a bottom border,
//! list items render a marker column (bullet / ordinal), and raw Markdown
//! fallback renders as plain text. This is the WYSIWYG editing presentation;
//! the read-only preview presentation lives in
//! `crate::editor::preview::render`.

pub(crate) mod blockquote;
pub(crate) mod callout;
pub(crate) mod code_ui;
pub(crate) mod fenced_code;
pub(crate) mod footnote;
pub(crate) mod heading;
pub(crate) mod html_block;
pub(crate) mod html_document;
pub(crate) mod image_handle;
pub(crate) mod inline;
pub(crate) mod inline_visuals;
pub(crate) mod latex_math;
pub(crate) mod layout;
pub(crate) mod list_item;
pub(crate) mod mermaid_diagram;
pub(crate) mod paragraph;
pub(crate) mod raw_markdown;
pub(crate) mod table_block;
pub(crate) mod thematic_break;

use gpui::*;

const BLOCK_EDITOR_CONTEXT: &str = "BlockEditor";

use crate::editor::block_protocol::BlockAction;
use crate::editor::controller::Editor;
use crate::editor::wysiwyg::render::inline::text_element::BlockTextElement;
use crate::editor::wysiwyg::render::{
    blockquote::render_blockquote,
    callout::render_callout,
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
use crate::editor::render::latex_render::{display_math_font_size, render_display_math_svg};
use crate::editor::render::mermaid_render::{
    mermaid_content_fingerprint, render_mermaid_svg_for_display,
};
use crate::editor::tree::block::{Block, ImageHandle};
use crate::infra::i18n::{I18nManager, I18nStrings};
use crate::infra::theme::{Theme, ThemeDimensions, ThemeManager};
use crate::model::block::BlockKind;
use crate::model::syntax::image::ImageResolvedSource;
use crate::model::syntax::math::parse_display_math_source;
use crate::model::syntax::mermaid::parse_mermaid_fence_source;
use crate::model::syntax::table::{TableAxisHighlight, TableAxisKind};

#[allow(dead_code)]
const TASK_CHECKMARK: &str = "\u{2713}";

pub(crate) fn render_custom_bullet_marker(depth: usize, color: Hsla) -> AnyElement {
    match depth % 3 {
        0 => {
            // Level 1: Solid Circle Disc (e.g. 5.5px)
            div()
                .size(px(5.5))
                .rounded_full()
                .bg(color)
                .into_any_element()
        }
        1 => {
            // Level 2: Hollow Circle (e.g. 5.5px outer, 1.2px stroke border)
            div()
                .size(px(5.5))
                .rounded_full()
                .border_1()
                .border_color(color)
                .into_any_element()
        }
        _ => {
            // Level 3+: Solid Square (e.g. 4.5px x 4.5px solid square)
            div()
                .size(px(4.5))
                .rounded(px(0.5))
                .bg(color)
                .into_any_element()
        }
    }
}

/// Makes a row-axis highlight color more opaque (more solid, still translucent)
/// for the header row, keeping the theme's hue so the header handle reads as a
/// stronger version of the body-row handles in whatever colors the theme uses.
#[allow(dead_code)]
fn header_axis_emphasis(color: Hsla) -> Hsla {
    Hsla {
        a: color.a + (1.0 - color.a) * 0.5,
        ..color
    }
}
pub(crate) fn render_image_placeholder(
    runtime: &ImageHandle,
    width: Length,
    height: Pixels,
    theme: &Theme,
    _strings: &I18nStrings,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let target_name = if !runtime.src.trim().is_empty() {
        runtime.src.trim()
    } else if !runtime.alt.trim().is_empty() {
        runtime.alt.trim()
    } else {
        "unnamed"
    };

    let title_text = if !runtime.alt.trim().is_empty() {
        format!("Image Not Found: {}", runtime.alt.trim())
    } else {
        "Image Not Found".to_string()
    };

    div()
        .w_full()
        .w(width)
        .h(height)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .rounded_none()
        .bg(c.image_placeholder_bg)
        .px(px(d.block_padding_x))
        .py(px(16.0))
        .child(
            div()
                .text_size(px(t.text_size))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.image_placeholder_text)
                .child(SharedString::from(title_text)),
        )
        .child(
            div()
                .text_size(px(t.code_size))
                .text_color(c.dialog_muted)
                .child(SharedString::from(format!("({})", target_name))),
        )
        .into_any_element()
}

pub(crate) fn render_loading_placeholder(
    runtime: &ImageHandle,
    width: Length,
    height: Pixels,
    theme: &Theme,
    _strings: &I18nStrings,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let target_name = if !runtime.src.trim().is_empty() {
        runtime.src.trim()
    } else {
        "image"
    };

    div()
        .w_full()
        .w(width)
        .h(height)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .rounded_none()
        .bg(c.image_placeholder_bg)
        .px(px(d.block_padding_x))
        .py(px(16.0))
        .child(
            div()
                .text_size(px(t.text_size))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.image_placeholder_text)
                .child(SharedString::from("Loading image...")),
        )
        .child(
            div()
                .text_size(px(t.code_size))
                .text_color(c.dialog_muted)
                .child(SharedString::from(format!("({})", target_name))),
        )
        .into_any_element()
}

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
                .left(px(guide_offset * level as f32))
                .w(px(d.quote_border_width))
                .bg(c.border_quote)
        }))
        .into_any_element()
}

pub(crate) fn callout_accent_and_background(
    variant: crate::model::block::CalloutKind,
    theme: &Theme,
) -> (Hsla, Hsla) {
    let c = &theme.colors;
    match variant {
        crate::model::block::CalloutKind::Note => (c.callout_note_border, c.callout_note_bg),
        crate::model::block::CalloutKind::Tip => (c.callout_tip_border, c.callout_tip_bg),
        crate::model::block::CalloutKind::Important => {
            (c.callout_important_border, c.callout_important_bg)
        }
        crate::model::block::CalloutKind::Warning => {
            (c.callout_warning_border, c.callout_warning_bg)
        }
        crate::model::block::CalloutKind::Caution => {
            (c.callout_caution_border, c.callout_caution_bg)
        }
    }
}

fn visible_quote_guides(block: &Block) -> usize {
    block.visible_quote_depth
}

pub(crate) fn effective_table_width(
    block: &Block,
    viewport_width: f32,
    d: &ThemeDimensions,
) -> f32 {
    let centered_width = Editor::centered_column_width(viewport_width, d);
    let visible_quote_guides = visible_quote_guides(block);
    let quote_inset = d.quote_padding_left * visible_quote_guides as f32;
    let callout_inset = if block.callout_depth > 0 {
        d.callout_padding_x * 2.0 + d.callout_border_width
    } else {
        0.0
    };

    (centered_width - quote_inset - callout_inset)
        .max((d.table_cell_padding_x * 2.0 + 80.0).max(120.0))
}

fn container_image_width_budget(block: &Block, viewport_width: f32, d: &ThemeDimensions) -> f32 {
    let centered_width = Editor::centered_column_width(viewport_width, d);
    let visible_quote_guides = visible_quote_guides(block);
    let quote_inset = d.quote_padding_left * visible_quote_guides as f32;
    let callout_inset = if block.callout_depth > 0 {
        d.callout_padding_x * 2.0 + d.callout_border_width
    } else {
        0.0
    };

    centered_width - quote_inset - callout_inset
}

pub(crate) fn effective_image_width(
    block: &Block,
    viewport_width: f32,
    d: &ThemeDimensions,
) -> f32 {
    let list_inset = d.nested_block_indent * block.render_depth as f32;
    (container_image_width_budget(block, viewport_width, d) - d.block_padding_x * 2.0 - list_inset)
        .max(160.0)
}

pub(crate) fn effective_list_item_image_width(
    block: &Block,
    viewport_width: f32,
    d: &ThemeDimensions,
) -> f32 {
    let marker_width = match block.kind() {
        BlockKind::BulletListItem => d.list_marker_width,
        BlockKind::TaskListItem { .. } => d.list_marker_width.max(d.task_checkbox_size),
        BlockKind::NumberedListItem => d.ordered_list_marker_width,
        _ => 0.0,
    };
    let list_inset = d.nested_block_indent * block.render_depth as f32;

    (container_image_width_budget(block, viewport_width, d)
        - d.block_padding_x * 2.0
        - list_inset
        - marker_width
        - d.list_marker_gap)
        .max(160.0)
}

/// Returns a human-readable list ordinal: numbers at depth 0, lowercase
/// letters at depth 1, and unicode roman numerals at depth 2+.
pub(crate) fn numbered_list_marker(depth: usize, ordinal: usize) -> String {
    match depth {
        0 => format!("{ordinal}."),
        1 => format!("{}.", alphabetic_list_marker(ordinal)),
        _ => format!("{}.", roman_list_marker(ordinal)),
    }
}

/// Expands beyond 26 by wrapping: a...z, a1...z1, a2...z2, ...
fn alphabetic_list_marker(ordinal: usize) -> String {
    const ALPHABET: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

    let ordinal = ordinal.max(1);
    if ordinal <= ALPHABET.len() {
        return char::from(ALPHABET[ordinal - 1]).to_string();
    }

    let wrapped = ordinal - (ALPHABET.len() + 1);
    let letter = char::from(ALPHABET[wrapped % ALPHABET.len()]);
    let suffix = wrapped + 1;
    format!("{letter}{suffix}")
}

/// Converts an ASCII roman numeral string to its unicode ligature equivalents
/// where possible (for example, "III" to a single roman numeral glyph).
fn roman_list_marker(ordinal: usize) -> String {
    let ascii = ascii_roman_numeral(ordinal.max(1));
    let mut index = 0;
    let mut marker = String::new();

    while index < ascii.len() {
        let remaining = &ascii[index..];
        if let Some((token_len, token)) = roman_unicode_token(remaining) {
            marker.push_str(token);
            index += token_len;
        } else {
            break;
        }
    }

    marker
}

fn ascii_roman_numeral(mut ordinal: usize) -> String {
    const MAP: &[(usize, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];

    let mut result = String::new();
    for (value, symbol) in MAP {
        while ordinal >= *value {
            result.push_str(symbol);
            ordinal -= *value;
        }
    }
    result
}

fn roman_unicode_token(remaining: &str) -> Option<(usize, &'static str)> {
    const TOKENS: &[(&str, &str)] = &[
        ("XII", "\u{216B}"),
        ("XI", "\u{216A}"),
        ("IX", "\u{2168}"),
        ("VIII", "\u{2167}"),
        ("VII", "\u{2166}"),
        ("VI", "\u{2165}"),
        ("IV", "\u{2163}"),
        ("III", "\u{2162}"),
        ("II", "\u{2161}"),
        ("I", "\u{2160}"),
        ("V", "\u{2164}"),
        ("X", "\u{2169}"),
        ("L", "\u{216C}"),
        ("C", "\u{216D}"),
        ("D", "\u{216E}"),
        ("M", "\u{216F}"),
    ];

    TOKENS.iter().find_map(|(ascii, unicode)| {
        remaining
            .starts_with(ascii)
            .then_some((ascii.len(), *unicode))
    })
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

    pub(crate) fn render_image_content(
        &self,
        runtime: &ImageHandle,
        max_width: Length,
        max_height: Pixels,
        placeholder_height: Pixels,
        theme: &Theme,
        strings: &I18nStrings,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let source = runtime.resolved_source.clone();
        let placeholder_theme = theme.clone();
        let loading_theme = theme.clone();
        let placeholder_strings = strings.clone();
        let loading_strings = strings.clone();
        let runtime_for_fallback = runtime.clone();
        let runtime_for_loading = runtime.clone();

        let image = match source {
            ImageResolvedSource::Local(path) => img(path),
            ImageResolvedSource::Remote(uri) => img(uri),
        }
        .max_w(max_width)
        .max_h(max_height)
        .object_fit(ObjectFit::Contain)
        .with_fallback(move || {
            render_image_placeholder(
                &runtime_for_fallback,
                max_width,
                placeholder_height,
                &placeholder_theme,
                &placeholder_strings,
            )
        })
        .with_loading(move || {
            render_loading_placeholder(
                &runtime_for_loading,
                max_width,
                placeholder_height,
                &loading_theme,
                &loading_strings,
            )
        });

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(d.image_caption_gap))
            .child(image);

        if let Some(title) = runtime
            .title
            .as_ref()
            .filter(|title| !title.trim().is_empty())
        {
            container = container.child(
                div()
                    .w_full()
                    .text_center()
                    .text_size(px(t.code_size))
                    .text_color(c.image_caption_text)
                    .child(SharedString::from(title.clone())),
            );
        }

        container.into_any_element()
    }

    pub(crate) fn render_math_content(&self, theme: &Theme) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let raw = self
            .record
            .raw_source
            .as_deref()
            .unwrap_or_else(|| self.display_text());

        let source = parse_display_math_source(raw).unwrap_or_else(|| {
            let body = raw
                .trim()
                .strip_prefix("$$")
                .unwrap_or(raw.trim())
                .strip_suffix("$$")
                .unwrap_or(raw.trim())
                .trim()
                .to_string();
            crate::model::syntax::math::DisplayMathSource {
                raw: raw.to_string(),
                body,
            }
        });

        if source.body.is_empty() {
            return div()
                .w_full()
                .text_size(px(t.text_size))
                .line_height(rems(t.text_line_height))
                .text_color(c.text_default)
                .child(SharedString::from(raw.to_string()))
                .into_any_element();
        }

        match render_display_math_svg(&source, c.text_default, display_math_font_size(t.text_size))
        {
            Ok(rendered) => div()
                .w_full()
                .flex()
                .justify_center()
                .py(px(d.block_padding_y.max(6.0)))
                .child(
                    img(rendered.path.clone())
                        .max_w(Length::Definite(relative(1.0)))
                        .max_h(px(d.image_root_max_height))
                        .object_fit(ObjectFit::Contain),
                )
                .into_any_element(),
            Err(err) => div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .rounded_sm()
                .bg(c.source_mode_block_bg)
                .px(px(d.block_padding_x))
                .py(px(d.block_padding_y))
                .text_size(px(t.text_size))
                .line_height(rems(t.text_line_height))
                .text_color(c.text_default)
                .child(SharedString::from(raw.to_string()))
                .child(
                    div()
                        .text_size(px(t.code_size))
                        .text_color(c.dialog_muted)
                        .child(SharedString::from(format!("LaTeX render error: {err}"))),
                )
                .into_any_element(),
        }
    }

    pub(crate) fn render_mermaid_content(&mut self, theme: &Theme, window: &Window) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let raw = self
            .record
            .raw_source
            .as_deref()
            .unwrap_or_else(|| self.display_text());

        let source = parse_mermaid_fence_source(raw).unwrap_or_else(|| {
            let trimmed = raw.trim();
            let body = if let Some(rest) = trimmed
                .strip_prefix("```mermaid")
                .or_else(|| trimmed.strip_prefix("```"))
            {
                rest.strip_suffix("```").unwrap_or(rest).trim().to_string()
            } else {
                trimmed.to_string()
            };
            crate::model::syntax::mermaid::MermaidSource {
                raw: raw.to_string(),
                body,
                info: "mermaid".to_string(),
            }
        });

        if source.body.is_empty() {
            return div()
                .w_full()
                .text_size(px(t.text_size))
                .line_height(rems(t.text_line_height))
                .text_color(c.text_default)
                .child(SharedString::from(raw.to_string()))
                .into_any_element();
        }

        let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
        let available_width = effective_image_width(self, viewport_width, d);

        // The display render is content-addressed, so the per-block cache
        // stays valid until the diagram source or the layout parameters
        // change; cursor-blink repaints then skip the SVG file reads.
        let fingerprint = mermaid_content_fingerprint(&source.body);
        let width_key = available_width.to_bits();
        let viewport_key = viewport_width.to_bits();
        let cached = self
            .mermaid_render_cache
            .as_ref()
            .filter(|(cached_fingerprint, cached_width, cached_viewport, _)| {
                *cached_fingerprint == fingerprint
                    && *cached_width == width_key
                    && *cached_viewport == viewport_key
            })
            .map(|(_, _, _, rendered)| rendered.clone());

        match cached {
            Some(rendered) => Self::render_mermaid_svg_element(rendered, available_width, self, d),
            None => {
                match render_mermaid_svg_for_display(&source, available_width, viewport_width) {
                    Ok(rendered) => {
                        self.mermaid_render_cache =
                            Some((fingerprint, width_key, viewport_key, rendered.clone()));
                        Self::render_mermaid_svg_element(rendered, available_width, self, d)
                    }
                    Err(err) => div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .rounded_sm()
                        .bg(c.source_mode_block_bg)
                        .px(px(d.block_padding_x))
                        .py(px(d.block_padding_y))
                        .text_size(px(t.text_size))
                        .line_height(rems(t.text_line_height))
                        .text_color(c.text_default)
                        .child(SharedString::from(raw.to_string()))
                        .child(
                            div()
                                .text_size(px(t.code_size))
                                .text_color(c.dialog_muted)
                                .child(SharedString::from(format!("Mermaid render error: {err}"))),
                        )
                        .into_any_element(),
                }
            }
        }
    }

    pub(crate) fn render_mermaid_svg_element(
        rendered: crate::editor::render::mermaid_render::MermaidSvgRender,
        available_width: f32,
        block: &Block,
        d: &ThemeDimensions,
    ) -> AnyElement {
        let display_width = rendered.display_width.max(1.0);
        let display_height = rendered.display_height.max(1.0);
        let image_path = rendered.path.clone();
        let image = move || {
            img(image_path.clone())
                .w(px(display_width))
                .h(px(display_height))
        };
        let content = if display_width <= available_width + 0.5 {
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(image())
                .into_any_element()
        } else {
            div()
                .id(ElementId::Name(
                    format!("mermaid-scroll-{}", block.record.id).into(),
                ))
                .w_full()
                .overflow_x_scroll()
                .scrollbar_width(px(0.0))
                .child(div().w(px(display_width)).child(image()))
                .into_any_element()
        };

        div()
            .w_full()
            .py(px(d.block_padding_y.max(6.0)))
            .child(content)
            .into_any_element()
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
            .on_action(cx.listener(Self::on_delete_back))
            .on_action(cx.listener(Self::on_delete))
            .on_action(cx.listener(Self::on_word_delete_back))
            .on_action(cx.listener(Self::on_word_delete_forward))
            .on_action(cx.listener(Self::on_focus_prev))
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
            .py(px(dimensions.block_padding_y))
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
        if self.sync_image_focus_state(focused) {
            cx.notify();
        }

        let showing_rendered_image = self.showing_rendered_image();
        // Inline math stays in the projected view while focused (its `$...$`
        // source shows as editable text), so links and other styling in the same
        // block keep their attributes instead of collapsing to raw Markdown, the
        // same way script spans already behave.
        self.sync_inline_projection_for_focus(focused && !showing_rendered_image);

        if input_active && self.cursor_blink_task.is_none() {
            self.start_cursor_blink(cx);
        } else if !input_active && self.cursor_blink_task.is_some() {
            self.cursor_blink_task = None;
        }
        if !input_active {
            self.reset_code_language_input_layout();
        }

        let block_id = ElementId::Name(format!("block-{}", self.record.id).into());
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
                is_header && crate::infra::config::settings::EditorSettings::show_table_headers(cx);
            let highlight = self.table_axis_highlight;
            let base_bg = if style_as_header {
                c.table_header_bg
            } else {
                c.table_cell_bg
            };
            let bg = match highlight {
                TableAxisHighlight::None => base_bg,
                TableAxisHighlight::Preview => c.table_axis_preview_bg,
                TableAxisHighlight::Selected => c.table_axis_selected_bg,
            };
            let border_color = if focused {
                c.table_cell_active_outline
            } else {
                match highlight {
                    TableAxisHighlight::None => c.table_border,
                    TableAxisHighlight::Preview => c.table_selection_border,
                    TableAxisHighlight::Selected => c.table_selection_border,
                }
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
                .px(px(0.0))
                .py(px(d.table_cell_padding_y))
                .border(if focused { px(2.0) } else { px(1.0) })
                .border_color(border_color)
                .bg(bg)
                .text_size(px(t.text_size))
                .text_color(c.text_default)
                .line_height(rems(t.text_line_height));

            let cell_base = if style_as_header {
                cell_base.font_weight(FontWeight::MEDIUM)
            } else {
                cell_base
            };

            let cell_content = if showing_rendered_image && let Some(runtime) = self.image_runtime()
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
                    None,
                    None,
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

            let left_reserved_slot = div().flex_none().w(px(12.0)).h_full();

            let is_menu_open = self.table_axis_selection.is_some();
            let menu_block = cx.entity();
            let cell_pos = self.table_cell_position();

            let cell_menu = div()
                .id(ElementId::Name(
                    format!("table-cell-menu-{}", self.record.id).into(),
                ))
                .w(px(12.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .opacity(if focused || is_menu_open { 1.0 } else { 0.0 })
                .hover(|this| this.bg(c.table_append_button_hover).opacity(1.0))
                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                    if let Some(pos) = cell_pos {
                        let _ = menu_block.update(cx, |_block, cx| {
                            cx.stop_propagation();
                            cx.emit(BlockAction::RequestOpenTableAxisMenu {
                                kind: TableAxisKind::Row,
                                index: pos.row,
                                position: event.position,
                            });
                        });
                    }
                })
                .child(
                    div()
                        .w(px(3.5))
                        .h(px(12.0))
                        .rounded(px(1.5))
                        .bg(if is_menu_open {
                            c.table_append_button_text
                        } else {
                            c.table_handle_icon
                        }),
                );

            let right_reserved_slot = div()
                .flex_none()
                .w(px(12.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(cell_menu);

            let cell_center_content = div()
                .flex_1()
                .min_w(px(0.0))
                .h_full()
                .flex()
                .items_center()
                .child(cell_content);

            let cell_wrapper = div()
                .w_full()
                .h_full()
                .flex()
                .items_center()
                .child(left_reserved_slot)
                .child(cell_center_content)
                .child(right_reserved_slot);

            return cell_base.relative().child(cell_wrapper).into_any_element();
        }

        // Source-mode rendering: raw text with no formatting.
        if self.is_source_raw_mode()
            && (focused
                || !matches!(
                    self.kind(),
                    BlockKind::HtmlBlock | BlockKind::MathBlock | BlockKind::MermaidBlock
                ))
            && !matches!(self.kind(), BlockKind::MathBlock | BlockKind::MermaidBlock)
        {
            if focused && self.cursor_blink_task.is_none() {
                self.start_cursor_blink(cx);
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
                source_base.bg(c.comment_bg).rounded_sm()
            } else if focused {
                source_base.bg(c.source_mode_block_bg).rounded_sm()
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

        if showing_rendered_image && self.kind() == BlockKind::Paragraph {
            let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
            let max_width = px(effective_image_width(self, viewport_width, d));
            if let Some(runtime) = self.image_runtime() {
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
                        .child(
                            div()
                                .w_full()
                                .p(relative(0.005))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(image_preview),
                        )
                        .child(editor_section);

                    return focused_base
                        .relative()
                        .on_hover(cx.listener(Self::on_code_block_hover))
                        .bg(c.code_bg)
                        .rounded(px(d.menu_item_radius))
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
            BlockKind::CodeBlock { .. } => render_fenced_code(
                self,
                is_placeholder,
                code_language_focused,
                focused_base,
                &theme,
                &strings,
                cx,
            ),
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
/// Wraps a rendered inline link run so the hand cursor only appears while the
/// Cmd/Ctrl follow modifier is held. Links in mixed inline-visual blocks (math,
/// scripts, inline images) render as plain divs, so this sets `PointingHand`
/// when its hitbox is hovered and the modifier is down, like `BlockTextElement`
/// does for normal text. The editor root repaints on follow-modifier toggles,
/// so the cursor re-evaluates without the pointer moving. Layout and painting
/// are delegated to the child.
struct LinkFollowCursor {
    child: AnyElement,
}

impl IntoElement for LinkFollowCursor {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for LinkFollowCursor {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.child.prepaint(window, cx);
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if hitbox.is_hovered(window) && window.modifiers().secondary() {
            // The editor root repaints on follow-modifier toggles, so the hand
            // cursor re-evaluates here even while the pointer stays still.
            window.set_cursor_style(CursorStyle::PointingHand, hitbox);
        }
        self.child.paint(window, cx);
    }
}

pub(crate) fn inline_word_chunks(text: &str, code: bool, has_background: bool) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    if code || has_background {
        return vec![text];
    }
    text.split_inclusive(char::is_whitespace).collect()
}

#[cfg(test)]
mod tests {
    use super::inline_word_chunks;
    use crate::editor::wysiwyg::render::html_document::{
        HtmlComputedStyle, html_node_visual_style,
    };
    use crate::editor::tree::block::Block;
    use crate::infra::i18n::I18nManager;
    use crate::infra::theme::{Theme, ThemeManager};
    use crate::model::block::{BlockData, BlockKind};
    use crate::model::inline::text::RichText;
    use crate::model::syntax::html::parse_html_document;
    use gpui::{Hsla, Rgba, TestAppContext, px};

    fn assert_color_near(color: Hsla, red: u8, green: u8, blue: u8, alpha: u8) {
        let color = Rgba::from(color);
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as i16;
        assert!((channel(color.r) - red as i16).abs() <= 1);
        assert!((channel(color.g) - green as i16).abs() <= 1);
        assert!((channel(color.b) - blue as i16).abs() <= 1);
        assert!((channel(color.a) - alpha as i16).abs() <= 1);
    }

    #[test]
    fn inline_word_chunks_split_text_runs_for_wrapping() {
        // Plain runs split per word so the flex-wrap row can break between
        // words and keep neighboring inline math on the same visual line.
        assert_eq!(
            inline_word_chunks("Fusce x malesuada", false, false),
            vec!["Fusce ", "x ", "malesuada"],
        );
        // Trailing whitespace stays attached so spacing survives the split.
        assert_eq!(inline_word_chunks("end ", false, false), vec!["end "]);
        assert!(inline_word_chunks("", false, false).is_empty());
    }

    #[test]
    fn inline_word_chunks_keep_boxed_runs_whole() {
        // Inline code and background highlights keep their box continuous.
        assert_eq!(
            inline_word_chunks("let x = 2", true, false),
            vec!["let x = 2"],
        );
        assert_eq!(
            inline_word_chunks("highlighted text", false, true),
            vec!["highlighted text"],
        );
    }

    #[test]
    fn html_render_style_inherits_color_and_font_size() {
        let theme = Theme::default_theme();
        let doc = parse_html_document(
            "<div style=\"color:blue; font-size:20px\"><span style=\"font-size:120%\">x</span></div>",
        );
        let root = HtmlComputedStyle::root(&theme);
        let parent = html_node_visual_style(&doc.nodes[0], root, &theme);
        let child = html_node_visual_style(&doc.nodes[0].children[0], parent.computed, &theme);

        assert_color_near(parent.computed.color, 0, 0, 255, 255);
        assert_color_near(child.computed.color, 0, 0, 255, 255);
        assert!((child.computed.font_size - 24.0).abs() < 0.01);
    }

    #[test]
    fn html_render_style_overrides_link_and_mark_defaults() {
        let theme = Theme::default_theme();
        let link_doc = parse_html_document("<a style=\"color:red\">x</a>");
        let link_style =
            html_node_visual_style(&link_doc.nodes[0], HtmlComputedStyle::root(&theme), &theme);
        assert_color_near(link_style.computed.color, 255, 0, 0, 255);

        let mark_doc = parse_html_document("<mark style=\"background-color:#123\">x</mark>");
        let mark_style =
            html_node_visual_style(&mark_doc.nodes[0], HtmlComputedStyle::root(&theme), &theme);
        assert_color_near(mark_style.background.unwrap(), 0x11, 0x22, 0x33, 0xff);
    }

    #[test]
    fn html_render_style_does_not_inherit_background_color() {
        let theme = Theme::default_theme();
        let doc =
            parse_html_document("<div style=\"background-color:#112233\"><span>child</span></div>");
        let root = HtmlComputedStyle::root(&theme);
        let parent = html_node_visual_style(&doc.nodes[0], root, &theme);
        let child = html_node_visual_style(&doc.nodes[0].children[0], parent.computed, &theme);

        assert_color_near(parent.background.unwrap(), 0x11, 0x22, 0x33, 0xff);
        assert!(child.background.is_none());
    }

    #[gpui::test]
    async fn code_language_picker_opens_below_right_toolbar(cx: &mut TestAppContext) {
        cx.update(|cx| {
            I18nManager::init(cx);
            ThemeManager::init(cx);
        });
        let (block, cx) = cx.add_window_view(|_window, cx| {
            Block::with_record(
                cx,
                BlockData::new(
                    BlockKind::CodeBlock {
                        language: Some("rust".into()),
                    },
                    RichText::plain("fn main() {}\n"),
                ),
            )
        });

        cx.update(|window, cx| {
            block.update(cx, |block, _cx| {
                block.code_toolbar_hovered = true;
                block.code_language_picker_open = true;
                block.code_language_focus_handle.focus(window);
            });
            window.draw(cx).clear();
        });
        cx.run_until_parked();

        let (text_bounds, language_bounds) = block.read_with(cx, |block, _cx| {
            (
                block.last_bounds.expect("code text should render"),
                block
                    .code_language_last_bounds
                    .expect("language input should render"),
            )
        });
        assert!(language_bounds.left() > text_bounds.left());
        assert!(language_bounds.top() > text_bounds.top());
        // The picker is a fixed 230px panel anchored right; the input inside
        // it renders slightly narrower.
        assert!(language_bounds.size.width >= px(180.0));
        assert!(language_bounds.size.width <= px(240.0));
    }
}
