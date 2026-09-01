//! Splitter border context menu rendering and handlers.

use gpui::*;

use crate::shell::Shell;
use splitter::tree::SplitAxis;
use theme::Theme;

impl Shell {
    pub(crate) fn render_window_panel_border_menu(
        &mut self,
        border_menu: splitter::BorderMenuState,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let shell = cx.entity().downgrade();
        let split_id = border_menu.split_id;

        let menu_style = ui::chrome::border_menu_style(theme);

        let split_h_shell = shell.clone();
        let split_h: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_h_shell.update(app, |shell, cx| {
                shell.split_panel(split_id, SplitAxis::Horizontal, 0.5, true, cx);
                cx.notify();
            });
        });
        let split_v_shell = shell.clone();
        let split_v: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_v_shell.update(app, |shell, cx| {
                shell.split_panel(split_id, SplitAxis::Vertical, 0.5, true, cx);
                cx.notify();
            });
        });
        let swap_shell = shell.clone();
        let swap: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = swap_shell.update(app, |shell, cx| {
                shell.panels.layout.swap_split_sides(split_id);
                cx.notify();
            });
        });
        let close_shell = shell.clone();
        let close: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = close_shell.update(app, |shell, cx| {
                shell.close_panel(split_id, cx);
                cx.notify();
            });
        });
        let dismiss_shell = shell.clone();
        let dismiss: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = dismiss_shell.update(app, |shell, cx| {
                shell.panels.layout.active_border_menu = None;
                cx.notify();
            });
        });

        splitter::interaction::render_border_menu(
            border_menu.position,
            vec![
                splitter::interaction::BorderMenuItem {
                    label: "Split Horizontally",
                    on_activate: split_h,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Split Vertically",
                    on_activate: split_v,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Swap Panels",
                    on_activate: swap,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Close Panel",
                    on_activate: close,
                },
            ],
            &menu_style,
            dismiss,
        )
    }
}
