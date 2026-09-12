use std::path::PathBuf;

use gpui::*;

use crate::state::ExplorerState;

use config::language::I18nStrings;
use platform_contracts::PanelId;
use theme::Theme;

impl ExplorerState {
    pub(crate) fn render_explorer_empty_state(
        &self,
        title: &str,
        message: &str,
        panel_id: PanelId,
        theme: &Theme,
        _strings: &I18nStrings,
        recent_folders: &[PathBuf],
        recent_files: &[PathBuf],
        _cx: &mut App,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let drop_target_bg = c.panel_row_hover;
        let weak = self.self_weak.clone();

        let display_title = if title.is_empty() {
            "Explorer is empty now"
        } else {
            title
        };

        // An empty message means no special hint or error;
        // non-empty messages (e.g. scan errors) are rendered below the prompts.
        let has_message = !message.is_empty();

        // Pure, bright, luminous pale styling with crisp right-angles
        let is_dark = theme.appearance == theme::Appearance::Dark;
        let strip_bg = if is_dark {
            c.dialog_secondary_button_bg
        } else {
            hsla(220.0 / 360.0, 0.12, 0.98, 1.0)
        };
        let strip_text_color = c.dialog_muted;

        let has_recents = !recent_folders.is_empty() || !recent_files.is_empty();

        // ── Upper section: 40% height (hints left, long strip right) ────────
        let upper_section = div()
            .id(("explorer-empty-upper-section", panel_id.as_usize()))
            .w_full()
            .h(relative(0.40))
            .flex_shrink_0()
            .flex()
            .flex_row()
            // Dropping external folders/files directly opens them
            .drag_over::<ExternalPaths>(move |this, _, _, _| this.bg(drop_target_bg))
            .on_drop::<ExternalPaths>({
                let weak = weak.clone();
                move |paths, window, cx| {
                    let paths: Vec<_> = paths.paths().to_vec();
                    let _ = weak.update(cx, |state, cx| {
                        for path in paths {
                            if path.is_dir() {
                                state.open_explorer_folder_path(path.clone(), window, cx);
                            } else {
                                state.open_explorer_file(path.clone(), true, window, cx);
                            }
                        }
                    });
                }
            })
            // Left content area: Left-aligned prompt hints, closer to top, with button shifted right
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .justify_start()
                    .pt(px(14.0))
                    .pl(px(16.0))
                    .pr(px(12.0))
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(c.text_default)
                            .child(display_title.to_string()),
                    )
                    .child(if has_message {
                        div()
                            .max_w(px(260.0))
                            .text_size(px(12.0))
                            .text_color(c.callout_caution_border)
                            .child(message.to_string())
                    } else {
                        div()
                            .max_w(px(260.0))
                            .text_size(px(12.0))
                            .text_color(c.dialog_muted)
                            .child("Drop a folder here or open one to start:")
                    })
                    // Large rectangular button: transparent default, large hover area matching recent rows
                    .child(
                        div()
                            .id(("explorer-empty-open-btn", panel_id.as_usize()))
                            .cursor_pointer()
                            .mt(px(20.0))
                            .self_end()
                            .mr(px(20.0))
                            .w(px(240.0))
                            .h(px(72.0))
                            .rounded(px(d.tree_item_radius))
                            .hover(move |this| this.bg(c.panel_row_hover))
                            .active(move |this| this.bg(c.panel_row_hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(8.0))
                            .child(
                                svg()
                                    .path("plugin://splitype.explorer/worktree/big_plus.svg")
                                    .size(px(16.0))
                                    .text_color(c.text_default),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(c.text_default)
                                    .child("Open Folder"),
                            )
                            .on_click({
                                let weak = weak.clone();
                                move |_event, window, cx| {
                                    let _ = weak.update(cx, |state, cx| {
                                        state.prompt_open_explorer_folder(window, cx);
                                    });
                                }
                            }),
                    ),
            )
            // Right vertical strip: 28px wide, h_full (spans all of 40%), right angles, pure bright pale, no border
            .child(
                div()
                    .w(px(28.0))
                    .h_full()
                    .flex_shrink_0()
                    .rounded_none()
                    .bg(strip_bg)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_start()
                    .pt(px(20.0))
                    .child(
                        svg()
                            .path("plugin://splitype.explorer/worktree/strip_open_folder.svg")
                            .w(px(28.0))
                            .h(px(140.0))
                            .text_color(strip_text_color),
                    ),
            );

        // ── Lower section: 60% height (strip left extends to bottom, recents right) ──
        let lower_section = div()
            .id(("explorer-empty-lower-section", panel_id.as_usize()))
            .w_full()
            .h(relative(0.60))
            .min_h(px(0.0))
            .flex()
            .flex_row()
            // Left vertical strip: 28px wide, h_full (extends to bottom of panel), right angles, pure bright pale, no border
            .child(
                div()
                    .w(px(28.0))
                    .h_full()
                    .flex_shrink_0()
                    .rounded_none()
                    .bg(strip_bg)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_start()
                    .pt(px(20.0))
                    .child(
                        svg()
                            .path("plugin://splitype.explorer/worktree/strip_open_recent.svg")
                            .w(px(28.0))
                            .h(px(140.0))
                            .text_color(strip_text_color),
                    ),
            )
            // Right scrollable list of recent folders and files
            .child(
                div()
                    .id(("explorer-empty-recent-scroll", panel_id.as_usize()))
                    .flex_1()
                    .h_full()
                    .overflow_y_scroll()
                    .py(px(12.0))
                    .px(px(12.0))
                    .child(if !has_recents {
                        div()
                            .h_full()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.0))
                            .text_color(c.dialog_muted)
                            .child("No recent items")
                    } else {
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(recent_folders.iter().map(|path| {
                                let folder_name = path
                                    .file_name()
                                    .map(|name| name.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.to_string_lossy().to_string());
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
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(d.tree_item_radius))
                                    .hover(|this| this.bg(c.panel_row_hover))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        svg()
                                            .path("plugin://splitype.explorer/worktree/folder.svg")
                                            .size(px(15.0))
                                            .text_color(c.dialog_muted),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .text_size(px(12.0))
                                            .text_color(c.text_default)
                                            .child(folder_name),
                                    )
                                    .on_click({
                                        let weak = weak.clone();
                                        move |_event, window, cx| {
                                            let _ = weak.update(cx, |state, cx| {
                                                state.open_explorer_folder_path(
                                                    path.clone(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }
                                    })
                            }))
                            .children(recent_files.iter().map(|path| {
                                let file_name = path
                                    .file_name()
                                    .map(|name| name.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                                let path = path.clone();
                                div()
                                    .id(ElementId::Name(
                                        format!("explorer-recent-{}-{}", panel_id, path.display())
                                            .into(),
                                    ))
                                    .cursor_pointer()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(d.tree_item_radius))
                                    .hover(|this| this.bg(c.panel_row_hover))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        svg()
                                            .path("plugin://splitype.explorer/worktree/markdown.svg")
                                            .size(px(15.0))
                                            .text_color(c.dialog_muted),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .text_size(px(12.0))
                                            .text_color(c.text_default)
                                            .child(file_name),
                                    )
                                    .on_click({
                                        let weak = weak.clone();
                                        move |_event, window, cx| {
                                            let _ = weak.update(cx, |state, cx| {
                                                state.open_explorer_file(
                                                    path.clone(),
                                                    true,
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }
                                    })
                            }))
                    }),
            );

        div()
            .id(("explorer-empty-state-root", panel_id.as_usize()))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .justify_between()
            .overflow_hidden()
            .child(upper_section)
            .child(lower_section)
            .into_any_element()
    }
}
