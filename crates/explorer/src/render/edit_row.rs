use gpui::*;

use crate::state::ExplorerState;

use crate::filename_editor::ExplorerFilenameInputElement;
use crate::state::{
    EXPLORER_NODE_HEIGHT, EXPLORER_NODE_INDENT, ExplorerValidation, FILE_ICON, FOLDER_ICON,
};
use platform_contracts::PanelId;
use theme::Theme;

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
        let weak = self.self_weak.clone();
        let state_entity = self
            .self_weak
            .upgrade()
            .expect("explorer state entity alive while rendering the edit row");

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
            .relative()
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(c.panel_row_hover)
            .children(Some(
                div()
                    .absolute()
                    .left_0()
                    .top(px(4.0))
                    .bottom(px(4.0))
                    .w(px(3.0))
                    .rounded_r(px(2.0))
                    .bg(c.focus_accent),
            ))
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
                    .on_key_down({
                        let weak = weak.clone();
                        move |event, window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_key_down(event, window, cx);
                            });
                        }
                    })
                    // The global keymap binds escape to DismissTransientUi;
                    // GPUI dispatches matched actions BEFORE raw key
                    // listeners, so Esc must be handled as an action here
                    // (the focused node runs first) — on_key_down would
                    // never see it.
                    .on_action({
                        let weak = weak.clone();
                        move |action: &platform_contracts::actions::DismissTransientUi, window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_escape(action, window, cx);
                            });
                        }
                    })
                    .on_action({
                        let weak = weak.clone();
                        move |action: &platform_contracts::actions::Copy, _window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_copy(action, _window, cx);
                            });
                        }
                    })
                    .on_action({
                        let weak = weak.clone();
                        move |action: &platform_contracts::actions::Cut, _window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_cut(action, _window, cx);
                            });
                        }
                    })
                    .on_action({
                        let weak = weak.clone();
                        move |action: &platform_contracts::actions::Paste, window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_paste(action, window, cx);
                            });
                        }
                    })
                    .child(ExplorerFilenameInputElement {
                        ime_host: edit.ime_host.clone().expect("ime host set on edit start"),
                        state: state_entity,
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
