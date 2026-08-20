//! Custom titlebar chrome — window control buttons, titlebar rendering, and
//! platform window options.
//!
//! Shared by the editor window and the standalone settings window, so it
//! lives in the reusable component layer (`crate::ui`): generic over the
//! hosting view entity and free of editor/model imports.

use gpui::*;

use crate::infra::theme::{Theme, ThemeColors, ThemeDimensions};
use crate::platform::app_identity::SPLITYPE_APP_ID;

const TITLEBAR_MIN_HEIGHT: f32 = 32.0;
const MAC_TRAFFIC_LIGHT_RESERVED_WIDTH: f32 = 84.0;

// ── Window control buttons ("chrome") ─────────────────────────────────────

pub const TITLEBAR_BUTTON_WIDTH: f32 = 46.0;
pub const TITLEBAR_ICON_SIZE: f32 = 12.0;
pub const TITLEBAR_CLOSE_ICON: &str = "icons/titlebar/chrome/close.svg";
pub const TITLEBAR_MAXIMIZE_ICON: &str = "icons/titlebar/chrome/maximize.svg";
pub const TITLEBAR_MINIMIZE_ICON: &str = "icons/titlebar/chrome/mins.svg";
pub const TITLEBAR_RESTORE_ICON: &str = "icons/titlebar/chrome/restore.svg";

/// Icon colour for the window control buttons, chosen for contrast against
/// the title bar background.
pub fn custom_titlebar_icon_color(theme: &Theme) -> Hsla {
    if theme.colors.dialog_surface.l < 0.5 {
        Hsla::from(rgba(0xf4f4f5ff))
    } else {
        Hsla::from(rgba(0x18181bff))
    }
}

pub fn titlebar_maximize_icon(is_maximized: bool, is_fullscreen: bool) -> &'static str {
    if is_maximized || is_fullscreen {
        TITLEBAR_RESTORE_ICON
    } else {
        TITLEBAR_MAXIMIZE_ICON
    }
}

/// Window control button (minimize / maximize / close).
fn titlebar_control_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    area: WindowControlArea,
) -> Stateful<Div> {
    let hover_bg = if area == WindowControlArea::Close {
        c.dialog_danger_button_bg
    } else {
        c.dialog_secondary_button_hover
    };
    div()
        .id(id)
        .w(px(TITLEBAR_BUTTON_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .window_control_area(area)
        .hover(move |this| this.bg(hover_bg))
        .cursor_pointer()
}

/// The minimize / maximize / close button group, rendered for app-drawn
/// controls. `entity` is the window view; the close button routes through
/// `on_close` so the caller decides what closing means for its window type.
pub fn render_window_control_buttons<T: 'static>(
    window: &Window,
    c: &ThemeColors,
    icon_color: Hsla,
    entity: WeakEntity<T>,
    on_close: fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>),
) -> AnyElement {
    let controls = window.window_controls();
    let mut controls_row = div().h_full().flex().items_center().flex_shrink_0();

    if controls.minimize {
        controls_row = controls_row.child(
            titlebar_control_button("window-titlebar-minimize", c, WindowControlArea::Min)
                .child(
                    svg()
                        .path(TITLEBAR_MINIMIZE_ICON)
                        .size(px(TITLEBAR_ICON_SIZE))
                        .text_color(icon_color),
                )
                .on_click(|event, window, _cx| {
                    if event.standard_click() {
                        window.minimize_window();
                    }
                }),
        );
    }

    if controls.maximize {
        controls_row = controls_row.child(
            titlebar_control_button("window-titlebar-maximize", c, WindowControlArea::Max)
                .child(
                    svg()
                        .path(titlebar_maximize_icon(
                            window.is_maximized(),
                            window.is_fullscreen(),
                        ))
                        .size(px(TITLEBAR_ICON_SIZE))
                        .text_color(icon_color),
                )
                .on_click(|event, window, _cx| {
                    if event.standard_click() {
                        window.zoom_window();
                    }
                }),
        );
    }

    controls_row = controls_row.child(
        titlebar_control_button("window-titlebar-close", c, WindowControlArea::Close)
            .child(
                svg()
                    .path(TITLEBAR_CLOSE_ICON)
                    .size(px(TITLEBAR_ICON_SIZE))
                    .text_color(icon_color),
            )
            .on_click(move |event, window, cx| {
                if event.standard_click() {
                    let _ = entity.update(cx, |view, cx| {
                        on_close(view, event, window, cx);
                    });
                }
            }),
    );

    controls_row.into_any_element()
}

