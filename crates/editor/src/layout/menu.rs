//! Context menus for split pane borders.

use gpui::*;
use splitter::SplitAxis;
use theme::Theme;
use ui::split::chrome::{BorderMenuActions, border_menu_style, render_standard_border_menu};

use crate::editor::Editor;

impl Editor {
    pub(crate) fn render_editor_pane_border_menu(
        &mut self,
        _theme: &Theme,
        _strings: &config::language::I18nStrings,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let border_menu = self.session.root.active_border_menu?;
        if self.session.root.tree.find_maximized_leaf().is_some() {
            self.session.root.active_border_menu = None;
            return None;
        }
        let editor = cx.entity().downgrade();
        let split_id = border_menu.split_id;
        let theme = cx.global::<theme::ThemeManager>().current().clone();
        let menu_style = border_menu_style(&theme);

        let split_h_ed = editor.clone();
        let split_v_ed = editor.clone();
        let swap_ed = editor.clone();
        let close_ed = editor.clone();
        let dismiss_ed = editor.clone();

        let actions = BorderMenuActions {
            split_horizontal: Box::new(move |app| {
                let _ = split_h_ed.update(app, |ed, cx| {
                    ed.split_pane_with_ratio(split_id, SplitAxis::Horizontal, 0.5);
                    ed.session_mut().root.active_border_menu = None;
                    cx.notify();
                });
            }),
            split_vertical: Box::new(move |app| {
                let _ = split_v_ed.update(app, |ed, cx| {
                    ed.split_pane_with_ratio(split_id, SplitAxis::Vertical, 0.5);
                    ed.session_mut().root.active_border_menu = None;
                    cx.notify();
                });
            }),
            swap: Box::new(move |app| {
                let _ = swap_ed.update(app, |ed, cx| {
                    ed.swap_pane_split_sides(split_id);
                    cx.notify();
                });
            }),
            close: Box::new(move |app| {
                let _ = close_ed.update(app, |ed, cx| {
                    ed.close_pane(split_id);
                    ed.session_mut().root.active_border_menu = None;
                    cx.notify();
                });
            }),
        };

        Some(render_standard_border_menu(
            border_menu.position,
            actions,
            &menu_style,
            Box::new(move |app| {
                let _ = dismiss_ed.update(app, |ed, cx| {
                    ed.session_mut().root.active_border_menu = None;
                    cx.notify();
                });
            }),
        ))
    }
}
