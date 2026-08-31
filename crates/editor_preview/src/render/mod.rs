//! Preview panel — read-only rendered snapshot of the document.
//!
//! The preview renders its own snapshot block tree with dedicated read-only
//! block renderers (one module per block kind), deliberately separate from
//! the WYSIWYG editing renderers so preview styling and interactions can
//! diverge independently. Current styles intentionally mirror the WYSIWYG
//! renderers.
//!
//! This module owns the full preview presentation; the coordinating crate
//! (app) only refreshes the preview tree, routes focus and hands over the
//! scroll shell.

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

use std::ops::Range;
use std::sync::Arc;

use gpui::*;

use editor_model::{PaneHost, PaneId, PaneRenderContext};
use config::language::I18nStrings;
use theme::Theme;
use markdown_ast_parser::parse::BlockKind;

use crate::node::PreviewBlock;
use crate::state::PreviewState;

/// Renders the preview pane for `state` inside the view shell described by
/// `view` (pane id, scroll handle, host proxy). The coordinating crate
/// refreshes the preview tree and applies pending focus/autoscroll before
/// calling this.
pub fn render_preview_pane(
    state: &PreviewState,
    view: &PaneRenderContext,
    theme: &Theme,
    _strings: &I18nStrings,
    window: &mut Window,
    cx: &App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let preview_selection = state.selection;

    let mut block_elements: Vec<AnyElement> = state
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| !matches!(block.kind(), BlockKind::FootnoteDefinition))
        .map(|(block_index, block)| {
            let selection_range = preview_selection
                .and_then(|sel| sel.range_for_block(block_index, block.display_len()));
            render_preview_block(
                block,
                block_index,
                selection_range,
                0,
                0,
                view.pane_id,
                &view.host,
                theme,
                window,
                cx,
            )
        })
        .collect();
    // Footnote definitions are collected out of the body flow and
    // rendered as one GitHub-style section at the bottom, behind a
    // divider line from the main content.
    let mut footnotes: Vec<PreviewBlock> = Vec::new();
    collect_preview_footnote_definitions(&state.blocks, &mut footnotes);
    if !footnotes.is_empty() {
        block_elements.push(footnote::render_preview_footnotes_section(
            &footnotes, view.pane_id, &view.host, theme, window, cx,
        ));
    }

    div()
        .w_full()
        .h_full()
        .relative()
        .bg(c.editor_background)
        .child(
            div()
                .id(ElementId::Name(
                    format!("tiled-preview-scroll-{}", view.pane_id).into(),
                ))
                .w_full()
                .h_full()
                .flex()
                .flex_col()
                .items_center()
                .overflow_y_scroll()
                .track_scroll(view.scroll)
                .p(px(d.editor_padding))
                .children(block_elements),
        )
        .into_any_element()
}

