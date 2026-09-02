//! Splitter border context menu rendering and handlers.

use gpui::*;

use crate::shell::Shell;
use splitter::tree::SplitAxis;
use theme::Theme;
use ui::split::chrome::{BorderMenuActions, border_menu_style, render_standard_border_menu};

impl Shell {
    pub(crate) fn render_window_panel_border_menu(
        &mut self,
        border_menu: splitter::BorderMenuState,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.panels.layout.tree.find_maximized_leaf().is_some() {
            self.panels.layout.active_border_menu = None;
            return div().into_any_element();
        }
        let shell = cx.entity().downgrade();
        let split_id = border_menu.split_id;

        let menu_style = border_menu_style(theme);

        let split_h_shell = shell.clone();
        let split_v_shell = shell.clone();
        let swap_shell = shell.clone();
        let close_shell = shell.clone();
        let dismiss_shell = shell.clone();

        let actions = BorderMenuActions {
            split_horizontal: Box::new(move |app| {
                let _ = split_h_shell.update(app, |shell, cx| {
                    shell.split_panel(split_id, SplitAxis::Horizontal, 0.5, true, cx);
                    shell.panels.layout.active_border_menu = None;
                    cx.notify();
                });
            }),
            split_vertical: Box::new(move |app| {
                let _ = split_v_shell.update(app, |shell, cx| {
                    shell.split_panel(split_id, SplitAxis::Vertical, 0.5, true, cx);
                    shell.panels.layout.active_border_menu = None;
                    cx.notify();
                });
            }),
            swap: Box::new(move |app| {
                let _ = swap_shell.update(app, |shell, cx| {
                    shell.panels.layout.swap_split_sides(split_id);
                    cx.notify();
                });
            }),
            close: Box::new(move |app| {
                let _ = close_shell.update(app, |shell, cx| {
                    shell.close_panel(split_id, cx);
                    shell.panels.layout.active_border_menu = None;
                    cx.notify();
                });
            }),
        };

        render_standard_border_menu(
            border_menu.position,
            actions,
            &menu_style,
            Box::new(move |app| {
                let _ = dismiss_shell.update(app, |shell, cx| {
                    shell.panels.layout.active_border_menu = None;
                    cx.notify();
                });
            }),
        )
    }
}
