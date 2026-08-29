//! Welcome screen rendered when an editor area has no open document.

use gpui::*;

use crate::editor::engine::controller::*;
use theme::Theme;

impl Editor {
    pub(crate) fn render_welcome_prompt(
        &mut self,
        pane_id: impl Into<PaneId>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pane_id = pane_id.into();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let panel_id = self.panel_id;
        let editor = cx.entity().downgrade();

        div()
            .id(ElementId::Name(
                format!("welcome-prompt-{}-{pane_id}", panel_id.0, pane_id = pane_id.0).into(),
            ))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .bg(c.editor_background)
            .cursor_pointer()
            // GPUI has no double-click event; track click timestamps in
            // editor state (closure-local state is rebuilt every frame).
            .on_click(move |_event, window, cx| {
                let now = std::time::Instant::now();
                let _ = editor.update(cx, |ed, cx| {
                    let is_double = ed.welcome_last_click.is_some_and(|previous| {
                        now.duration_since(previous) < std::time::Duration::from_millis(500)
                    });
                    ed.welcome_last_click = Some(now);
                    if is_double {
                        // The clicked editor becomes the active editor
                        // (deferred: the Shell re-pushes state to every
                        // editor, and this one is mid-update).
                        ed.defer_shell_action(cx, move |shell, cx| {
                            shell.activate_panel(panel_id, cx);
                        });
                        ed.new_untitled_tab(cx);
                        // Focus the new source panel so typing works
                        // immediately after entering editing.
                        ed.focus_pane(pane_id, window, cx);
                    }
                });
            })
            .child(
                div()
                    .text_size(px(d.menu_text_size.max(13.0)))
                    .text_color(c.text_default)
                    .font_weight(FontWeight::MEDIUM)
                    .child("Double-click to start editing"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.dialog_muted)
                    .child("Or open a file from the explorer or menus"),
            )
            .into_any_element()
    }
}
