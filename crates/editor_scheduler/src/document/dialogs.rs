//! Editor-side dialog state and actions: the table-insert dialog (opened
//! from the editor's context menu) and the document-facing half of the
//! window-close confirmation.
//!
//! The window-level dialogs themselves (unsaved changes, drop-replace,
//! Help-menu info) render on the Shell (`crate::app::window::dialogs`);
//! this module keeps the per-tab state those dialogs show and the editor
//! actions their buttons route to.

use ui::popover::overlay;

use gpui::*;

use crate::engine::controller::Editor;
use theme::Theme;

/// State for the table insertion dialog opened from the context menu.
pub struct TableInsertDialogState {
    pub target_entity: Option<EntityId>,
    pub position: Option<Point<Pixels>>,
    pub rows: usize,
    pub columns: usize,
    pub hovered_rows: Option<usize>,
    pub hovered_cols: Option<usize>,
}

impl TableInsertDialogState {
    pub fn new(
        target_entity: Option<EntityId>,
        rows: usize,
        columns: usize,
        position: Option<Point<Pixels>>,
    ) -> Self {
        Self {
            target_entity,
            position,
            rows: rows.clamp(1, 8),
            columns: columns.clamp(1, 8),
            hovered_rows: None,
            hovered_cols: None,
        }
    }
}

impl Editor {
    /// Cancel the unsaved-changes dialog without closing the window
    /// (routed from the Shell's dialog overlay).
    pub fn cancel_close_dialog(&mut self, cx: &mut Context<Self>) {
        let mut restore_entity = None;
        for tab in self.session.tabs_mut() {
            if tab.file.show_unsaved_changes_dialog {
                tab.file.show_unsaved_changes_dialog = false;
                tab.file.pending_close_after_save = false;
                if let Some(restore) = tab.file.close_dialog_restore_focus.take() {
                    restore_entity = Some(restore);
                }
            }
        }
        if let Some(restore) = restore_entity {
            let pane = self.active_pane_state();
            if let Some(wysiwyg) = pane.as_wysiwyg_mut() {
                wysiwyg.focus.active_entity = Some(restore);
            }
        }
        cx.notify();
    }

