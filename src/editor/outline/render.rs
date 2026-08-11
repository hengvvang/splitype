//! Outline panel — heading tree navigation.

use gpui::*;

use crate::editor::controller::*;
use crate::editor::explorer_state::state::{
    EXPLORER_NODE_HEIGHT, EXPLORER_NODE_INDENT, stable_node_hash,
};
use crate::editor::outline::state::{OutlineNode, OutlineNodeKind};
use crate::editor::outline::{build_outline_tree, prune_outline_state};
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;
use crate::ui::empty_state::empty_state_container;

impl Editor {
    pub(crate) fn sync_explorer_outline(&mut self, cx: &mut Context<Self>) {
        let Some(source) = self.active_editor_serialized_text(cx) else {
            return;
        };
        if self.panels.outline.source.as_deref() == Some(source.as_str()) {
            return;
        }

        let outline = build_outline_tree(&source);
        prune_outline_state(&mut self.panels.outline, &outline);
        self.panels.outline.tree = outline;
        self.panels.outline.source = Some(source);
    }
    pub(crate) fn select_outline_node(&mut self, id: String, cx: &mut Context<Self>) {
        self.panels.outline.selected = Some(id);
        cx.notify();
    }
    pub(crate) fn render_outline_tree(
        &self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        if self.panels.outline.tree.is_empty() {
            return self.render_outline_empty_state(theme, strings);
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .children(self.render_outline_nodes(
                &self.panels.outline.tree,
                0,
                area_id,
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
                    .path("icons/editor/outline/markdown.svg")
                    .size(px(40.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .max_w(px(230.0))
                    .text_size(px(t.text_size * 0.78))
                    .line_height(px(t.text_size * t.text_line_height * 0.90))
                    .text_color(c.dialog_muted)
                    .child(strings.explorer_empty_outline.clone()),
            )
            .into_any_element()
    }
    pub(crate) fn render_tiled_outline_panel(
        &mut self,
        area_id: usize,
        _panel_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_explorer_models(cx);
        let editor = cx.entity().downgrade();
        self.render_outline_tree(area_id, theme, strings, &editor)
    }

    pub(crate) fn toggle_outline_node(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.panels.outline.expanded.remove(id) {
            self.panels.outline.expanded.insert(id.to_string());
        }
        cx.notify();
    }

    // ── Outline rendering (non-virtualized; heading trees are small) ────

    pub(crate) fn render_outline_nodes(
        &self,
        nodes: &[OutlineNode],
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::new();
        for node in nodes {
            elements.push(self.render_outline_node(node, depth, area_id, theme, editor));
            if !node.children.is_empty() && self.panels.outline.expanded.contains(&node.id) {
                elements.extend(self.render_outline_nodes(
                    &node.children,
                    depth + 1,
                    area_id,
                    theme,
                    editor,
                ));
            }
        }
        elements
    }
    pub(crate) fn render_outline_node(
        &self,
        node: &OutlineNode,
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let is_expanded = self.panels.outline.expanded.contains(&node.id);
        let has_children = !node.children.is_empty();
        let selected = matches!(&self.panels.outline.selected, Some(id) if id == &node.id);
        let node_id = node.id.clone();
        let click_editor = editor.clone();
        let click_kind = node.kind.clone();
        let arrow_node_id = node.id.clone();
        let arrow_editor = editor.clone();

        let heading_badge = match &node.kind {
            OutlineNodeKind::Heading { level, .. } => {
                let badge_color = match level {
                    1 => c.callout_note_border,
                    2 => c.callout_tip_border,
                    3 => c.callout_important_border,
                    4 => c.callout_warning_border,
                    5 => c.callout_caution_border,
                    _ => c.dialog_muted,
                };
                Some(
                    div()
                        .px(px(4.0))
                        .py(px(1.0))
                        .rounded(px(3.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_color)
                        .bg(badge_color.opacity(0.12))
                        .child(format!("H{level}")),
                )
            }
        };

        let label_color = if selected {
            c.text_default
        } else {
            c.dialog_muted
        };

        let mut arrow_el = div()
            .w(px(14.0))
            .h(px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center();

        if has_children {
            arrow_el = arrow_el
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_expanded {
                            "icons/explorer/worktree/chevron-down.svg"
                        } else {
                            "icons/explorer/worktree/chevron-right.svg"
                        })
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_outline_node(&arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        div()
            .id(ElementId::Name(
                format!("explorer-node-{area_id}-{}", stable_node_hash(&node.id)).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(if selected {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .children(heading_badge)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(label_color)
                    .child(node.label.clone()),
            )
            .on_click(move |_event, _window, cx| {
                let node_id = node_id.clone();
                let click_kind = click_kind.clone();
                let _ = click_editor.update(cx, |editor, cx| match click_kind {
                    OutlineNodeKind::Heading { .. } => {
                        editor.select_outline_node(node_id, cx);
                    }
                });
            })
            .into_any_element()
    }
}