// ── Window options ────────────────────────────────────────────────────────

/// Selects whether splitype or the platform should render window controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitlebarControlMode {
    NativeTrafficLights,
    AppControls,
}

/// Layout metadata shared by editor and preferences windows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CustomTitlebarLayout {
    pub height: f32,
    pub controls: TitlebarControlMode,
}

/// Chooses the drag mechanism for the platform titlebar implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitlebarDragStrategy {
    PlatformHitTest,
    ExplicitMoveRequest,
}

pub fn titlebar_options_for_target_os(target_os: &str, title: SharedString) -> TitlebarOptions {
    TitlebarOptions {
        title: Some(title),
        appears_transparent: matches!(target_os, "macos" | "windows"),
        traffic_light_position: if target_os == "macos" {
            Some(point(px(14.0), px(10.0)))
        } else {
            None
        },
    }
}

pub fn window_decorations_for_target_os(target_os: &str) -> Option<WindowDecorations> {
    match target_os {
        "linux" | "freebsd" => Some(WindowDecorations::Client),
        _ => None,
    }
}

pub fn splitype_window_options_for_target_os(
    target_os: &str,
    title: SharedString,
    bounds: Bounds<Pixels>,
) -> WindowOptions {
    WindowOptions {
        app_id: Some(SPLITYPE_APP_ID.to_string()),
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(titlebar_options_for_target_os(target_os, title)),
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: window_decorations_for_target_os(target_os),
        ..WindowOptions::default()
    }
}

pub fn splitype_window_options(title: SharedString, bounds: Bounds<Pixels>) -> WindowOptions {
    splitype_window_options_for_target_os(std::env::consts::OS, title, bounds)
}

// ── Titlebar layout & drag ────────────────────────────────────────────────

pub fn custom_titlebar_layout_for_target_os(
    target_os: &str,
    decorations: Decorations,
    dimensions: &ThemeDimensions,
) -> Option<CustomTitlebarLayout> {
    let height = dimensions.menu_bar_height.max(TITLEBAR_MIN_HEIGHT);
    match target_os {
        "macos" => Some(CustomTitlebarLayout {
            height,
            controls: TitlebarControlMode::NativeTrafficLights,
        }),
        "windows" => Some(CustomTitlebarLayout {
            height,
            controls: TitlebarControlMode::AppControls,
        }),
        "linux" | "freebsd" if matches!(decorations, Decorations::Client { .. }) => {
            Some(CustomTitlebarLayout {
                height,
                controls: TitlebarControlMode::AppControls,
            })
        }
        _ => None,
    }
}

/// Windows/macOS use hit-test drag areas; Linux client decorations need an explicit move request.
pub fn titlebar_drag_strategy_for_target_os(
    target_os: &str,
    decorations: Decorations,
) -> TitlebarDragStrategy {
    match target_os {
        "linux" | "freebsd" if matches!(decorations, Decorations::Client { .. }) => {
            TitlebarDragStrategy::ExplicitMoveRequest
        }
        _ => TitlebarDragStrategy::PlatformHitTest,
    }
}

pub fn custom_titlebar_height_for_target_os(
    target_os: &str,
    decorations: Decorations,
    dimensions: &ThemeDimensions,
) -> f32 {
    custom_titlebar_layout_for_target_os(target_os, decorations, dimensions)
        .map(|layout| layout.height)
        .unwrap_or(0.0)
}