/// Renders one snapshot block with the read-only preview presentation.
///
/// `depth` is the nesting level used for indentation, mirroring the WYSIWYG
/// `render_depth`-based indent. `quote_depth` tracks blockquote nesting for
/// the quote guide lines. Container blocks recurse into their children.
pub(crate) fn render_preview_block(
    block: &PreviewBlock,
    block_index: usize,
    selection_range: Option<Range<usize>>,
    depth: usize,
    quote_depth: usize,
    pane_id: PaneId,
    host: &Arc<dyn PaneHost>,
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
            heading::render_preview_heading(block, level, selection_range.clone(), base, theme)
        }
        BlockKind::BulletListItem => {
            list_item::render_preview_bulleted_list_item(block, depth, selection_range.clone(), base, theme)
        }
        BlockKind::TaskListItem { checked } => {
            list_item::render_preview_task_list_item(block, checked, depth, selection_range.clone(), base, theme)
        }
        BlockKind::NumberedListItem => {
            list_item::render_preview_numbered_list_item(block, depth, selection_range.clone(), base, theme)
        }
        BlockKind::Blockquote => {
            blockquote::render_preview_blockquote(block, depth, selection_range.clone(), base, theme)
        }
        BlockKind::Callout(variant) => {
            callout::render_preview_callout(block, variant, depth, selection_range.clone(), base, theme)
        }
        BlockKind::FootnoteDefinition => {
            footnote::render_preview_footnote_definition(block, depth, base, theme)
        }
        BlockKind::CodeBlock { ref language } => {
            if markdown_ast_parser::block::mermaid::is_mermaid_info_string(language.as_deref()) {
                mermaid_diagram::render_preview_mermaid_diagram(block, base, theme, window)
            } else if language.as_deref().map_or(false, |l| {
                l.eq_ignore_ascii_case("math") || l.eq_ignore_ascii_case("latex")
            }) {
                latex_math::render_preview_latex_math(block, base, theme)
            } else {
                fenced_code::render_preview_fenced_code(block, base, theme)
            }
        }
        BlockKind::Table => table_block::render_preview_table(block, base, theme, window),
        BlockKind::HtmlBlock => html_block::render_preview_html_block(block, base, theme),
        BlockKind::MathBlock => latex_math::render_preview_latex_math(block, base, theme),
        BlockKind::MermaidBlock => {
            mermaid_diagram::render_preview_mermaid_diagram(block, base, theme, window)
        }
        BlockKind::RawMarkdown => {
            paragraph::render_preview_raw_markdown(block, selection_range.clone(), base, theme)
        }
        BlockKind::Paragraph | BlockKind::HtmlComment => {
            if block.is_standalone_image() {
                image::render_preview_image(block, base, theme, window)
            } else {
                paragraph::render_preview_paragraph(block, selection_range.clone(), base, theme)
            }
        }
    };

    // Container blocks (blockquote, callout, list item, footnote) render their
    // own content line above; their nested children render below as indented
    // rows, matching the flattened WYSIWYG layout. Footnote definitions are
    // excluded here as well — they are collected into the bottom section.
    let children_elements: Vec<AnyElement> = block
        .children
        .iter()
        .filter(|child| !matches!(child.kind(), BlockKind::FootnoteDefinition))
        .map(|child| {
            render_preview_block(
                child,
                block_index,
                selection_range.clone(),
                depth + 1,
                effective_quote_depth,
                pane_id,
                host,
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

    if let BlockKind::Callout(variant) = block.kind() {
        let (accent, _) = syntax_highlighter::render_helpers::callout_colors(variant, theme);
        div()
            .w_full()
            .relative()
            .pl(px(d.quote_padding_left))
            .child(combined)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(d.block_padding_x))
                    .w(px(d.callout_border_width))
                    .bg(accent),
            )
            .into_any_element()
    } else {
        wrap_with_preview_quote_guides(combined, effective_quote_depth, theme)
    }
}

/// Computes the centered content column width for the preview, mirroring the
/// WYSIWYG centered-column layout (no quote/callout insets since preview
/// depths are structural).
pub(crate) fn preview_centered_column_width(
    viewport_width: f32,
    d: &theme::ThemeDimensions,
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
    (available_content_width * ratio)
        .max(320.0)
        .min(available_content_width)
}

/// Collects every footnote definition block in the preview tree in document
/// order — including definitions nested inside quotes, callouts, or lists — so
/// the preview can render them in a single bottom section.
pub(crate) fn collect_preview_footnote_definitions(
    roots: &[PreviewBlock],
    out: &mut Vec<PreviewBlock>,
) {
    for block in roots {
        if block.kind() == BlockKind::FootnoteDefinition {
            out.push(block.clone());
        }
        collect_preview_footnote_definitions(&block.children, out);
    }
}

/// Wraps content with the quote guide lines at the given depth, mirroring
/// the WYSIWYG quote guides.
pub(crate) fn wrap_with_preview_quote_guides(
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
                .left(px(d.block_padding_x + guide_offset * level as f32))
                .w(px(d.quote_border_width))
                .bg(c.border_quote)
        }))
        .into_any_element()
}

