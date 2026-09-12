use gpui::prelude::*;
use gpui::*;

use crate::state::ExplorerState;
use crate::state::{
    EXPLORER_NODE_HEIGHT, EXPLORER_NODE_INDENT, ExplorerValidation, FOLDER_ICON, file_type_icon,
};
use platform_contracts::PanelId;
use theme::Theme;

impl ExplorerState {
    /// Render the inline create/rename row: a borderless single-line editor
    /// embedded directly into the tree row (mirrors Zed's inline editor).
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
        let filename = self.filename_editor().read(cx).text().to_string();
        let trimmed_name = filename.trim();
        let is_dir = edit.is_dir
            || (edit.target_id.is_none()
                && (trimmed_name.ends_with('/') || trimmed_name.ends_with('\\')));
        let validation = edit.validation.clone();

        let editor = self.filename_editor();
        let is_focused = editor.read(cx).is_focused(window);
        if !is_focused {
            let focus_handle = editor.read(cx).focus_handle().clone();
            window.focus(&focus_handle, cx);
        }

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
            // Clicks inside the edit row must not reach panel background
            .on_click(|_event, _window, cx| cx.stop_propagation())
            // Arrow placeholder keeps row aligned with siblings
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
            // Borderless native single-line filename editor
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h(px(EXPLORER_NODE_HEIGHT))
                    .flex()
                    .items_center()
                    .child(editor.clone()),
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
