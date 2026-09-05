//! Standalone settings window — hosts the schema-driven settings UI
//! in its own top-level window.

use gpui::*;

use config::language::I18nManager;
use theme::ThemeManager;
use crate::chrome::custom_titlebar::{
    custom_titlebar_height, render_custom_titlebar, splitype_window_options,
};

use settings::{SettingsUiState, render_settings_body};

/// Independent standalone settings window view.
pub struct SettingsWindow {
    pub(crate) state: Entity<SettingsUiState>,
}

impl SettingsWindow {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: cx.new(|_cx| SettingsUiState::new()),
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeManager>().current().clone();
        let strings = cx.global::<I18nManager>().strings().clone();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let window_title = SharedString::from(strings.settings_window_title.clone());
        window.set_window_title(window_title.as_ref());
        let titlebar_height = custom_titlebar_height(window, d);

        let body = render_settings_body("window", self.state.clone(), &theme, cx);

        let main_body = div()
            .flex()
            .flex_row()
            .h_full()
            .pt(px(titlebar_height))
            .child(body);

        let base = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(c.editor_background)
            .child(main_body);

        if let Some(titlebar) = render_custom_titlebar(
            "win-pref-titlebar",
            window_title,
            None,
            &theme,
            window,
            cx,
            Self::on_titlebar_close,
        ) {
            base.child(titlebar)
        } else {
            base
        }
    }
}

impl SettingsWindow {
    fn on_titlebar_close(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if event.standard_click() {
            window.remove_window();
        }
    }
}

/// Opens the standalone settings window, reusing it when already open.
pub fn open_settings_window(cx: &mut App) -> Option<WindowHandle<SettingsWindow>> {
    if let Some(handle) = cx
        .windows()
        .into_iter()
        .find_map(|window| window.downcast::<SettingsWindow>())
    {
        let _ = handle.update(cx, |_settings, window, _cx| {
            window.activate_window();
        });
        return Some(handle);
    }

    let bounds = Bounds::centered(None, size(px(760.0), px(520.0)), cx);
    let title = cx
        .global::<I18nManager>()
        .strings()
        .settings_window_title
        .clone();
    let window_title = SharedString::from(title);

    let handle = match cx.open_window(
        splitype_window_options(window_title, bounds),
        move |_window, cx| cx.new(SettingsWindow::new),
    ) {
        Ok(handle) => handle,
        Err(error) => {
            tracing::error!(%error, "failed to open settings window");
            return None;
        }
    };

    let _ = handle.update(cx, |_settings, window, _cx| {
        window.activate_window();
    });

    Some(handle)
}
