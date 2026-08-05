//! Outline panel — heading tree navigation.

use gpui::*;

use crate::editor::controller::*;
use crate::editor::windows::layout::workspace::WorkspaceSelection;
use crate::editor::windows::outline::{build_outline_tree, prune_outline_state};
use crate::infra::i18n::I18nStrings;
use crate::theme::Theme;
use crate::ui::components::empty_state::empty_state_container;

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
            return self.render_outline_empty_state(theme, strings);
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

    /// Empty-state view for the outline panel: the document exists but has
    /// no headings. Deliberately separate from the explorer's empty state —
    /// the outline has no actionable button, just a hint.
    pub(crate) fn render_outline_empty_state(
        &self,
        theme: &Theme,
        strings: &I18nStrings,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;

        empty_state_container()
            .gap(px(10.0))
            .px(px(24.0))
            .child(
                svg()
                    .path("icon/workspace/markdown.svg")
                    .size(px(36.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .max_w(px(230.0))
                    .text_size(px(t.text_size * 0.78))
                    .line_height(px(t.text_size * t.text_line_height * 0.90))
                    .text_color(c.dialog_muted)
                    .child(strings.workspace_empty_outline.clone()),
            )
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
