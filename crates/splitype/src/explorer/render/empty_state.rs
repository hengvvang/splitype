use std::path::PathBuf;

use gpui::*;

use crate::app::shell::Shell;
use crate::app::window::panels::PanelId;
use i18n::I18nStrings;
use theme::Theme;
use ui::empty_state::empty_state_container;

impl Shell {
    pub(crate) fn render_explorer_empty_state(
        &self,
        title: &str,
        message: &str,
        panel_id: PanelId,
        theme: &Theme,
        strings: &I18nStrings,
        recent_folders: &[PathBuf],
        recent_files: &[PathBuf],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let click_shell = cx.entity().downgrade();
        let drop_target_bg = c.dialog_secondary_button_hover;

        let display_title = if title.is_empty() {
            "Explorer is empty now"
        } else {
            title
        };

        // An empty message means the empty state has no hint line at all;
        // non-empty messages (e.g. scan errors) are still rendered.
        let has_message = !message.is_empty();

        empty_state_container(("explorer-empty-state-scroll", panel_id.0))
            .gap(px(10.0))
            .px(px(24.0))
            .pt(px(96.0))
            .pb(px(24.0))
            // Dropping folders onto the empty state opens them as worktrees
            // (mirrors Zed's empty-state drop-to-open).
            .drag_over::<ExternalPaths>(move |this, _, _, _| this.bg(drop_target_bg))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(|shell, paths, window, cx| {
                for path in paths.paths() {
                    if path.is_dir() {
                        shell.open_explorer_folder_path(path.clone(), cx);
                    } else {
                        shell.open_explorer_file(
                            path.clone(),
                            crate::editor::engine::controller::OpenFileMode::Persistent,
                            window,
                            cx,
                        );
                    }
                }
            }))
            .child(
                svg()
                    .path("icons/explorer/worktree/folder.svg")
                    .size(px(40.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c.text_default)
                    .child(display_title.to_string()),
            )
            .child(if has_message {
                div()
                    .max_w(px(230.0))
                    .text_size(px(t.text_size * 0.78))
                    .line_height(px(t.text_size * t.text_line_height * 0.90))
                    .text_color(c.dialog_muted)
                    .child(message.to_string())
            } else {
                div()
            })
            .child(
                div()
                    .id(("explorer-empty-open-btn", panel_id.0))
                    .cursor_pointer()
                    .mt(px(4.0))
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .rounded(px(d.button_radius))
                    .border_1()
                    .border_color(c.dialog_border)
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.92))
                    .child(
                        svg()
                            .path("icons/explorer/worktree/open_folder.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_secondary_button_text),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.dialog_secondary_button_text)
                            .child("Open Folder"),
                    )
                    .on_click(move |_event, window, cx| {
                        let _ = click_shell.update(cx, |shell, cx| {
                            shell.prompt_open_explorer_folder(window, cx);
                        });
                    }),
            )
            .child(
                // Recent folders and files quick-open list under the button;
                // hidden when both histories are empty or the state carries
                // an error message.
                if (recent_folders.is_empty() && recent_files.is_empty()) || has_message {
                    div()
                } else {
                    div()
                        .mt(px(16.0))
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(2.0))
                        .child(
                            div()
                                .ml(px(10.0))
                                .text_size(px(14.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c.dialog_muted)
                                .child(strings.explorer_recent_title.clone()),
                        )
                        .children(recent_folders.iter().map(|path| {
                            let folder_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let item_shell = cx.entity().downgrade();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!(
                                        "explorer-recent-folder-{}-{}",
                                        panel_id,
                                        path.display()
                                    )
                                    .into(),
                                ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(3.0))
                                .rounded(px(d.tree_item_radius))
                                .hover(|this| this.bg(c.panel_row_hover))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    svg()
                                        .path("icons/explorer/worktree/folder.svg")
                                        .size(px(16.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(200.0))
                                        .truncate()
                                        .text_size(px(13.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(folder_name),
                                )
                                .on_click(move |_event, _window, cx| {
                                    let _ = item_shell.update(cx, |shell, cx| {
                                        shell.open_explorer_folder_path(path.clone(), cx);
                                    });
                                })
                        }))
                        .children(recent_files.iter().map(|path| {
                            let file_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let item_shell = cx.entity().downgrade();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!("explorer-recent-{}-{}", panel_id, path.display())
                                        .into(),
                                 ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(3.0))
                                .rounded(px(d.tree_item_radius))
                                .hover(|this| this.bg(c.panel_row_hover))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    svg()
                                        .path("icons/explorer/worktree/markdown.svg")
                                        .size(px(16.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(200.0))
                                        .truncate()
                                        .text_size(px(13.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(file_name),
                                )
                                .on_click(move |_event, window, cx| {
                                    let _ = item_shell.update(cx, |shell, cx| {
                                        shell.open_explorer_file(
                                            path.clone(),
                                            crate::editor::engine::controller::OpenFileMode::Persistent,
                                            window,
                                            cx,
                                        );
                                    });
                                })
                        }))
                },
            )
            .into_any_element()
    }
}
