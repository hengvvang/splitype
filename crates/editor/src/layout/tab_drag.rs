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

/// Normalized coordinates for ergonomic tab docking layout based on the Golden Ratio (φ ≈ 1.618):
/// - Outer editor perimeter: 19.1% golden margin along all edges (0.19), creating prominent, balanced outer zones
/// - Middle inner pane box: 61.8% golden section (0.19..=0.81)
/// - Center merge zone: 30% wide (0.35..=0.65) by 28% tall (0.36..=0.64), perfectly aligned with perspective diagonal vectors
/// - Inner pane split zones: 16% horizontal / 17% vertical depth, creating a beautifully continuous 3D frustum perspective
pub const DOCK_OUTER_MARGIN: f32 = 0.19;
pub const DOCK_CENTER_X_MIN: f32 = 0.35;
pub const DOCK_CENTER_X_MAX: f32 = 0.65;
pub const DOCK_CENTER_Y_MIN: f32 = 0.36;
pub const DOCK_CENTER_Y_MAX: f32 = 0.64;

/// Helper to test if a 2D point lies inside a convex quadrilateral.
fn point_in_convex_quad(px: f32, py: f32, quad: &[(f32, f32); 4]) -> bool {
    let mut sign = None;
    for i in 0..4 {
        let (x1, y1) = quad[i];
        let (x2, y2) = quad[(i + 1) % 4];
        let cross = (x2 - x1) * (py - y1) - (y2 - y1) * (px - x1);
        if cross.abs() < 1e-5 {
            continue;
        }
        let cross_sign = cross > 0.0;
        match sign {
            None => sign = Some(cross_sign),
            Some(s) if s != cross_sign => return false,
            _ => {}
        }
    }
    true
}

