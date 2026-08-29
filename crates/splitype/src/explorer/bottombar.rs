use gpui::*;

use crate::app::shell::Shell;
use workspace::PanelId;
use theme::Theme;
use ui::bottombar::bottombar_container;
use ui::button::{icon_chip_button, toolbar_icon_size};

impl Shell {
    /// Bottom bar of an Explorer area: add-folder button plus the worktree
    /// count. The folder icon opens a folder picker that adds the chosen
    /// directory as a new worktree (mirrors the root row's folder button,
    /// minus the replace semantics).
    pub(crate) fn render_explorer_bottombar(
        &self,
        panel_id: PanelId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let shell_add = cx.entity().downgrade();
        let worktree_count = self.panels.explorer.worktrees.len();
        let btn_icon_size = toolbar_icon_size(d.bottombar_height);

        bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(("explorer-bottombar", panel_id.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        icon_chip_button(c, d)
                            .id(("explorer-bottombar-add-folder", panel_id.0))
                            .child(
                                svg()
                                    .path("icons/explorer/bottombar/new_folder.svg")
                                    .size(px(btn_icon_size))
                                    .text_color(c.text_default),
                            )
                            .on_click(move |_event, window, cx| {
                                let _ = shell_add.update(cx, |shell, cx| {
                                    shell.prompt_open_explorer_folder(window, cx);
                                });
                                cx.stop_propagation();
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .text_size(px(10.5))
                            .text_color(c.dialog_muted)
                            .child(if worktree_count == 1 {
                                "1 folder".to_string()
                            } else {
                                format!("{worktree_count} folders")
                            }),
                    ),
            )
            .into_any_element()
    }
}
