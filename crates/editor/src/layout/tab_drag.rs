use std::sync::{Arc, Mutex};
use gpui::*;
use platform_contracts::PanelId;
use editor_contracts::DocumentId;
use splitter::Direction;
use theme::Theme;
use ui::split::{ActionHint, OverlayStyle};

/// Target region for tab-drag docking, dividing the surface into
/// a center merge zone, 4 inner pane quadrants, and 4 outer editor quadrants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabDockTarget {
    /// Merges tab into the target editor's tab bar without splitting.
    MergeCenter,
    /// Splits an inner pane in the current editor along the cardinal direction.
    InnerPane(Direction),
    /// Splits a new editor panel along the cardinal direction.
    OuterEditor(Direction),
}

/// Calculates the tab dock target from normalized relative coordinates [0.0..=1.0].
///
/// Follows the nested quadrants geometry:
/// - (0.40..=0.60, 0.40..=0.60) is the center merge zone.
/// - The remainder of the inner box (0.25..=0.75, 0.25..=0.75) is divided by diagonals into 4 inner pane triangles.
/// - Outside the inner box is divided by diagonals into 4 outer editor trapezoids.
pub fn calc_tab_dock_target(rel_x: f32, rel_y: f32) -> TabDockTarget {
    let u = rel_x.clamp(0.0, 1.0);
    let v = rel_y.clamp(0.0, 1.0);

    // 1. Center merge zone (0.40..=0.60)
    if (0.40..=0.60).contains(&u) && (0.40..=0.60).contains(&v) {
        return TabDockTarget::MergeCenter;
    }

    // 2. Cardinal quadrant from diagonals:
    let dir = if v <= u && v <= (1.0 - u) {
        Direction::Up
    } else if v >= u && v >= (1.0 - u) {
        Direction::Down
    } else if u <= v && u <= (1.0 - v) {
        Direction::Left
    } else {
        Direction::Right
    };

    // 3. Inner vs Outer boundary (0.25..=0.75)
    let in_inner_box = (0.25..=0.75).contains(&u) && (0.25..=0.75).contains(&v);
    if in_inner_box {
        TabDockTarget::InnerPane(dir)
    } else {
        TabDockTarget::OuterEditor(dir)
    }
}

/// Hover information describing the active docking partition and modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabDragHoverInfo {
    pub target: TabDockTarget,
    pub shift_held: bool,
}

/// Data payload of an in-flight dragged document tab.
#[derive(Clone)]
pub struct DraggedTab {
    pub source_panel_id: PanelId,
    pub source_tab_index: usize,
    pub document_id: DocumentId,
    pub title: SharedString,
    pub drag_view: Entity<DraggedTabView>,
    pub active_hover_editor: Arc<Mutex<Option<WeakEntity<crate::editor::Editor>>>>,
}

/// Active drag hover state recorded by an Editor panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabDragHoverState {
    pub target: TabDockTarget,
    pub shift_held: bool,
    pub pointer_pos: Point<Pixels>,
}

/// Floating ghost card preview following the mouse during tab drag.
/// Spliced into a unified card when hovering over an editor recognition partition.
pub struct DraggedTabView {
    pub title: SharedString,
    pub hover: Option<TabDragHoverInfo>,
}

impl DraggedTabView {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            hover: None,
        }
    }

    pub fn set_hover(&mut self, hover: Option<TabDragHoverInfo>) {
        self.hover = hover;
    }
}