/// Calculates the tab dock target from normalized relative coordinates [0.0..=1.0].
///
/// Follows the ergonomic trapezoid partition geometry:
/// - Center merge zone: 40% wide x 30% tall rectangular target (0.30..=0.70, 0.35..=0.65).
/// - Inner pane split zones: 4 wide directional trapezoids between outer edge and center.
/// - Outer editor split zones: 14% perimeter edge snap trapezoids.
pub fn calc_tab_dock_target(rel_x: f32, rel_y: f32) -> TabDockTarget {
    let u = rel_x.clamp(0.0, 1.0);
    let v = rel_y.clamp(0.0, 1.0);

    // 1. Center merge zone (generous 40% wide x 30% tall ergonomic target)
    if (DOCK_CENTER_X_MIN..=DOCK_CENTER_X_MAX).contains(&u)
        && (DOCK_CENTER_Y_MIN..=DOCK_CENTER_Y_MAX).contains(&v)
    {
        return TabDockTarget::MergeCenter;
    }

    let ix0 = DOCK_OUTER_MARGIN;
    let iy0 = DOCK_OUTER_MARGIN;
    let ix1 = 1.0 - DOCK_OUTER_MARGIN;
    let iy1 = 1.0 - DOCK_OUTER_MARGIN;

    let mx0 = DOCK_CENTER_X_MIN;
    let my0 = DOCK_CENTER_Y_MIN;
    let mx1 = DOCK_CENTER_X_MAX;
    let my1 = DOCK_CENTER_Y_MAX;

    // 2. Inner pane quadrants (comfortable 16% horizontal / 21% vertical depth)
    let inner_up = [(ix0, iy0), (ix1, iy0), (mx1, my0), (mx0, my0)];
    let inner_down = [(ix0, iy1), (mx0, my1), (mx1, my1), (ix1, iy1)];
    let inner_left = [(ix0, iy0), (mx0, my0), (mx0, my1), (ix0, iy1)];
    let inner_right = [(mx1, my0), (ix1, iy0), (ix1, iy1), (mx1, my1)];

    if point_in_convex_quad(u, v, &inner_up) {
        return TabDockTarget::InnerPane(Direction::Up);
    }
    if point_in_convex_quad(u, v, &inner_down) {
        return TabDockTarget::InnerPane(Direction::Down);
    }
    if point_in_convex_quad(u, v, &inner_left) {
        return TabDockTarget::InnerPane(Direction::Left);
    }
    if point_in_convex_quad(u, v, &inner_right) {
        return TabDockTarget::InnerPane(Direction::Right);
    }

    // 3. Outer editor perimeter (slender 14% edge snap zones)
    let outer_up = [(0.0, 0.0), (1.0, 0.0), (ix1, iy0), (ix0, iy0)];
    let outer_down = [(ix0, iy1), (ix1, iy1), (1.0, 1.0), (0.0, 1.0)];
    let outer_left = [(0.0, 0.0), (ix0, iy0), (ix0, iy1), (0.0, 1.0)];

    if point_in_convex_quad(u, v, &outer_up) {
        return TabDockTarget::OuterEditor(Direction::Up);
    }
    if point_in_convex_quad(u, v, &outer_down) {
        return TabDockTarget::OuterEditor(Direction::Down);
    }
    if point_in_convex_quad(u, v, &outer_left) {
        return TabDockTarget::OuterEditor(Direction::Left);
    }

    TabDockTarget::OuterEditor(Direction::Right)
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

        // Faint black background close to pure black, with crisp light text
        let top_bg = hsla(0.0, 0.0, 0.09, 0.96);
        let top_text = hsla(0.0, 0.0, 0.94, 1.0);
        let top_border = hsla(0.0, 0.0, 1.0, 0.12);

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

            // Calculate content width so top and bottom panels are guaranteed to be strictly equal in width
            let action_title_w = estimate_text_width(title, 13.0);
            let action_detail_w = detail
                .map(|d| 7.0 + estimate_text_width(d, 12.5))
                .unwrap_or(0.0);
            let action_row_w = 15.0 + 7.0 + action_title_w + action_detail_w + 28.0;

            let mut hints_content_w = 0.0;
            for (i, hint) in hints.iter().enumerate() {
                if i > 0 {
                    hints_content_w += 16.0;
                }
                let key_w = estimate_text_width(hint.key, 10.5) + 10.0;
                let desc_w = estimate_text_width(hint.desc, 11.0);
                hints_content_w += key_w + 4.0 + desc_w;
            }
            let hints_row_w = if hints.is_empty() {
                0.0
            } else {
                hints_content_w + 28.0
            };

            let tab_title_w = estimate_text_width(&self.title, 11.5) + 24.0;
            let needed_w = action_row_w.max(hints_row_w).max(tab_title_w);
            let card_w = needed_w.clamp(210.0, 320.0).ceil();

            div()
                .w(px(card_w))
                .flex()
                .flex_col()
                .items_stretch()
                .gap(px(2.0))
                // Top panel: Independent tab title card (4px rounded, faint black bg, strictly equal width, truncated)
                .child(
                    div()
                        .w(px(card_w))
                        .px(px(12.0))
                        .py(px(4.5))
                        .rounded(px(4.0))
                        .bg(top_bg)
                        .border(px(1.0))
                        .border_color(top_border)
                        .shadow_md()
                        .flex()
                        .items_center()
                        .overflow_hidden()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .truncate()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(top_text)
                                .child(self.title.clone()),
                        ),
                )
                // Bottom panel: Independent action card matching split panel and swap panel design (4px rounded, strictly equal width, compact)
                .child({
                    let mut action_card = div()
                        .w(px(card_w))
                        .px(px(14.0))
                        .py(px(8.0))
                        .rounded(px(4.0))
                        .bg(style.surface)
                        .border(px(1.0))
                        .border_color(style.border)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(5.0))
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
                        );

                    if !hint_elements.is_empty() {
                        action_card = action_card.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .children(hint_elements),
                        );
                    }

                    action_card
                })
        } else {
            // Standalone tab pill (during normal drag before entering recognition zones)
            div()
                .max_w(px(260.0))
                .px(px(12.0))
                .py(px(4.5))
                .rounded(px(4.0))
                .bg(top_bg)
                .border(px(1.0))
                .border_color(top_border)
                .shadow_md()
                .flex()
                .items_center()
                .overflow_hidden()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(top_text)
                        .child(self.title.clone()),
                )
        }
    }
}

