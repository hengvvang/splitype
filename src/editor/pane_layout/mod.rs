//! Editor pane layout — rendering and gesture driving for the
//! `EditorPaneKind` split tree (Wysiwyg / Source Code / Preview /
//! Outline panes) inside each Editor panel.

pub(crate) mod border_menu;
pub(crate) mod drag_drop;
pub(crate) mod render_node;
pub(crate) mod welcome;

use gpui::*;

use crate::editor::controller::*;
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;

impl Editor {
    /// Render one Editor area's pane layout. One Editor entity serves
    /// exactly one area, so `tab()`/`doc()` always read this editor's session.
    ///
    /// Side effects before rendering the tree:
    /// - drop runtimes of panes that were closed or joined;
    /// - derive `focused_pane` from the keyboard focus when
    ///   nothing was explicitly selected (projection fallback).
    pub(crate) fn render_editor_pane_layout(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        self.tab_list_mut();
        let inner_tree = self.session.root.tree.clone();

        // Drop view states of panes that were closed or joined. (One Editor
        // entity serves one panel, so all runtimes here belong to this
        // render pass.)
        if self.has_active_tab() {
            let tab = self.tab_mut();
            tab.panes.retain(|pane, _| inner_tree.contains_leaf(*pane));
        }

        // Derive the focused pane from the keyboard focus when nothing is
        // focused yet — clicking inside a block or Tab navigation never
        // reaches the pane div. Explicit clicks take precedence. Only runs
        // for editing panels: a welcome panel has no tabs, hence no edit
        // targets to derive from.
        if self.focused_pane_id.is_none()
            && self.panel_mode().is_editing()
            && let Some(target_id) = self.focused_edit_target_entity_id(window, cx)
        {
            if let Some((pane_id, _)) = self.tab().panes.iter().find(|(_, state)| {
                state
                    .source_block
                    .as_ref()
                    .is_some_and(|block| block.entity_id() == target_id)
            }) {
                // Keyboard focus sits in a source pane's own block.
                if inner_tree.contains_leaf(*pane_id) {
                    self.focused_pane_id = Some(*pane_id);
                }
            } else if self.doc().block_entity_by_id(target_id).is_some() {
                // Keyboard focus sits in the shared document: point at the
                // panel's first Wysiwyg pane.
                let mut ids = Vec::new();
                inner_tree.leaf_ids(&mut ids);
                if let Some(pane_id) = ids
                    .into_iter()
                    .find(|id| inner_tree.find_leaf_kind(*id) == Some(EditorPaneKind::Wysiwyg))
                {
                    self.focused_pane_id = Some(pane_id);
                }
            }
        }

        let inner_rendered = self.render_editor_pane_node(&inner_tree, theme, strings, window, cx);

        let dropdown = {
            let root = &self.session.root;
            // The open dropdown belongs to the currently focused pane.
            let pane_id = self.focused_pane_id.unwrap_or(1);
            if root.tree.find_leaf(pane_id).is_some_and(|p| p.open_dropdown) {
                let current_kind = root
                    .tree
                    .find_leaf_kind(pane_id)
                    .unwrap_or(EditorPaneKind::Wysiwyg);
                Some(self.render_editor_pane_dropdown_menu(pane_id, current_kind, theme, cx))
            } else {
                None
            }
        };

        let mut container = div()
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(inner_rendered);

        if let Some(dropdown) = dropdown {
            container = container.child(dropdown);
        }

        // Inner corner-drag preview: rendered inside the pane layout container so
        // the normalized rects position with `relative()` against the
        // layout's initialization region (topbar/bottombar excluded). Host
        // policy: only plain (no-modifier) drags show an indicator.
        let d = &theme.dimensions;
        let overlay_style = splitype_splitter::interaction::OverlayStyle {
            accent: c.split_indicator,
            tile_radius: d.panel_tile_radius,
            border: c.dialog_border,
            selection: c.selection,
            active: c.focus_accent,
            surface: c.dialog_surface,
            text: c.dialog_title,
        };
        let inner_size = self
            .panel_rect
            .map(|rect| size(rect.size.width, rect.size.height))
            .unwrap_or_else(|| window.viewport_size());
        // The corner-drag session lives on the dragging panel itself;
        // find it via the root.
        if let Some(drag_panel) = self.session_mut().root.corner_drag_panel() {
            let drag = self
                .session_mut()
                .root
                .tree
                .find_leaf(drag_panel)
                .unwrap()
                .active_corner_drag
                .unwrap();
            if drag.modifier == splitype_splitter::sessions::CornerDragModifier::None
                || drag.modifier == splitype_splitter::sessions::CornerDragModifier::Ctrl
                || drag.modifier == splitype_splitter::sessions::CornerDragModifier::Shift
            {
                if let Some(preview) =
                    crate::editor::corner_drag_preview::render_corner_drag_preview(
                        &self.session_mut().root,
                        &drag,
                        inner_size,
                        &overlay_style,
                    )
                {
                    container = container.child(preview);
                }
            }
        }

        // Pane border menu: same context menu as the outer window
        // panels, rendered by the layout crate and wired to the per-panel
        // pane operations. The split node id doubles as the id of
        // its second (right/bottom) leaf, matching the outer tree's
        // semantics: Split/Close act on that side, Swap flips the sides.
        if let Some(border_menu) = self.session_mut().root.active_border_menu {
            let menu_overlay = self.render_editor_pane_border_menu(border_menu, theme, cx);
            container = container.child(menu_overlay);
        }

        container.into_any_element()
    }
}
