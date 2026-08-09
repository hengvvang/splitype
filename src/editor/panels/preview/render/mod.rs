//! Preview panel — read-only rendered snapshot of the document.
//!
//! The preview renders its own snapshot block tree with dedicated read-only
//! block renderers (one module per block kind), deliberately separate from
//! the WYSIWYG editing renderers so preview styling and interactions can
//! diverge independently. Current styles intentionally mirror the WYSIWYG
//! renderers (`crate::editor::panels::wysiwyg::render`).

pub(crate) mod blockquote;
pub(crate) mod callout;
pub(crate) mod fenced_code;
pub(crate) mod footnote;
pub(crate) mod heading;
pub(crate) mod html_block;
pub(crate) mod image;
pub(crate) mod inline;
pub(crate) mod latex_math;
pub(crate) mod list_item;
pub(crate) mod mermaid_diagram;
pub(crate) mod paragraph;
pub(crate) mod table_block;
pub(crate) mod thematic_break;

use gpui::*;

use crate::editor::controller::*;
use crate::editor::tree::block::Block;
use crate::infra::i18n::I18nStrings;
use crate::model::block::BlockKind;
use crate::infra::theme::Theme;

impl Editor {
    pub(crate) fn render_tiled_preview_panel(
        &mut self,
        area_id: usize,
        panel_id: usize,
        theme: &Theme,
        _strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        self.refresh_preview_blocks(cx);

        // Render each snapshot root through the dedicated read-only preview
        // renderers. No GPUI view mounting, no event suppression needed: the
        // preview elements carry no interaction handlers at all.
        let block_elements: Vec<AnyElement> = self
            .tab().preview
            .blocks
            .iter()
            .map(|entity| render_preview_block(entity.read(cx), 0, 0, theme, window, cx))
            .collect();

        div()
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(
                div()
                    .id(ElementId::Name(
                        format!("tiled-preview-scroll-{area_id}-{panel_id}").into(),
                    ))
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .overflow_y_scroll()
                    .p(px(d.editor_padding))
                    .children(block_elements),
            )
            .into_any_element()
    }
}

/// Renders one snapshot block with the read-only preview presentation.
///
/// `depth` is the nesting level used for indentation, mirroring the WYSIWYG
/// `render_depth`-based indent. `quote_depth` tracks blockquote nesting for
/// the quote guide lines. Container blocks recurse into their children.
pub(crate) fn render_preview_block(
    block: &Block,
    depth: usize,
    quote_depth: usize,
    theme: &Theme,
    window: &mut Window,
    cx: &App,
) -> AnyElement {
    let d = &theme.dimensions;

    let depth_padding = d.block_padding_x + d.nested_block_indent * depth as f32;
    let base = div()
        .w_full()
        .min_w(px(0.0))
        .flex_shrink_0()
        .min_h(px(d.block_min_height))
        .py(px(d.block_padding_y))
        .pl(px(depth_padding))
        .pr(px(d.block_padding_x));

    // Blockquote rows and everything inside them sit one quote level deeper.
    let effective_quote_depth = if matches!(block.kind(), BlockKind::Blockquote) {
        quote_depth + 1
    } else {
        quote_depth
    };

    let content = match block.kind() {
        BlockKind::ThematicBreak => thematic_break::render_preview_thematic_break(theme),
        BlockKind::Heading { level } => {
            heading::render_preview_heading(block, level, base, theme)
        }
        BlockKind::BulletListItem => {
            list_item::render_preview_bulleted_list_item(block, depth, base, theme)
        }
        BlockKind::TaskListItem { checked } => {
            list_item::render_preview_task_list_item(block, checked, depth, base, theme)
        }
        BlockKind::NumberedListItem => {
            list_item::render_preview_numbered_list_item(block, depth, base, theme)
        }
        BlockKind::Blockquote => blockquote::render_preview_blockquote(block, depth, base, theme),
        BlockKind::Callout(variant) => {
            callout::render_preview_callout(block, variant, depth, base, theme)
        }
        BlockKind::FootnoteDefinition => {
            footnote::render_preview_footnote_definition(block, depth, base, theme)
        }
        BlockKind::CodeBlock { .. } => fenced_code::render_preview_fenced_code(block, base, theme),
        BlockKind::Table => table_block::render_preview_table(block, base, theme, window),
        BlockKind::HtmlBlock => html_block::render_preview_html_block(block, base, theme),
        BlockKind::MathBlock => latex_math::render_preview_latex_math(block, base, theme),
        BlockKind::MermaidBlock => mermaid_diagram::render_preview_mermaid_diagram(
            block, base, theme, window,
        ),
        BlockKind::RawMarkdown => paragraph::render_preview_raw_markdown(block, base, theme),
        BlockKind::Paragraph | BlockKind::HtmlComment => {
            if block.renders_as_standalone_image() {
                image::render_preview_image(block, base, theme, window)
            } else {
                paragraph::render_preview_paragraph(block, base, theme)
            }
        }
    };

    // Container blocks (blockquote, callout, list item, footnote) render their
    // own content line above; their nested children render below as indented
    // rows, matching the flattened WYSIWYG layout.
    let children_elements: Vec<AnyElement> = block
        .children
        .iter()
        .map(|child| {
            render_preview_block(
                child.read(cx),
                depth + 1,
                effective_quote_depth,
                theme,
                window,
                cx,
            )
        })
        .collect();

    let combined = div()
        .w_full()
        .flex()
        .flex_col()
        .child(content)
        .children(children_elements)
        .into_any_element();

    wrap_with_preview_quote_guides(combined, effective_quote_depth, theme)
}

/// Computes the centered content column width for the preview, mirroring the
/// WYSIWYG centered-column layout (no quote/callout insets since preview
/// depths are structural).
pub(crate) fn preview_centered_column_width(
    viewport_width: f32,
    d: &crate::infra::theme::ThemeDimensions,
) -> f32 {
    let available_content_width = (viewport_width - d.editor_padding * 2.0).max(1.0);
    let ratio = if viewport_width <= d.centered_shrink_start {
        1.0
    } else {
        let t = ((viewport_width - d.centered_shrink_start)
            / (d.centered_shrink_end - d.centered_shrink_start))
            .clamp(0.0, 1.0);
        1.0 - t * (1.0 - d.centered_min_ratio)
    };
    (available_content_width * ratio).max(320.0).min(available_content_width)
}

/// Wraps content with the quote guide lines at the given depth, mirroring
/// the WYSIWYG quote guides.
fn wrap_with_preview_quote_guides(
    content: AnyElement,
    quote_depth: usize,
    theme: &Theme,
) -> AnyElement {
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
