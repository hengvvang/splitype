use gpui::prelude::*;
use gpui::*;

use crate::state::ExplorerState;

use crate::filename_editor::ExplorerFilenameInputElement;
use crate::state::{
    EXPLORER_NODE_HEIGHT, EXPLORER_NODE_INDENT, ExplorerValidation, FOLDER_ICON, file_type_icon,
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
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let Some(edit) = self.edit.as_ref() else {
            return div().into_any_element();
        };
        let c = &theme.colors;
        let t = &theme.typography;
        let depth = edit.depth;
        let trimmed_name = edit.filename.text.trim();
        let is_dir = edit.is_dir
            || (edit.target_id.is_none()
                && (trimmed_name.ends_with('/') || trimmed_name.ends_with('\\')));
        let validation = edit.validation.clone();
        let focus_handle = edit.filename.focus_handle.clone().unwrap();
        if !focus_handle.is_focused(window) {
            focus_handle.focus(window, cx);
        }
        let weak = self.self_weak.clone();
        let state_entity = self
            .self_weak
            .upgrade()
            .expect("explorer state entity alive while rendering the edit row");

        let icon = if is_dir {
            (FOLDER_ICON, c.text_default)
        } else {
            let ext = std::path::Path::new(trimmed_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            (file_type_icon(&ext), c.text_default)
        };

        let validation_label = match &validation {
            Some(ExplorerValidation::Warning(message)) => {
                Some((message.clone(), c.callout_warning_border))
            }
            Some(ExplorerValidation::Error(message)) => {
                Some((message.clone(), c.callout_caution_border))
            }
            None => None,
        };

        let guide_color = theme
            .token("splitype.explorer.indent_guide")
            .unwrap_or_else(|| c.separator.opacity(0.35));

        div()
            .id(ElementId::Name(format!("explorer-edit-{panel_id}").into()))
            .relative()
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(c.panel_row_hover)
            .children((0..depth).map(|k| {
                div()
                    .absolute()
                    .left(px(13.0 + k as f32 * EXPLORER_NODE_INDENT))
                    .top_0()
                    .bottom_0()
                    .w(px(1.0))
                    .bg(guide_color)
            }))
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
                    .id(("explorer-filename-input-box", panel_id.as_usize()))
                    .key_context("ExplorerFilenameInput")
                    .track_focus(&focus_handle)
                    .flex_1()
                    .min_w(px(0.0))
                    .h(px(EXPLORER_NODE_HEIGHT))
                    .flex()
                    .items_center()
                    .on_key_down({
                        let weak = weak.clone();
                        move |event, window, cx| {
                            let handled = weak
                                .update(cx, |state, cx| {
                                    state.on_explorer_filename_key_down(event, window, cx)
                                })
                                .unwrap_or(false);
                            if handled {
                                cx.stop_propagation();
                            }
                        }
                    })
                    .on_action({
                        let weak = weak.clone();
                        move |_: &crate::ops::selection::TrashSelectedEntry, window, cx| {
                            cx.stop_propagation();
                            let _ = weak.update(cx, |state, cx| {
                                if let Some(edit) = state.edit.as_mut() {
                                    if let Some(marked) = edit.filename.marked_range.take() {
                                        edit.filename.replace_range(marked, "");
                                    } else {
                                        edit.filename.delete_backward();
                                    }
                                }
                                state.populate_explorer_validation(cx);
                                state.autoscroll_explorer_edit(window, cx);
                            });
                        }
                    })
                    .on_action({
                        let weak = weak.clone();
                        move |_: &crate::ops::selection::DeleteSelectedEntry, window, cx| {
                            cx.stop_propagation();
                            let _ = weak.update(cx, |state, cx| {
                                if let Some(edit) = state.edit.as_mut() {
                                    if let Some(marked) = edit.filename.marked_range.take() {
                                        edit.filename.replace_range(marked, "");
                                    } else {
                                        edit.filename.delete_forward();
                                    }
                                }
                                state.populate_explorer_validation(cx);
                                state.autoscroll_explorer_edit(window, cx);
                            });
                        }
                    })
                    .on_action({
                        move |_: &crate::ops::selection::DuplicateSelectedEntry, _window, cx| {
                            cx.stop_propagation();
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
                    .on_action({
                        let weak = weak.clone();
                        move |action: &platform_contracts::actions::SelectAll, window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_select_all(action, window, cx);
                            });
                        }
                    })
                    .on_action({
                        move |_: &crate::ops::selection::UndoFileOperation, _window, cx| {
                            cx.stop_propagation();
                        }
                    })
                    .on_action({
                        move |_: &crate::ops::selection::RedoFileOperation, _window, cx| {
                            cx.stop_propagation();
                        }
                    })
                    .on_mouse_down(MouseButton::Left, {
                        let weak = weak.clone();
                        move |event, window, cx| {
                            cx.stop_propagation();
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_mouse_down(event, window, cx);
                            });
                        }
                    })
                    .on_mouse_up(MouseButton::Left, {
                        let weak = weak.clone();
                        move |event, window, cx| {
                            cx.stop_propagation();
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_mouse_up(event, window, cx);
                            });
                        }
                    })
                    .on_mouse_move({
                        let weak = weak.clone();
                        move |event, window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.on_explorer_filename_mouse_move(event, window, cx);
                            });
                        }
                    })
                    .child(ExplorerFilenameInputElement {
                        ime_host: edit.ime_host.clone().expect("ime host set on edit start"),
                        state: state_entity,
                    }),
            )
            .when_some(validation_label, |this, (message, color)| {
                this.child(deferred(
                    div()
                        .occlude()
                        .absolute()
                        .top(px(EXPLORER_NODE_HEIGHT + 2.0))
                        .left(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT + 42.0))
                        .right(px(8.0))
                        .py(px(4.0))
                        .px(px(8.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(color)
                        .bg(c.editor_background)
                        .shadow_md()
                        .text_size(px(t.text_size * 0.82))
                        .text_color(color)
                        .child(message),
                ))
            })
            .into_any_element()
    }
}