/// Helper to estimate rendered text width for equal-width panel sizing
fn estimate_text_width(text: &str, font_size: f32) -> f32 {
    text.chars()
        .map(|ch| {
            if ch.is_ascii_whitespace() {
                font_size * 0.35
            } else if ch.is_ascii_punctuation() {
                font_size * 0.45
            } else if ch.is_ascii_uppercase() {
                font_size * 0.68
            } else if ch.is_ascii() {
                font_size * 0.56
            } else if ch as u32 > 0x2E80 {
                font_size * 1.0
            } else {
                font_size * 0.85
            }
        })
        .sum()
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

    // Line thickness is uniform for all lines (do not increase width on active)
    let line_width = 1.5;

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

            // Coordinates: Inner bounds (DOCK_OUTER_MARGIN ..= 1.0 - DOCK_OUTER_MARGIN)
            let ix0 = ox + w * DOCK_OUTER_MARGIN;
            let iy0 = oy + h * DOCK_OUTER_MARGIN;
            let ix1 = ox + w * (1.0 - DOCK_OUTER_MARGIN);
            let iy1 = oy + h * (1.0 - DOCK_OUTER_MARGIN);

            // Coordinates: Center box (DOCK_CENTER_X_MIN..DOCK_CENTER_X_MAX, DOCK_CENTER_Y_MIN..DOCK_CENTER_Y_MAX)
            let mx0 = ox + w * DOCK_CENTER_X_MIN;
            let my0 = oy + h * DOCK_CENTER_Y_MIN;
            let mx1 = ox + w * DOCK_CENTER_X_MAX;
            let my1 = oy + h * DOCK_CENTER_Y_MAX;

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
                // Outer 4 Editor zones (19% golden perimeter edge snap trapezoids)
                (TabDockTarget::OuterEditor(Direction::Up), &[(x0, y0), (x1, y0), (ix1, iy0), (ix0, iy0)]),
                (TabDockTarget::OuterEditor(Direction::Down), &[(ix0, iy1), (ix1, iy1), (x1, y1), (x0, y1)]),
                (TabDockTarget::OuterEditor(Direction::Left), &[(x0, y0), (ix0, iy0), (ix0, iy1), (x0, y1)]),
                (TabDockTarget::OuterEditor(Direction::Right), &[(ix1, iy0), (x1, y0), (x1, y1), (ix1, iy1)]),
                // Inner 4 Pane zones (comfortable directional trapezoids)
                (TabDockTarget::InnerPane(Direction::Up), &[(ix0, iy0), (ix1, iy0), (mx1, my0), (mx0, my0)]),
                (TabDockTarget::InnerPane(Direction::Down), &[(ix0, iy1), (mx0, my1), (mx1, my1), (ix1, iy1)]),
                (TabDockTarget::InnerPane(Direction::Left), &[(ix0, iy0), (mx0, my0), (mx0, my1), (ix0, iy1)]),
                (TabDockTarget::InnerPane(Direction::Right), &[(mx1, my0), (ix1, iy0), (ix1, iy1), (mx1, my1)]),
                // Center Merge zone (30% x 28% golden center rectangle)
                (TabDockTarget::MergeCenter, &[(mx0, my0), (mx1, my0), (mx1, my1), (mx0, my1)]),
            ];

            // 1. Draw all inactive recognition zone fills in neutral gray
            for (zone_target, pts) in &zones {
                if *zone_target != target {
                    fill_poly(pts, inactive_fill, window);
                }
            }

            // 2. Draw active recognition zone fill in vivid blue
            for (zone_target, pts) in &zones {
                if *zone_target == target {
                    fill_poly(pts, active_fill, window);
                }
            }

            // 3. Draw inactive boundary strokes in neutral gray (line_width = 1.5)
            for (zone_target, pts) in &zones {
                if *zone_target != target {
                    stroke_poly(pts, inactive_stroke, line_width, window);
                }
            }

            // 4. Draw active boundary strokes in vivid blue without increasing width (line_width = 1.5)
            for (zone_target, pts) in &zones {
                if *zone_target == target {
                    stroke_poly(pts, active_stroke, line_width, window);
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
        assert_eq!(calc_tab_dock_target(0.38, 0.5), TabDockTarget::MergeCenter);
        assert_eq!(calc_tab_dock_target(0.62, 0.5), TabDockTarget::MergeCenter);
    }

    #[core::prelude::v1::test]
    fn test_calc_tab_dock_target_inner() {
        assert_eq!(calc_tab_dock_target(0.5, 0.25), TabDockTarget::InnerPane(Direction::Up));
        assert_eq!(calc_tab_dock_target(0.5, 0.75), TabDockTarget::InnerPane(Direction::Down));
        assert_eq!(calc_tab_dock_target(0.26, 0.5), TabDockTarget::InnerPane(Direction::Left));
        assert_eq!(calc_tab_dock_target(0.74, 0.5), TabDockTarget::InnerPane(Direction::Right));
    }

    #[core::prelude::v1::test]
    fn test_calc_tab_dock_target_outer() {
        assert_eq!(calc_tab_dock_target(0.5, 0.10), TabDockTarget::OuterEditor(Direction::Up));
        assert_eq!(calc_tab_dock_target(0.5, 0.90), TabDockTarget::OuterEditor(Direction::Down));
        assert_eq!(calc_tab_dock_target(0.10, 0.5), TabDockTarget::OuterEditor(Direction::Left));
        assert_eq!(calc_tab_dock_target(0.90, 0.5), TabDockTarget::OuterEditor(Direction::Right));
    }
}