pub fn custom_titlebar_height(window: &Window, dimensions: &ThemeDimensions) -> f32 {
    if cfg!(target_os = "macos") && window.is_fullscreen() {
        return 0.0;
    }

    custom_titlebar_height_for_target_os(
        std::env::consts::OS,
        window.window_decorations(),
        dimensions,
    )
}

pub fn custom_titlebar_background(theme: &Theme) -> Hsla {
    theme.colors.dialog_surface
}

/// Window control button (minimize / maximize / close).
pub fn render_custom_titlebar<T: 'static>(
    id: &'static str,
    title: SharedString,
    left_content: Option<AnyElement>,
    theme: &Theme,
    window: &Window,
    cx: &mut Context<T>,
    on_close: fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>),
) -> Option<AnyElement> {
    if cfg!(target_os = "macos") && window.is_fullscreen() {
        return None;
    }

    let layout = custom_titlebar_layout_for_target_os(
        std::env::consts::OS,
        window.window_decorations(),
        &theme.dimensions,
    )?;
    let drag_strategy =
        titlebar_drag_strategy_for_target_os(std::env::consts::OS, window.window_decorations());
    let c = &theme.colors;
    let t = &theme.typography;
    let icon_color = custom_titlebar_icon_color(theme);
    let entity = cx.entity().downgrade();

    let drag_title = div()
        .id("window-titlebar-drag-title")
        .h_full()
        .flex_1()
        .min_w(px(0.0))
        .px(px(12.0))
        .flex()
        .items_center()
        .window_control_area(WindowControlArea::Drag)
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .text_size(px(theme.dimensions.menu_text_size))
                .font_weight(t.dialog_button_weight.to_font_weight())
                .text_color(c.dialog_secondary_button_text)
                .child(title),
        );

    let drag_title = match drag_strategy {
        TitlebarDragStrategy::PlatformHitTest => drag_title,
        TitlebarDragStrategy::ExplicitMoveRequest => {
            drag_title.on_mouse_down(MouseButton::Left, |event, window, cx| {
                if event.click_count >= 2 {
                    window.zoom_window();
                } else {
                    window.start_window_move();
                }
                cx.stop_propagation();
            })
        }
    }
    .on_click(|event, window, _cx| {
        if event.is_right_click() {
            window.show_window_menu(event.position());
        }
    });

    let root = div()
        .id(id)
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(layout.height))
        .occlude()
        .flex()
        .items_center()
        .bg(custom_titlebar_background(theme))
        .border_b(px(theme.dimensions.dialog_border_width))
        .border_color(c.dialog_border);

    let root = match layout.controls {
        TitlebarControlMode::NativeTrafficLights => {
            let mut r = root.child(div().w(px(MAC_TRAFFIC_LIGHT_RESERVED_WIDTH)).h_full());
            if let Some(left) = left_content {
                r = r.child(left);
            }
            r.child(drag_title)
                .child(div().w(px(MAC_TRAFFIC_LIGHT_RESERVED_WIDTH)).h_full())
        }
        TitlebarControlMode::AppControls => {
            let controls_row =
                render_window_control_buttons(window, c, icon_color, entity, on_close);
            if let Some(left) = left_content {
                root.child(left).child(drag_title).child(controls_row)
            } else {
                root.child(drag_title).child(controls_row)
            }
        }
    };

    Some(root.into_any_element())
}

#[cfg(test)]
mod tests {
    // NOTE: import explicitly, not via `use super::*` — the parent module's
    // generic element builders (`render_window_control_buttons<T>`, …) push
    // `#[test]` expansion past the recursion limit on Windows.
    use super::{
        TITLEBAR_MAXIMIZE_ICON, TITLEBAR_MIN_HEIGHT, TITLEBAR_RESTORE_ICON, TitlebarDragStrategy,
        custom_titlebar_background, custom_titlebar_height_for_target_os,
        custom_titlebar_icon_color, titlebar_drag_strategy_for_target_os, titlebar_maximize_icon,
        titlebar_options_for_target_os, window_decorations_for_target_os,
    };
    use crate::infra::theme::Theme;
    use gpui::{Decorations, Hsla, Tiling, WindowDecorations, rgba};

