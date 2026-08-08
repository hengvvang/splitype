//! Bottom bar of an Explorer area: the add-folder button and the worktree
//! count.

use gpui::*;

use crate::theme::Theme;
use crate::ui::components::bottombar::bottombar_container;
use crate::ui::components::button::icon_chip_button;

impl crate::editor::controller::Editor {
    /// Bottom bar of an Explorer area: add-folder button plus the worktree
    /// count. The folder icon opens a folder picker that adds the chosen
    /// directory as a new worktree (mirrors the root row's folder button,
    /// minus the replace semantics).
    pub(crate) fn render_explorer_bottombar(
        &self,
        area_id: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let ed_add = cx.entity().downgrade();
        let worktree_count = self.panels.explorer.worktrees.len();

        bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(("explorer-bottombar", area_id))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        icon_chip_button(c, d)
                            .id(("ws-bottombar-add-folder", area_id))
                            .child(
                                svg()
                                    .path("icons/explorer/bottombar/folder-plus.svg")
                                    .size(px(16.0))
                                    .text_color(c.text_default),
                            )
                            .on_click(move |_ev, window, cx| {
                                let _ = ed_add.update(cx, |ed, cx| {
                                    ed.prompt_open_explorer_folder(window, cx);
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
