//! Tab drag and drop domain model, geometry hit testing, ghost pill rendering,
//! and 8-quadrant compass overlay with split preview shadow.

use gpui::*;
use platform_contracts::PanelId;
use editor_contracts::DocumentId;
use splitter::Direction;
use theme::Theme;

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

/// Data payload of an in-flight dragged document tab.
#[derive(Clone, Debug)]
pub struct DraggedTab {
    pub source_panel_id: PanelId,
    pub source_tab_index: usize,
    pub document_id: DocumentId,
    pub title: SharedString,
}

use ui::split::{ActionHint, OverlayStyle, cursor_action_panel};

/// Active drag hover state recorded by an Editor panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabDragHoverState {
    pub target: TabDockTarget,
    pub shift_held: bool,
    pub pointer_pos: Point<Pixels>,
}

/// Floating ghost pill preview following the mouse during tab drag.
pub struct DraggedTabView {
    pub title: SharedString,
}

impl DraggedTabView {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl Render for DraggedTabView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let c = &theme.colors;
        let d = &theme.dimensions;
        div()
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(d.tab_radius))
            .bg(c.dialog_surface.opacity(0.95))
            .border(px(1.0))
            .border_color(c.focus_accent)
            .shadow_lg()
            .text_size(px(11.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c.text_default)
            .child(self.title.clone())
    }
}