impl Render for DraggedTabView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let style = OverlayStyle::from_theme(&theme);

        if let Some(hover) = self.hover {
            // Spliced unified card combining tab title and split/swap cursor action panel
            let (icon_path, title, detail, hints): (
                Option<&'static str>,
                &'static str,
                Option<&'static str>,
                &[ActionHint],
            ) = match hover.target {
                TabDockTarget::MergeCenter => (
                    Some("icons/splitter/swap.svg"),
                    "Merge into Tabs",
                    None,
                    &[ActionHint { key: "Esc", desc: "Cancel" }],
                ),
                TabDockTarget::InnerPane(dir) => {
                    let dir_str = match dir {
                        Direction::Up => "Top",
                        Direction::Down => "Bottom",
                        Direction::Left => "Left",
                        Direction::Right => "Right",
                    };
                    (
                        Some("icons/splitter/split-area.svg"),
                        "Split Pane",
                        Some(dir_str),
                        &[ActionHint { key: "Esc", desc: "Cancel" }],
                    )
                }
                TabDockTarget::OuterEditor(dir) => {
                    let dir_str = match dir {
                        Direction::Up => "Top",
                        Direction::Down => "Bottom",
                        Direction::Left => "Left",
                        Direction::Right => "Right",
                    };
                    let icon = match dir {
                        Direction::Up => Some("icons/splitter/dock-up.svg"),
                        Direction::Down => Some("icons/splitter/dock-down.svg"),
                        Direction::Left => Some("icons/splitter/dock-left.svg"),
                        Direction::Right => Some("icons/splitter/dock-right.svg"),
                    };
                    let title = if hover.shift_held {
                        "Duplicate Editor"
                    } else {
                        "Move to New Editor"
                    };
                    let hints = if hover.shift_held {
                        &[ActionHint { key: "Esc", desc: "Cancel" }][..]
                    } else {
                        &[
                            ActionHint { key: "Shift", desc: "Clone" },
                            ActionHint { key: "Esc", desc: "Cancel" },
                        ][..]
                    };
                    (icon, title, Some(dir_str), hints)
                }
            };

            let mut hint_elements = Vec::new();
            for (i, hint) in hints.iter().enumerate() {
                if i > 0 {
                    hint_elements.push(
                        div()
                            .text_size(px(10.0))
                            .text_color(style.text.opacity(0.30))
                            .child("•"),
                    );
                }
                hint_elements.push(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            // Key badge (黑体字, bold, tactile keycap style matching split/swap panel)
                            div()
                                .px(px(5.0))
                                .py(px(1.5))
                                .rounded(px(2.0))
                                .bg(style.hover)
                                .text_size(px(10.5))
                                .font_weight(FontWeight::BOLD)
                                .text_color(style.text)
                                .child(hint.key),
                        )
                        .child(
                            // Description (灰体字, muted, clean matching split/swap panel)
                            div()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(style.text.opacity(0.60))
                                .child(hint.desc),
                        ),
                );
            }

            div()
                .flex()
                .flex_col()
                .rounded(px(2.0))
                .bg(style.surface)
                .shadow_lg()
                .overflow_hidden()
                // Top header: The dragged tab title seamlessly spliced (rounded 2px)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .px(px(14.0))
                        .py(px(6.0))
                        .bg(style.hover)
                        .border_b(px(1.0))
                        .border_color(style.border.opacity(0.40))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(style.text)
                                .child(self.title.clone()),
                        ),
                )
                // Bottom content: matches split panel and swap panel (cursor_action_panel) design
                .child(
                    div()
                        .px(px(14.0))
                        .py(px(9.0))
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(7.0))
                                .children(icon_path.map(|p| svg().path(p).size(px(15.0)).text_color(style.accent)))
                                .child(
                                    // Title: Bold primary text
                                    div()
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(style.text)
                                        .child(title),
                                )
                                .children(detail.map(|d| {
                                    div()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(style.accent)
                                        .child(d)
                                })),
                        )
                        .children(
                            if !hint_elements.is_empty() {
                                Some(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(7.0))
                                        .children(hint_elements),
                                )
                            } else {
                                None
                            },
                        ),
                )
        } else {
            // Standalone tab pill (during normal drag before entering recognition zones)
            div()
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(2.0))
                .bg(style.surface.opacity(0.95))
                .shadow_lg()
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(style.text)
                .child(self.title.clone())
        }
    }
}

