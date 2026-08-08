//! Window control buttons (minimize / maximize / close) — the "chrome" of
//! the custom title bar.
//!
//! Rendered into the right side of the title bar on platforms that use
//! app-drawn controls (Windows, Linux client decorations); macOS uses the
//! native traffic lights instead (`TitlebarControlMode`).

use gpui::*;

use crate::theme::{Theme, ThemeColors};

pub(crate) const TITLEBAR_BUTTON_WIDTH: f32 = 46.0;
pub(crate) const TITLEBAR_ICON_SIZE: f32 = 12.0;
pub(crate) const TITLEBAR_CLOSE_ICON: &str = "icons/titlebar/chrome/close.svg";
pub(crate) const TITLEBAR_MAXIMIZE_ICON: &str = "icons/titlebar/chrome/maximize.svg";
pub(crate) const TITLEBAR_MINIMIZE_ICON: &str = "icons/titlebar/chrome/minimize.svg";
pub(crate) const TITLEBAR_RESTORE_ICON: &str = "icons/titlebar/chrome/restore.svg";

/// Icon colour for the window control buttons, chosen for contrast against
/// the title bar background.
pub(crate) fn custom_titlebar_icon_color(theme: &Theme) -> Hsla {
    if theme.colors.dialog_surface.l < 0.5 {
        Hsla::from(rgba(0xf4f4f5ff))
    } else {
        Hsla::from(rgba(0x18181bff))
    }
}

pub(crate) fn titlebar_maximize_icon(is_maximized: bool, is_fullscreen: bool) -> &'static str {
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
pub(crate) fn render_window_control_buttons<T: 'static>(
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
            .on_click(move |event, window, app| {
                if event.standard_click() {
                    let _ = entity.update(app, |view, cx| {
                        on_close(view, event, window, cx);
                    });
                }
            }),
    );

    controls_row.into_any_element()
}