    #[test]
    fn titlebar_options_enable_transparency_on_mac_and_windows() {
        assert!(titlebar_options_for_target_os("windows", "splitype".into()).appears_transparent);
        assert!(titlebar_options_for_target_os("macos", "splitype".into()).appears_transparent);
        assert!(!titlebar_options_for_target_os("linux", "splitype".into()).appears_transparent);
    }

    #[test]
    fn linux_and_freebsd_request_client_decorations() {
        assert_eq!(
            window_decorations_for_target_os("linux"),
            Some(WindowDecorations::Client)
        );
        assert_eq!(
            window_decorations_for_target_os("freebsd"),
            Some(WindowDecorations::Client)
        );
        assert_eq!(window_decorations_for_target_os("unknown"), None);
    }

    #[test]
    fn custom_titlebar_height_respects_platform_and_decorations() {
        let dimensions = Theme::default_theme().dimensions;
        assert_eq!(
            custom_titlebar_height_for_target_os("windows", Decorations::Server, &dimensions),
            dimensions.menu_bar_height.max(TITLEBAR_MIN_HEIGHT)
        );
        assert_eq!(
            custom_titlebar_height_for_target_os(
                "linux",
                Decorations::Client {
                    tiling: Tiling::default()
                },
                &dimensions,
            ),
            dimensions.menu_bar_height.max(TITLEBAR_MIN_HEIGHT)
        );
        assert_eq!(
            custom_titlebar_height_for_target_os("linux", Decorations::Server, &dimensions),
            0.0
        );
        assert_eq!(
            custom_titlebar_height_for_target_os("unknown", Decorations::Server, &dimensions),
            0.0
        );
    }

    #[test]
    fn titlebar_drag_strategy_matches_platform_window_api() {
        assert_eq!(
            titlebar_drag_strategy_for_target_os("windows", Decorations::Server),
            TitlebarDragStrategy::PlatformHitTest
        );
        assert_eq!(
            titlebar_drag_strategy_for_target_os("macos", Decorations::Server),
            TitlebarDragStrategy::PlatformHitTest
        );
        assert_eq!(
            titlebar_drag_strategy_for_target_os(
                "linux",
                Decorations::Client {
                    tiling: Tiling::default()
                },
            ),
            TitlebarDragStrategy::ExplicitMoveRequest
        );
        assert_eq!(
            titlebar_drag_strategy_for_target_os("linux", Decorations::Server),
            TitlebarDragStrategy::PlatformHitTest
        );
    }

    #[test]
    fn custom_titlebar_background_uses_dialog_surface_token() {
        let theme = Theme::light_theme();
        assert_eq!(
            custom_titlebar_background(&theme),
            theme.colors.dialog_surface
        );
    }

    #[test]
    fn custom_titlebar_icon_color_contrasts_with_theme_surface() {
        assert_eq!(
            custom_titlebar_icon_color(&Theme::default_theme()),
            Hsla::from(rgba(0xf4f4f5ff))
        );
        assert_eq!(
            custom_titlebar_icon_color(&Theme::light_theme()),
            Hsla::from(rgba(0x18181bff))
        );
    }

    #[test]
    fn titlebar_maximize_icon_tracks_window_state() {
        assert_eq!(titlebar_maximize_icon(false, false), TITLEBAR_MAXIMIZE_ICON);
        assert_eq!(titlebar_maximize_icon(true, false), TITLEBAR_RESTORE_ICON);
        assert_eq!(titlebar_maximize_icon(false, true), TITLEBAR_RESTORE_ICON);
    }
}