    /// Save the current document and then close the window (routed from
    /// the Shell's dialog overlay).
    pub fn save_and_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for tab in self.session.tabs_mut() {
            if tab.file.show_unsaved_changes_dialog {
                tab.file.show_unsaved_changes_dialog = false;
                tab.file.pending_close_after_save = true;
            }
        }
        self.save_document(window, cx);
    }

    /// Discard unsaved changes and close the window immediately (routed
    /// from the Shell's dialog overlay). Clears the dirty flag so close can proceed.
    pub fn discard_and_close(&mut self, cx: &mut Context<Self>) {
        for tab in self.session.tabs_mut() {
            if tab.file.show_unsaved_changes_dialog {
                tab.file.show_unsaved_changes_dialog = false;
                tab.file.pending_close_after_save = false;
                tab.file.dirty = false;
                tab.file.pending_window_edited = false;
                tab.file.pending_window_title_refresh = true;
                tab.file.close_dialog_restore_focus = None;
            }
        }
        cx.notify();
    }
    /// Cancel the pending-close-after-save flag (called when save fails or is
    /// cancelled, or when the save completes but close is no longer desired).
    pub fn abort_pending_close_after_save(&mut self, cx: &mut Context<Self>) {
        self.tab_mut().file.pending_close_after_save = false;
        self.tab_mut().file.close_dialog_restore_focus = None;
        cx.notify();
    }

    /// The interactive Table Insert Matrix Picker popup with Row/Column dimension badges,
    /// dynamic hover preview grid, and Insert / Cancel action buttons.
    pub fn render_table_insert_dialog_overlay(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let dialog = self.table_insert_dialog.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;

        let panel_width = 236.0_f32;
        let panel_height = 296.0_f32;

        let viewport = window.viewport_size();
        let max_x = f32::from(viewport.width) - panel_width - 16.0;
        let max_y = f32::from(viewport.height) - panel_height - 16.0;

        let (panel_x, panel_y) = if let Some(pos) = dialog.position {
            let origin = self.panel_rect.map(|rect| rect.origin).unwrap_or_default();
            let rel_x = pos.x - origin.x;
            let rel_y = pos.y - origin.y;
            (
                px(f32::from(rel_x).clamp(8.0, max_x.max(8.0))),
                px(f32::from(rel_y + px(10.0)).clamp(8.0, max_y.max(8.0))),
            )
        } else {
            (
                px(((f32::from(viewport.width) - panel_width) / 2.0).max(8.0)),
                px(((f32::from(viewport.height) - panel_height) / 2.0).max(8.0)),
            )
        };

        let max_matrix_rows = 8usize;
        let max_matrix_cols = 8usize;

        let display_rows = dialog
            .hovered_rows
            .unwrap_or(dialog.rows)
            .clamp(1, max_matrix_rows);
        let display_cols = dialog
            .hovered_cols
            .unwrap_or(dialog.columns)
            .clamp(1, max_matrix_cols);

        use ui::table_matrix_picker::{render_matrix_dimension_indicator, MatrixCellColors};
        let colors = MatrixCellColors::from_theme(theme);
        let top_indicator = render_matrix_dimension_indicator(display_rows, display_cols, "Row", "Column", theme);

        // Matrix Grid: 8x8 square matrix grid matching original dimensions
        let mut grid_rows = Vec::with_capacity(max_matrix_rows);
        for r in 0..max_matrix_rows {
            let row_num = r + 1;
            let mut row_cells = Vec::with_capacity(max_matrix_cols);
            for col in 0..max_matrix_cols {
                let col_num = col + 1;
                let cell_bg = if let (Some(h_r), Some(h_c)) = (dialog.hovered_rows, dialog.hovered_cols) {
                    if r < h_r && col < h_c {
                        colors.hover_only
                    } else {
                        colors.inactive
                    }
                } else {
                    colors.inactive
                };

                let cell = div()
                    .id(ElementId::Name(format!("table-insert-cell-{}-{}", r, col).into()))
                    .size(px(22.0))
                    .rounded(px(3.5))
                    .bg(cell_bg)
                    .cursor_pointer()
                    .on_hover(cx.listener(move |editor, hovered: &bool, _window, cx| {
                        if *hovered {
                            editor.set_table_insert_hover(Some(row_num), Some(col_num), cx);
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _event, _window, cx| {
                            editor.insert_table_from_dialog(row_num, col_num, cx);
                        }),
                    );
                row_cells.push(cell);
            }
            grid_rows.push(
                div()
                    .flex()
                    .gap(px(4.0))
                    .children(row_cells),
            );
        }

        let matrix_grid = div()
            .id("table-insert-matrix-grid")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .py(px(6.0))
            .on_hover(cx.listener(|editor, hovered: &bool, _window, cx| {
                if !*hovered {
                    editor.set_table_insert_hover(None, None, cx);
                }
            }))
            .children(grid_rows);

        Some(
            deferred(
                overlay()
                    .id("table-insert-dialog-overlay")
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|editor, _event, _window, cx| {
                            editor.close_table_insert_dialog(cx);
                        }),
                    )
                    .child(
                        div()
                            .id("table-insert-dialog-panel")
                            .absolute()
                            .left(panel_x)
                            .top(panel_y)
                            .p(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.menu_panel_radius))
                            .shadow_lg()
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation();
                            })
                            .on_key_down(cx.listener(|editor, event, _window, cx| {
                                editor.handle_table_insert_key_down(event, cx);
                            }))
                            .child(top_indicator)
                            .child(matrix_grid),
                    ),
            )
            .into_any_element(),
        )
    }
}
