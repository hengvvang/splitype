use gpui::*;

use crate::ExplorerState;

use crate::filename_editor::ExplorerFilenameInputElement;
use crate::state::state::{
    EXPLORER_NODE_HEIGHT, EXPLORER_NODE_INDENT, ExplorerValidation, FILE_ICON, FOLDER_ICON,
};
use theme::Theme;
use window::PanelId;

impl ExplorerState {
    /// Render the inline create/rename row: a filename input with keyboard
    /// handling, IME bridge, and live validation feedback.
    pub(crate) fn render_explorer_edit_row(
        &self,
        panel_id: PanelId,
        theme: &Theme,
        _cx: &mut App,
    ) -> AnyElement {
        let Some(edit) = self.edit.as_ref() else {
            return div().into_any_element();
        };
        let c = &theme.colors;
        let t = &theme.typography;
        let depth = edit.depth;
        let is_dir = edit.is_dir;
        let validation = edit.validation.clone();
        let focus_handle = edit.filename.focus_handle.clone().unwrap();

        let icon = if is_dir {
            (FOLDER_ICON, c.text_default)
        } else {
            (FILE_ICON, c.text_default)
        };

        let validation_label = match validation {
            Some(ExplorerValidation::Warning(message)) => Some((message, c.callout_warning_border)),
            Some(ExplorerValidation::Error(message)) => Some((message, c.callout_caution_border)),
            None => None,
        };

        div()
            .id(ElementId::Name(format!("explorer-edit-{panel_id}").into()))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .rounded(px(theme.dimensions.tree_item_radius))
            .bg(c.dialog_secondary_button_hover)
            // Clicks inside the edit row must not reach the panel
            // background (double-click there would create a new file).
            .on_click(|_event, _window, cx| cx.stop_propagation())
            // Arrow placeholder keeps the row aligned with siblings.
            .child(
                div()
                    .w(px(14.0))
                    .h(px(18.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().size(px(14.0))),
            )
            .child(
                svg()
                    .path(icon.0)
                    .size(px(19.0))
                    .flex_shrink_0()
                    .text_color(icon.1),
            )
            .child(
                div()
                    .id(("explorer-filename-input-box", panel_id.0))
                    .key_context("ExplorerFilenameInput")
                    .track_focus(&focus_handle)
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .on_key_down(move |event, window, cx| {
                        ExplorerState::update(cx, |state, cx| {
                            state.on_explorer_filename_key_down(event, window, cx);
                        });
                    })
                    // The global keymap binds escape to DismissTransientUi;
                    // GPUI dispatches matched actions BEFORE raw key
                    // listeners, so Esc must be handled as an action here
                    // (the focused node runs first) — on_key_down would
                    // never see it.
                    .on_action(move |action: &window::DismissTransientUi, window, cx| {
                        ExplorerState::update(cx, |state, cx| {
                            state.on_explorer_escape(action, window, cx);
                        });
                    })
                    .on_action(move |action: &window::Copy, _window, cx| {
                        ExplorerState::update(cx, |state, cx| {
                            state.on_explorer_filename_copy(action, _window, cx);
                        });
                    })
                    .on_action(move |action: &window::Cut, _window, cx| {
                        ExplorerState::update(cx, |state, cx| {
                            state.on_explorer_filename_cut(action, _window, cx);
                        });
                    })
                    .on_action(move |action: &window::Paste, window, cx| {
                        ExplorerState::update(cx, |state, cx| {
                            state.on_explorer_filename_paste(action, window, cx);
                        });
                    })
                    .child(ExplorerFilenameInputElement {
                        ime_host: edit.ime_host.clone().expect("ime host set on edit start"),
                    }),
            )
            .children(validation_label.map(|(message, color)| {
                div()
                    .max_w(px(160.0))
                    .truncate()
                    .text_size(px(t.text_size * 0.72))
                    .text_color(color)
                    .child(message)
                    .into_any_element()
            }))
            .into_any_element()
    }
}
