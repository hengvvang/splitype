//! Preview footnote definition rendering — real-id text plus content.

use gpui::*;

use crate::editor::panes::preview::node::PreviewBlock;
use crate::infra::theme::Theme;

/// Renders a footnote definition block read-only.
pub(crate) fn render_preview_footnote_definition(
    block: &PreviewBlock,
    _depth: usize,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let plain_text = block.data.text.plain_text();
    let (id, content) =
        crate::model::block::footnote::split_footnote_definition_text(&plain_text);
    let mut header = base
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(d.list_marker_gap))
        .text_size(px(t.code_size))
        .text_color(c.text_default)
        .child(
            div()
                .min_w(px(0.0))
                .flex_grow(1.0)
                .flex()
                .flex_row()
                .flex_wrap()
                .items_baseline()
                .child(
                    div()
                        .text_color(c.footnote_backref)
                        .child(id.to_string()),
                )
                .child(
                    div()
                        .text_color(c.text_default)
                        .child(format!(": {}", content)),
                ),
        );

    if block.has_footnote_definition_backref() {
        header = header.child(
            div()
                .text_color(c.footnote_backref)
                .hover(|this| this.underline().text_color(c.text_link))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    cx.stop_propagation();
                })
                .child("\u{21A9}"),
        );
    }

    header.into_any_element()
}

use crate::editor::engine::controller::{Editor, PaneId};

/// Renders the collected GitHub-style footnotes section: a top divider line
/// followed by every footnote definition in document order.
pub(crate) fn render_preview_footnotes_section(
    footnotes: &[PreviewBlock],
    pane_id: PaneId,
    editor_handle: &Entity<Editor>,
    theme: &Theme,
    window: &mut Window,
    cx: &App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let rows: Vec<AnyElement> = footnotes
        .iter()
        .enumerate()
        .map(|(idx, block)| {
            super::render_preview_block(
                block,
                idx,
                None,
                0,
                0,
                pane_id,
                editor_handle,
                theme,
                window,
                cx,
            )
        })
        .collect();

    div()
        .w_full()
        .flex_shrink_0()
        .mt(px(d.block_gap * 2.0))
        .pt(px(8.0))
        .border_t(px(1.0))
        .border_color(c.footnote_border)
        .children(rows)
        .into_any_element()
}