/// Renders the 8-quadrant surface wireframe, highlights the active drop zone,
/// and displays the cursor-following action card matching split/swap window design.
pub fn render_tab_drag_compass(
    hover: &TabDragHoverState,
    container_size: Size<Pixels>,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let accent = c.split_indicator;
    let border_color = c.dialog_border;
    let overlay_style = OverlayStyle::from_theme(theme);

    // 1. 50% Split Shadow
    let shadow = match hover.target {
        TabDockTarget::InnerPane(dir) | TabDockTarget::OuterEditor(dir) => {
            let base = div()
                .absolute()
                .bg(accent.opacity(0.16))
                .border(px(1.5))
                .border_color(accent);
            match dir {
                Direction::Left => Some(base.left_0().top_0().bottom_0().w_1_2().rounded_l(px(d.panel_tile_radius))),
                Direction::Right => Some(base.right_0().top_0().bottom_0().w_1_2().rounded_r(px(d.panel_tile_radius))),
                Direction::Up => Some(base.left_0().right_0().top_0().h_1_2().rounded_t(px(d.panel_tile_radius))),
                Direction::Down => Some(base.left_0().right_0().bottom_0().h_1_2().rounded_b(px(d.panel_tile_radius))),
            }
        }
        TabDockTarget::MergeCenter => None,
    };

    // 2. Cursor-following action panel content
    let (icon_path, title, detail, hints): (
        Option<&'static str>,
        &'static str,
        Option<String>,
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
                Some(dir_str.to_string()),
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
            (icon, title, Some(dir_str.to_string()), hints)
        }
    };

    let target = hover.target;

    // 3. Full-surface wireframe grid and active zone highlight
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

            // Coordinates: Outer rect
            let x0 = ox;
            let y0 = oy;
            let x1 = ox + w;
            let y1 = oy + h;

            // Coordinates: Inner rect (0.25..=0.75)
            let ix0 = ox + w * 0.25;
            let iy0 = oy + h * 0.25;
            let ix1 = ox + w * 0.75;
            let iy1 = oy + h * 0.75;

            // Coordinates: Center box (0.40..=0.60)
            let mx0 = ox + w * 0.40;
            let my0 = oy + h * 0.40;
            let mx1 = ox + w * 0.60;
            let my1 = oy + h * 0.60;

            let fill_color = accent.opacity(0.22);
            let highlight_stroke_color = accent.opacity(0.85);
            let guide_stroke_color = border_color.opacity(0.35);

            let fill_poly = |pts: &[(f32, f32)], window: &mut Window| {
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
                    window.paint_path(path, fill_color);
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

            let draw_line = |p1: (f32, f32), p2: (f32, f32), stroke_color: Hsla, width: f32, window: &mut Window| {
                let mut builder = PathBuilder::stroke(px(width));
                builder.move_to(point(px(p1.0), px(p1.1)));
                builder.line_to(point(px(p2.0), px(p2.1)));
                if let Ok(path) = builder.build() {
                    window.paint_path(path, stroke_color);
                }
            };

            // 1. Draw base wireframe grid
            // Outer rect
            draw_line((x0, y0), (x1, y0), guide_stroke_color, 1.0, window);
            draw_line((x1, y0), (x1, y1), guide_stroke_color, 1.0, window);
            draw_line((x1, y1), (x0, y1), guide_stroke_color, 1.0, window);
            draw_line((x0, y1), (x0, y0), guide_stroke_color, 1.0, window);

            // Inner rect
            draw_line((ix0, iy0), (ix1, iy0), guide_stroke_color, 1.0, window);
            draw_line((ix1, iy0), (ix1, iy1), guide_stroke_color, 1.0, window);
            draw_line((ix1, iy1), (ix0, iy1), guide_stroke_color, 1.0, window);
            draw_line((ix0, iy1), (ix0, iy0), guide_stroke_color, 1.0, window);

            // Center rect
            draw_line((mx0, my0), (mx1, my0), guide_stroke_color, 1.0, window);
            draw_line((mx1, my0), (mx1, my1), guide_stroke_color, 1.0, window);
            draw_line((mx1, my1), (mx0, my1), guide_stroke_color, 1.0, window);
            draw_line((mx0, my1), (mx0, my0), guide_stroke_color, 1.0, window);

            // Diagonals (outer to inner)
            draw_line((x0, y0), (ix0, iy0), guide_stroke_color, 1.0, window);
            draw_line((x1, y0), (ix1, iy0), guide_stroke_color, 1.0, window);
            draw_line((x1, y1), (ix1, iy1), guide_stroke_color, 1.0, window);
            draw_line((x0, y1), (ix0, iy1), guide_stroke_color, 1.0, window);

            // Diagonals (inner to center)
            draw_line((ix0, iy0), (mx0, my0), guide_stroke_color, 1.0, window);
            draw_line((ix1, iy0), (mx1, my0), guide_stroke_color, 1.0, window);
            draw_line((ix1, iy1), (mx1, my1), guide_stroke_color, 1.0, window);
            draw_line((ix0, iy1), (mx0, my1), guide_stroke_color, 1.0, window);

            // 2. Active Zone Highlight (fill + 2px glow border)
            let active_poly: Option<&[(f32, f32)]> = match target {
                TabDockTarget::MergeCenter => Some(&[(mx0, my0), (mx1, my0), (mx1, my1), (mx0, my1)]),
                TabDockTarget::InnerPane(Direction::Up) => Some(&[(ix0, iy0), (ix1, iy0), (mx1, my0), (mx0, my0)]),
                TabDockTarget::InnerPane(Direction::Down) => Some(&[(ix0, iy1), (mx0, my1), (mx1, my1), (ix1, iy1)]),
                TabDockTarget::InnerPane(Direction::Left) => Some(&[(ix0, iy0), (mx0, my0), (mx0, my1), (ix0, iy1)]),
                TabDockTarget::InnerPane(Direction::Right) => Some(&[(mx1, my0), (ix1, iy0), (ix1, iy1), (mx1, my1)]),
                TabDockTarget::OuterEditor(Direction::Up) => Some(&[(x0, y0), (x1, y0), (ix1, iy0), (ix0, iy0)]),
                TabDockTarget::OuterEditor(Direction::Down) => Some(&[(ix0, iy1), (ix1, iy1), (x1, y1), (x0, y1)]),
                TabDockTarget::OuterEditor(Direction::Left) => Some(&[(x0, y0), (ix0, iy0), (ix0, iy1), (x0, y1)]),
                TabDockTarget::OuterEditor(Direction::Right) => Some(&[(ix1, iy0), (x1, y0), (x1, y1), (ix1, iy1)]),
            };

            if let Some(pts) = active_poly {
                fill_poly(pts, window);
                stroke_poly(pts, highlight_stroke_color, 2.0, window);
            }
        },
    );

    let action_panel = cursor_action_panel(
        Some(hover.pointer_pos),
        container_size,
        icon_path,
        title,
        detail,
        hints,
        &overlay_style,
    );

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .absolute()
                .inset_0()
                .size_full()
                .child(canvas_element),
        )
        .children(shadow)
        .child(action_panel)
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
