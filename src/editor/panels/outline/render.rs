//! Outline panel — heading tree navigation.

use gpui::*;

use crate::editor::controller::*;
use crate::editor::panels::outline::{build_outline_tree, prune_outline_state};
use crate::editor::window::workspace::WorkspaceSelection;
use crate::infra::i18n::I18nStrings;
use crate::theme::Theme;

impl Editor {
    pub(crate) fn sync_workspace_outline(&mut self, cx: &mut Context<Self>) {
        let source = self.serialized_document_text(cx);
        if self.panels.workspace.outline_source.as_deref() == Some(source.as_str()) {
            return;
        }

        let outline = build_outline_tree(&source);
        prune_outline_state(&mut self.panels.workspace, &outline);
        self.panels.workspace.outline_tree = outline;
        self.panels.workspace.outline_source = Some(source);
    }
    pub(crate) fn select_outline_node(&mut self, id: String, cx: &mut Context<Self>) {
        self.panels.workspace.selected = Some(WorkspaceSelection::Outline(id));
        cx.notify();
    }
    pub(crate) fn render_workspace_outline_tree(
        &self,
        theme: &Theme,
        strings: &I18nStrings,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        if self.panels.workspace.outline_tree.is_empty() {
            return self.render_workspace_empty_state(
                "",
                &strings.workspace_empty_outline,
                theme,
                editor,
            );
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .children(self.render_workspace_nodes(
                &self.panels.workspace.outline_tree,
                0,
                theme,
                editor,
            ))
            .into_any_element()
    }
    pub(crate) fn render_tiled_outline_panel(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_workspace_models(cx);
        let editor = cx.entity().downgrade();
        self.render_workspace_outline_tree(theme, strings, &editor)
    }
}
