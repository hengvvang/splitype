//! Editor pane layout — rendering and gesture driving for the
//! `PaneKind` split tree (WYSIWYG / Source Code / Preview / custom panes) inside each Editor panel.

pub(crate) mod drag;
pub(crate) mod menu;
pub(crate) mod node;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::editor::Editor;
use config::language::I18nStrings;
use core_contracts::PaneId;
use theme::Theme;

impl Editor {
    /// Render one Editor area's pane layout.
    pub(crate) fn render_editor_pane_layout(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let inner_tree = self.session.root.tree.clone();

        if let Some(tab) = self.session.active_tab_mut() {
            tab.panes.retain(|pane, _| inner_tree.contains_leaf(pane.0));
        } else {
            self.session.empty_panes.retain(|pane, _| inner_tree.contains_leaf(pane.0));
        }

        if self.focused_pane_id.is_none() {
            if let Some(leaf_id) = inner_tree.first_leaf_id() {
                self.focused_pane_id = Some(PaneId(leaf_id));
            }
        }

        let maximized_pane = inner_tree.find_maximized_leaf();
        let is_maximized = maximized_pane.is_some();
        let inner_rendered = if let Some(maximized_pane) = maximized_pane {
            let single = splitter::tree::SplitTree::Leaf(
                splitter::container::SplitterContainer::new(
                    maximized_pane.id,
                    maximized_pane.kind,
                ),
            );
            self.render_editor_pane_split_tree(&single, theme, strings, window, cx)
        } else {
            self.render_editor_pane_split_tree(&inner_tree, theme, strings, window, cx)
        };

        div()
            .id("editor-pane-layout")
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .relative()
            .bg(c.editor_background)
            .child(inner_rendered)
            .when(is_maximized, |el| {
                el.child(
                    div()
                        .id("editor-maximized-indicator")
                        .absolute()
                        .top_2()
                        .right_2()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(c.dialog_surface)
                        .text_size(px(11.0))
                        .text_color(c.dialog_muted)
                        .child("Maximized (click button or shortcut to restore)"),
                )
            })
            .into_any_element()
    }
}


