//! Standard pane layout scaffold helper.
//!
//! Provides a standardized layout for autonomous pane plugins:
//! - Top: Breadcrumb / topbar
//! - Middle: [ Left: Main Content (flex-1) | Right: Docked Outline (parallel) | Far-right: Vertical Scrollbar ]
//! - Bottom: Horizontal Scrollbar

use crate::pane::PaneId;
use gpui::{AnyElement, InteractiveElement, IntoElement, ParentElement, Styled, div, px};

/// Assembles a pane's visual tree using parallel non-floating columns.
pub fn render_pane_layout(
    pane_id: PaneId,
    top_bar: Option<AnyElement>,
    content: AnyElement,
    docked_outline: Option<AnyElement>,
    v_scrollbar: AnyElement,
    h_scrollbar: AnyElement,
) -> AnyElement {
    div()
        .id(("pane-layout-frame", pane_id.as_usize()))
        .w_full()
        .h_full()
        .flex()
        .flex_col()
        .relative()
        .children(top_bar)
        .child(
            div()
                .w_full()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_row()
                .relative()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h_full()
                        .relative()
                        .child(content),
                )
                .children(docked_outline)
                .child(v_scrollbar),
        )
        .child(h_scrollbar)
        .into_any_element()
}