/// Renders the 8-quadrant surface wireframe and highlights the active drop zone
/// as true geometric trapezoids and center rectangle. No text or badges are drawn
/// on the overlay surface (all action text is in the floating cursor panel).
pub fn render_tab_drag_compass(
    hover: &TabDragHoverState,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let accent = c.split_indicator;
    let target = hover.target;

    // Neutral gray for inactive partitions clearly visible on both light and dark themes
    let inactive_fill = c.dialog_muted.opacity(0.14);
    let inactive_stroke = c.dialog_muted.opacity(0.55);
    // Vibrant glowing blue for active partition
    let active_fill = accent.opacity(0.35);
    let active_stroke = accent.opacity(0.95);

    // Full-surface canvas rendering the 9 actual recognition zones:
    // 4 outer trapezoids (Editor), 4 inner trapezoids (Pane), and 1 center rectangle (Merge)
    let canvas_element = canvas(
        move |_bounds, _window, _cx| (),
        move |bounds, (), window, _cx| {
            let ox = f32::from(bounds.origin.x);
            let oy = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);

            if w <= 0.0 || h <= 0.0 {
                return;
            }

            // Coordinates: Outer bounds (0.0 ..= 1.0)
            let x0 = ox;
            let y0 = oy;
            let x1 = ox + w;
            let y1 = oy + h;

            // Coordinates: Inner bounds (0.25 ..= 0.75)
            let ix0 = ox + w * 0.25;
            let iy0 = oy + h * 0.25;
            let ix1 = ox + w * 0.75;
            let iy1 = oy + h * 0.75;

            // Coordinates: Center box (0.40 ..= 0.60)
            let mx0 = ox + w * 0.40;
            let my0 = oy + h * 0.40;
            let mx1 = ox + w * 0.60;
            let my1 = oy + h * 0.60;

            let fill_poly = |pts: &[(f32, f32)], color: Hsla, window: &mut Window| {
                if pts.len() < 3 {
                    return;
                }
                let mut builder = PathBuilder::fill();
                builder.move_to(point(px(pts[0].0), px(pts[0].1)));
                for pt in &pts[1..] {
                    builder.line_to(point(px(pt.0), px(pt.1)));
                }
                builder.close();
                if let Ok(path) = builder.build() {
                    window.paint_path(path, color);
                }
            };

            let stroke_poly = |pts: &[(f32, f32)], stroke_color: Hsla, width: f32, window: &mut Window| {
                if pts.len() < 2 {
                    return;
                }
                let mut builder = PathBuilder::stroke(px(width));
                builder.move_to(point(px(pts[0].0), px(pts[0].1)));
                for pt in &pts[1..] {
                    builder.line_to(point(px(pt.0), px(pt.1)));
                }
                builder.close();
                if let Ok(path) = builder.build() {
                    window.paint_path(path, stroke_color);
                }
            };

            // The 9 exact recognition zones:
            let zones: [(TabDockTarget, &[(f32, f32)]); 9] = [
                // Outer 4 Editor zones (actual outer trapezoids)
                (TabDockTarget::OuterEditor(Direction::Up), &[(x0, y0), (x1, y0), (ix1, iy0), (ix0, iy0)]),
                (TabDockTarget::OuterEditor(Direction::Down), &[(ix0, iy1), (ix1, iy1), (x1, y1), (x0, y1)]),
                (TabDockTarget::OuterEditor(Direction::Left), &[(x0, y0), (ix0, iy0), (ix0, iy1), (x0, y1)]),
                (TabDockTarget::OuterEditor(Direction::Right), &[(ix1, iy0), (x1, y0), (x1, y1), (ix1, iy1)]),
                // Inner 4 Pane zones (actual inner trapezoids)
                (TabDockTarget::InnerPane(Direction::Up), &[(ix0, iy0), (ix1, iy0), (mx1, my0), (mx0, my0)]),
                (TabDockTarget::InnerPane(Direction::Down), &[(ix0, iy1), (mx0, my1), (mx1, my1), (ix1, iy1)]),
                (TabDockTarget::InnerPane(Direction::Left), &[(ix0, iy0), (mx0, my0), (mx0, my1), (ix0, iy1)]),
                (TabDockTarget::InnerPane(Direction::Right), &[(mx1, my0), (ix1, iy0), (ix1, iy1), (mx1, my1)]),
                // Center Merge zone (actual center rectangle)
                (TabDockTarget::MergeCenter, &[(mx0, my0), (mx1, my0), (mx1, my1), (mx0, my1)]),
            ];

            // 1. Draw all inactive recognition zones in neutral gray
            for (zone_target, pts) in &zones {
                if *zone_target != target {
                    fill_poly(pts, inactive_fill, window);
                    stroke_poly(pts, inactive_stroke, 1.5, window);
                }
            }

            // 2. Draw the active recognition zone in vibrant blue with a prominent 3px glowing border
            for (zone_target, pts) in &zones {
                if *zone_target == target {
                    fill_poly(pts, active_fill, window);
                    stroke_poly(pts, active_stroke, 3.0, window);
                }
            }
        },
    )
    .size_full()
    .absolute()
    .inset_0();

    div()
        .absolute()
        .inset_0()
        .child(canvas_element)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[core::prelude::v1::test]
    fn test_calc_tab_dock_target_center() {
        assert_eq!(calc_tab_dock_target(0.5, 0.5), TabDockTarget::MergeCenter);
        assert_eq!(calc_tab_dock_target(0.45, 0.45), TabDockTarget::MergeCenter);
        assert_eq!(calc_tab_dock_target(0.55, 0.55), TabDockTarget::MergeCenter);
    }

    #[core::prelude::v1::test]
    fn test_calc_tab_dock_target_inner() {
        assert_eq!(calc_tab_dock_target(0.5, 0.3), TabDockTarget::InnerPane(Direction::Up));
        assert_eq!(calc_tab_dock_target(0.5, 0.7), TabDockTarget::InnerPane(Direction::Down));
        assert_eq!(calc_tab_dock_target(0.3, 0.5), TabDockTarget::InnerPane(Direction::Left));
        assert_eq!(calc_tab_dock_target(0.7, 0.5), TabDockTarget::InnerPane(Direction::Right));
    }

    #[core::prelude::v1::test]
    fn test_calc_tab_dock_target_outer() {
        assert_eq!(calc_tab_dock_target(0.5, 0.1), TabDockTarget::OuterEditor(Direction::Up));
        assert_eq!(calc_tab_dock_target(0.5, 0.9), TabDockTarget::OuterEditor(Direction::Down));
        assert_eq!(calc_tab_dock_target(0.1, 0.5), TabDockTarget::OuterEditor(Direction::Left));
        assert_eq!(calc_tab_dock_target(0.9, 0.5), TabDockTarget::OuterEditor(Direction::Right));
    }
}
