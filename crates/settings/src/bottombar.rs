//! Bottom status bar of the Settings panel: search input box and match statistics.

use gpui::*;
use theme::{Theme, ThemeColors};
use ui::SearchInput;

use crate::state::SettingsUiState;

fn bottombar_container(c: &ThemeColors, height: f32, padding_x: f32) -> Div {
    div()
        .h(px(height))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(padding_x))
        .bg(c.dialog_surface)
        .border_t_1()
        .border_color(c.dialog_border)
}

/// Free-function entry point: renders the settings bottom bar with search input.
pub fn render_settings_bottombar(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let search_query = state.read(cx).search_query.clone();
    let search_focus = state.update(cx, |ui, cx| ui.search_focus_handle(cx));
    let weak_state = state.downgrade();

    let on_change_state = weak_state.clone();
    let on_dismiss_state = weak_state;

    let search_input_id = ElementId::Name(format!("{id_namespace}-settings-search").into());

    let search_input = SearchInput::new(
        search_input_id,
        search_query.clone(),
        search_focus,
    )
    .placeholder("Search settings…")
    .colors(c.clone())
    .dimensions(d.clone())
    .show_clear_button(true)
    .on_change(move |new_query, _window, cx| {
        if let Some(state) = on_change_state.upgrade() {
            state.update(cx, |ui, _| {
                ui.search_query = new_query;
            });
            cx.refresh_windows();
        }
    })
    .on_dismiss(move |_window, cx| {
        if let Some(state) = on_dismiss_state.upgrade() {
            state.update(cx, |ui, _| {
                ui.clear_search();
            });
            cx.refresh_windows();
        }
    });

    let count_label = if !search_query.trim().is_empty() {
        let count = crate::host::count_total_search_matches(&search_query);
        let text = match count {
            0 => "No results".to_string(),
            1 => "1 result".to_string(),
            n => format!("{n} results"),
        };
        Some(
            div()
                .text_size(px(11.5))
                .text_color(c.dialog_muted)
                .child(text),
        )
    } else {
        None
    };

    let bar_height = (d.bottombar_height + 8.0).max(36.0);
    let padding_x = d.bottombar_padding_x.max(8.0);

    bottombar_container(c, bar_height, padding_x)
        .id((
            SharedString::from(format!("{id_namespace}-bottombar")),
            0_usize,
        ))
        .child(
            div()
                .w(px(220.0))
                .flex_shrink_0()
                .child(search_input),
        )
        .children(count_label)
        .into_any_element()
}
