//! Schema-driven settings host — renders the settings UI entirely from the
//! plugin registry's manifest-declared settings schemas.
//!
//! The host knows nothing about what any setting configures: it walks
//! [`PluginRegistry`] for manifests with `settings` declarations, draws the
//! navigation from the plugin names, and renders one control per declaration
//! from its [`SettingKind`]. Values are read from and written to the
//! canonical [`SettingsStore`] through dotted keys; theme selections apply
//! live through the settings sync hook, and the group declaring the theme
//! family picker also hosts the per-user color override panel.

use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::*;
use serde_json::Value;

use config::language::I18nManager;
use config::settings::{CoreSettings, PluginSettings, SettingsStore};
use platform_contracts::{PluginManifest, PluginRegistry, SettingDeclaration, SettingKind};
use theme::{Theme, ThemeColors, ThemeDimensions, ThemeManager};
use ui::section::section_card;
use ui::select::{select_option, select_panel, select_trigger};
use ui::settings_form::{
    NumberFieldProps, SearchableFontPickerProps, SettingsClickHandler, SettingsKeyHandler,
    make_row_with_reset, render_number_field, render_searchable_font_picker,
};
use ui::switch::Switch;
use ui::tab::nav_tab;

use crate::state::SettingsUiState;

/// One selectable entry of a picker control.
struct PickerOption {
    value: String,
    label: String,
}

/// Renders the two-column settings body: navigation rail over the plugins
/// that declare settings, and the active plugin's settings page.
pub fn render_settings_body(
    id_namespace: &str,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let manifests: Vec<Arc<PluginManifest>> =
        PluginRegistry::registered_manifests().unwrap_or_default();
    let settings_plugins: Vec<Arc<PluginManifest>> = manifests
        .into_iter()
        .filter(|manifest| !manifest.settings.is_empty())
        .collect();

    let active_id = state.read(cx).active_plugin.clone();
    let active = settings_plugins
        .iter()
        .find(|manifest| manifest.plugin.as_str() == active_id)
        .or_else(|| settings_plugins.first())
        .cloned();

    // Left navigation: one entry per plugin with declared settings.
    let mut nav_items = Vec::new();
    for manifest in &settings_plugins {
        let plugin_id = manifest.plugin.as_str().to_string();
        let is_active = active
            .as_ref()
            .is_some_and(|active| active.plugin.as_str() == plugin_id);
        let label = manifest.name.clone();
        let nav_state = state.clone();
        let mut tab = nav_tab(
            ElementId::Name(format!("{id_namespace}-nav-{plugin_id}").into()),
            c,
            d,
        )
        .relative()
        .when(is_active, |this| this.bg(c.panel_row_hover))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(if is_active {
                    c.text_default
                } else {
                    c.dialog_muted
                })
                .child(label),
        )
        .on_click(move |_event, _window, cx| {
            nav_state.update(cx, |ui, _| {
                ui.active_plugin = plugin_id.clone();
            });
            cx.refresh_windows();
        });

        if is_active {
            tab = tab.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(8.0))
                    .bottom(px(8.0))
                    .w(px(3.0))
                    .rounded_full()
                    .bg(c.focus_accent),
            );
        }

        nav_items.push(tab.into_any_element());
    }

    let nav_rail = div()
        .w(px(160.0))
        .h_full()
        .flex_shrink_0()
        .p(px(8.0))
        .border_r_1()
        .border_color(c.dialog_border)
        .flex()
        .flex_col()
        .gap(px(2.0))
        .children(nav_items);

    let content = match &active {
        Some(manifest) => render_plugin_page(id_namespace, &state, manifest, theme, cx),
        None => div().into_any_element(),
    };

    let right_content = div()
        .id((
            SharedString::from(format!("{id_namespace}-content")),
            0_usize,
        ))
        .relative()
        .flex_1()
        .h_full()
        .p(px(14.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(content);

    div()
        .w_full()
        .h_full()
        .flex()
        .flex_row()
        .bg(c.editor_background)
        .child(nav_rail)
        .child(right_content)
        .into_any_element()
}

/// Renders one plugin's settings page: a page header plus standalone setting cards.
fn render_plugin_page(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    manifest: &PluginManifest,
    theme: &Theme,
    cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let plugin_id = manifest.plugin.as_str();

    let groups = settings_groups(manifest);
    let has_multiple_groups =
        groups.len() > 1 || groups.first().is_some_and(|(g, _)| g != &manifest.name);

    let mut section_elements = Vec::new();
    for (group, declarations) in groups {
        let rows: Vec<AnyElement> = declarations
            .iter()
            .map(|declaration| {
                render_setting_row(id_namespace, state, plugin_id, declaration, theme, cx)
            })
            .collect();

        let mut group_div = div().flex().flex_col().gap(px(4.0));
        if has_multiple_groups {
            group_div = group_div.child(
                div()
                    .pt(px(6.0))
                    .pb(px(2.0))
                    .text_size(px(13.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.text_default)
                    .child(group),
            );
        }
        group_div = group_div.children(rows);
        section_elements.push(group_div.into_any_element());

        // The group declaring the theme family picker also hosts the
        // per-user color override panel.
        if declarations
            .iter()
            .any(|declaration| declaration.kind == SettingKind::Theme)
        {
            section_elements.push(render_theme_overrides_panel(id_namespace, state, theme, cx));
        }
    }

    let description = manifest.description.clone().map(|text| {
        div()
            .text_size(px(12.0))
            .text_color(c.dialog_muted)
            .child(text)
    });

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(c.text_default)
                        .child(manifest.name.clone()),
                )
                .children(description),
        )
        .children(section_elements)
        .into_any_element()
}

/// Groups a plugin's declarations: flat keys form one group named after the
/// plugin; dotted keys group by their first segment (e.g. `startup.open` →
/// `Startup`).
fn settings_groups(manifest: &PluginManifest) -> Vec<(String, Vec<SettingDeclaration>)> {
    if !manifest
        .settings
        .iter()
        .any(|declaration| declaration.key.contains('.'))
    {
        return vec![(manifest.name.clone(), manifest.settings.clone())];
    }
    let mut grouped: std::collections::BTreeMap<String, Vec<SettingDeclaration>> =
        std::collections::BTreeMap::new();
    for declaration in &manifest.settings {
        let group = declaration
            .key
            .split('.')
            .next()
            .unwrap_or(&declaration.key)
            .to_string();
        grouped.entry(group).or_default().push(declaration.clone());
    }
    grouped
        .into_iter()
        .map(|(group, declarations)| (title_case(&group), declarations))
        .collect()
}

/// Renders one declaration as a settings row with the control matching its
/// kind and a reset-to-default action.
fn render_setting_row(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    plugin_id: &str,
    declaration: &SettingDeclaration,
    theme: &Theme,
    cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let border_color = c.dialog_border;

    let current = current_value(plugin_id, declaration, cx);
    let control = render_control(
        id_namespace,
        state,
        plugin_id,
        declaration,
        &current,
        theme,
        cx,
    );

    let reset_plugin = plugin_id.to_string();
    let reset_key = declaration.key.clone();
    let reset_default = declaration.default.clone();
    let reset = Box::new(
        move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
            let _ = SettingsStore::set_plugin_value(
                cx,
                &reset_plugin,
                &reset_key,
                reset_default.clone(),
            );
        },
    );

    make_row_with_reset(
        border_color,
        c,
        d,
        declaration.title.clone(),
        declaration.description.clone().unwrap_or_default(),
        Some(reset),
        control,
    )
}

/// The effective value of a declaration: the stored value when it exists and
/// satisfies the declaration, otherwise the declared default.
fn current_value(plugin_id: &str, declaration: &SettingDeclaration, cx: &App) -> Value {
    SettingsStore::get(cx)
        .plugin_value(plugin_id, &declaration.key)
        .filter(|value| declaration.accepts(value))
        .unwrap_or_else(|| declaration.default.clone())
}

/// Renders the control for one declaration, dispatching on its kind.
fn render_control(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    plugin_id: &str,
    declaration: &SettingDeclaration,
    current: &Value,
    theme: &Theme,
    cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let key = declaration.key.clone();

    match &declaration.kind {
        SettingKind::Bool => {
            let checked = current.as_bool().unwrap_or(false);
            let plugin = plugin_id.to_string();
            Switch::new(format!("{id_namespace}-switch-{key}"))
                .checked(checked)
                .on_click(Box::new(
                    move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        let _ = SettingsStore::set_plugin_value(
                            cx,
                            &plugin,
                            &key,
                            Value::Bool(!checked),
                        );
                    },
                ))
                .into_any_element()
        }
        SettingKind::Number => render_number_control(
            id_namespace,
            state,
            plugin_id,
            declaration,
            current,
            c,
            d,
            cx,
        ),
        SettingKind::String => render_text_control(
            id_namespace,
            state,
            plugin_id,
            declaration,
            current,
            c,
            d,
            cx,
        ),
        SettingKind::Enum => render_picker(
            id_namespace,
            state,
            plugin_id,
            declaration,
            current,
            declaration
                .options
                .iter()
                .map(|option| PickerOption {
                    value: option.value.clone(),
                    label: option.label.clone(),
                })
                .collect(),
            current.as_str().map(|value| {
                declaration
                    .options
                    .iter()
                    .find(|option| option.value == value)
                    .map(|option| option.label.clone())
                    .unwrap_or_else(|| value.to_string())
            }),
            c,
            d,
            |_cx, _value| {},
            cx,
        ),
        SettingKind::Font => {
            let fonts: Vec<SharedString> = cx
                .text_system()
                .all_font_names()
                .into_iter()
                .map(SharedString::from)
                .collect();
            let stored = current.as_str().unwrap_or_default();
            let current_name = if stored.is_empty() {
                "Default".to_string()
            } else {
                stored.to_string()
            };
            let is_open = state.read(cx).open_picker.as_deref() == Some(key.as_str());
            let search_query = state
                .read(cx)
                .search_queries
                .get(&key)
                .cloned()
                .unwrap_or_default();
            let focus_handle = state.update(cx, |ui, cx| ui.focus_handle(&key, cx));

            let toggle_state = state.clone();
            let toggle_key = key.clone();
            let focus_handle_for_toggle = focus_handle.clone();
            let on_toggle = Box::new(
                move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                    let will_open = toggle_state.update(cx, |ui, _| {
                        ui.open_picker = if ui.open_picker.as_deref() == Some(toggle_key.as_str()) {
                            None
                        } else {
                            Some(toggle_key.clone())
                        };
                        ui.open_picker.is_some()
                    });
                    if will_open {
                        window.focus(&focus_handle_for_toggle, cx);
                    }
                    cx.refresh_windows();
                },
            );

            let search_state = state.clone();
            let search_key = key.clone();
            let on_search_change =
                Box::new(move |query: String, _window: &mut Window, cx: &mut App| {
                    search_state.update(cx, |ui, _| {
                        ui.search_queries.insert(search_key.clone(), query);
                    });
                    cx.refresh_windows();
                });

            let select_state = state.clone();
            let select_plugin = plugin_id.to_string();
            let select_key = key.clone();
            let on_select_font: ui::settings_form::SettingsOptionHandler<String> =
                Box::new(move |font: String| {
                    let select_state = select_state.clone();
                    let select_plugin = select_plugin.clone();
                    let select_key = select_key.clone();
                    Box::new(move |_event, _window, cx| {
                        let stored = if font == "default" {
                            String::new()
                        } else {
                            font.clone()
                        };
                        let _ = SettingsStore::set_plugin_value(
                            cx,
                            &select_plugin,
                            &select_key,
                            Value::String(stored),
                        );
                        select_state.update(cx, |ui, _| ui.open_picker = None);
                    })
                });

            render_searchable_font_picker(
                c,
                d,
                SearchableFontPickerProps {
                    id_prefix: format!("{id_namespace}-font-{key}"),
                    current_font_name: current_name,
                    default_label: "Default".to_string(),
                    is_open,
                    search_query,
                    focus_handle,
                    on_toggle,
                    on_search_change,
                    available_fonts: fonts,
                    on_select_font,
                },
            )
        }
        SettingKind::Theme => {
            // One option per family; user families shadow plugin/builtin
            // families with the same id.
            let mut seen = std::collections::BTreeSet::new();
            let options: Vec<PickerOption> = cx
                .global::<ThemeManager>()
                .available_themes()
                .iter()
                .filter(|entry| seen.insert(entry.family.clone()))
                .map(|entry| PickerOption {
                    value: entry.family.clone(),
                    label: if entry.author.is_empty() {
                        entry.family_name.clone()
                    } else {
                        format!("{} · {}", entry.family_name, entry.author)
                    },
                })
                .collect();
            let label = current.as_str().map(|id| {
                options
                    .iter()
                    .find(|option| option.value == id)
                    .map(|option| option.label.clone())
                    .unwrap_or_else(|| id.to_string())
            });
            render_picker(
                id_namespace,
                state,
                plugin_id,
                declaration,
                current,
                options,
                label,
                c,
                d,
                // The settings write below triggers the theme sync hook,
                // which resolves and applies the selected family live.
                |_cx, _value| {},
                cx,
            )
        }
        SettingKind::Language => {
            let options: Vec<PickerOption> = cx
                .global::<I18nManager>()
                .available_languages()
                .iter()
                .map(|entry| PickerOption {
                    value: entry.id.clone(),
                    label: entry.name.clone(),
                })
                .collect();
            let label = current.as_str().map(|id| {
                options
                    .iter()
                    .find(|option| option.value == id)
                    .map(|option| option.label.clone())
                    .unwrap_or_else(|| id.to_string())
            });
            render_picker(
                id_namespace,
                state,
                plugin_id,
                declaration,
                current,
                options,
                label,
                c,
                d,
                |cx, language_id| {
                    let _ = cx.update_global::<I18nManager, _>(|manager, _cx| {
                        manager.set_language_by_id(&language_id)
                    });
                },
                cx,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_number_control(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    plugin_id: &str,
    declaration: &SettingDeclaration,
    current: &Value,
    c: &theme::ThemeColors,
    d: &theme::ThemeDimensions,
    cx: &mut App,
) -> AnyElement {
    let key = declaration.key.clone();
    let step = declaration.step.unwrap_or(1.0);
    let min = declaration.min;
    let max = declaration.max;
    let number = current
        .as_f64()
        .or_else(|| current.as_u64().map(|value| value as f64))
        .unwrap_or_else(|| declaration.default.as_f64().unwrap_or(0.0));

    let value_text = match &declaration.unit {
        Some(unit) => format!("{} {unit}", format_number(number, step)),
        None => format_number(number, step),
    };
    let is_editing = state.read(cx).edit_buffers.contains_key(&key);
    let edit_buffer = state.read(cx).edit_buffers.get(&key).cloned();
    let focus_handle = state.update(cx, |ui, cx| ui.focus_handle(&key, cx));

    let dec_plugin = plugin_id.to_string();
    let dec_key = key.clone();
    let on_dec = Box::new(
        move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
            write_number(cx, &dec_plugin, &dec_key, number - step, min, max);
        },
    );
    let inc_plugin = plugin_id.to_string();
    let inc_key = key.clone();
    let on_inc = Box::new(
        move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
            write_number(cx, &inc_plugin, &inc_key, number + step, min, max);
        },
    );
    let on_start_edit = start_edit(
        state.clone(),
        key.clone(),
        format_number(number, step),
        focus_handle.clone(),
    );

    let commit_plugin = plugin_id.to_string();
    let commit_key = key.clone();
    let on_key_down = editable_key_handler(state.clone(), key.clone(), true, move |cx, text| {
        let Ok(value) = text.trim().parse::<f64>() else {
            return;
        };
        write_number(cx, &commit_plugin, &commit_key, value, min, max);
    });

    render_number_field(
        c,
        d,
        NumberFieldProps {
            id_prefix: format!("{id_namespace}-number-{key}"),
            value_text,
            is_editing,
            edit_buffer,
            focus_handle,
            on_dec,
            on_inc,
            on_start_edit,
            on_key_down,
        },
    )
}

fn render_text_control(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    plugin_id: &str,
    declaration: &SettingDeclaration,
    current: &Value,
    c: &theme::ThemeColors,
    d: &theme::ThemeDimensions,
    cx: &mut App,
) -> AnyElement {
    let key = declaration.key.clone();
    let value = current.as_str().unwrap_or_default().to_string();
    let is_editing = state.read(cx).edit_buffers.contains_key(&key);
    let display = state
        .read(cx)
        .edit_buffers
        .get(&key)
        .cloned()
        .unwrap_or_else(|| value.clone());
    let focus_handle = state.update(cx, |ui, cx| ui.focus_handle(&key, cx));

    let commit_plugin = plugin_id.to_string();
    let commit_key = key.clone();
    let on_key_down = editable_key_handler(state.clone(), key.clone(), false, move |cx, text| {
        let _ =
            SettingsStore::set_plugin_value(cx, &commit_plugin, &commit_key, Value::String(text));
    });

    div()
        .id(ElementId::Name(format!("{id_namespace}-text-{key}").into()))
        .key_context("SettingsInput")
        .track_focus(&focus_handle)
        .relative()
        .overflow_hidden()
        .cursor_text()
        .w(px(160.0))
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(d.select_trigger_radius))
        .bg(if is_editing {
            c.dialog_surface
        } else {
            c.dialog_secondary_button_bg
        })
        .border_1()
        .border_color(c.dialog_border)
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .text_size(px(12.0))
                .text_color(if display.is_empty() {
                    c.dialog_muted
                } else {
                    c.text_default
                })
                .child(if display.is_empty() {
                    "Empty".to_string()
                } else {
                    display
                }),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(2.0))
                .rounded_b(px(d.select_trigger_radius))
                .bg(if is_editing {
                    c.focus_accent
                } else {
                    c.dialog_border
                }),
        )
        .on_click(start_edit(
            state.clone(),
            key.clone(),
            value.clone(),
            focus_handle.clone(),
        ))
        .on_key_down(on_key_down)
        .into_any_element()
}

/// Renders a dropdown picker over the given options; `on_select` runs the
/// kind-specific live apply (theme/language) before the value is persisted.
#[allow(clippy::too_many_arguments)]
fn render_picker(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    plugin_id: &str,
    declaration: &SettingDeclaration,
    current: &Value,
    options: Vec<PickerOption>,
    current_label: Option<String>,
    c: &theme::ThemeColors,
    d: &theme::ThemeDimensions,
    on_select: impl Fn(&mut App, String) + 'static,
    cx: &mut App,
) -> AnyElement {
    let key = declaration.key.clone();
    let current_value = current.as_str().unwrap_or_default();
    let is_open = state.read(cx).open_picker.as_deref() == Some(key.as_str());
    let label = current_label.unwrap_or_else(|| current_value.to_string());

    let toggle_state = state.clone();
    let toggle_key = key.clone();
    let trigger = select_trigger(format!("{id_namespace}-picker-{key}"), c, d)
        .text_size(px(12.0))
        .text_color(c.text_default)
        .child(div().flex_1().min_w(px(0.0)).truncate().child(label))
        .child(
            div().flex_shrink_0().pl(px(4.0)).child(
                svg()
                    .path("icons/settings/select-chevron.svg")
                    .size(px(16.0))
                    .text_color(c.dialog_muted),
            ),
        )
        .on_click(Box::new(
            move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                toggle_state.update(cx, |ui, _| {
                    ui.open_picker = if ui.open_picker.as_deref() == Some(toggle_key.as_str()) {
                        None
                    } else {
                        Some(toggle_key.clone())
                    };
                });
                cx.refresh_windows();
            },
        ));

    let mut wrap = div().relative().child(trigger);
    if is_open {
        let apply = Arc::new(on_select);
        let mut items = Vec::new();
        for option in &options {
            let is_selected = option.value == current_value;
            let option_value = option.value.clone();
            let option_label = option.label.clone();
            let close_state = state.clone();
            let close_key = key.clone();
            let select_plugin = plugin_id.to_string();
            let apply = apply.clone();
            items.push(
                select_option(
                    ElementId::Name(
                        format!("{id_namespace}-picker-{key}-opt-{option_value}").into(),
                    ),
                    c,
                    d,
                )
                .bg(c.dialog_surface)
                .text_size(px(12.0))
                .text_color(if is_selected {
                    c.dialog_primary_button_bg
                } else {
                    c.text_default
                })
                .child(option_label)
                .child(if is_selected {
                    svg()
                        .path("icons/settings/checkmark.svg")
                        .size(px(15.0))
                        .text_color(c.dialog_primary_button_bg)
                        .into_any_element()
                } else {
                    div().w(px(13.0)).into_any_element()
                })
                .on_click(Box::new(
                    move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        apply(cx, option_value.clone());
                        let _ = SettingsStore::set_plugin_value(
                            cx,
                            &select_plugin,
                            &close_key,
                            Value::String(option_value.clone()),
                        );
                        close_state.update(cx, |ui, _| ui.open_picker = None);
                    },
                ))
                .into_any_element(),
            );
        }
        wrap = wrap.child(gpui::deferred(select_panel(c, d).children(items)));
    }
    wrap.into_any_element()
}

/// Builds an inline-edit start handler seeding the buffer with `seed`.
fn start_edit(
    state: Entity<SettingsUiState>,
    key: String,
    seed: String,
    focus_handle: FocusHandle,
) -> SettingsClickHandler {
    Box::new(
        move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
            state.update(cx, |ui, _| {
                ui.edit_buffers
                    .entry(key.clone())
                    .or_insert_with(|| seed.clone());
            });
            window.focus(&focus_handle, cx);
            cx.refresh_windows();
        },
    )
}

/// Builds an inline-edit key handler: Escape cancels, Backspace trims,
/// Enter commits, printable characters extend. `numeric_only` restricts
/// input to digits, '.', and '-'.
fn editable_key_handler(
    state: Entity<SettingsUiState>,
    key: String,
    numeric_only: bool,
    commit: impl Fn(&mut App, String) + 'static,
) -> SettingsKeyHandler {
    let commit = Arc::new(commit);
    Box::new(
        move |event: &KeyDownEvent, _window: &mut Window, cx: &mut App| {
            match event.keystroke.key.as_str() {
                "escape" => {
                    state.update(cx, |ui, _| {
                        ui.edit_buffers.remove(&key);
                    });
                }
                "enter" => {
                    let buffer = state
                        .read(cx)
                        .edit_buffers
                        .get(&key)
                        .cloned()
                        .unwrap_or_default();
                    state.update(cx, |ui, _| {
                        ui.edit_buffers.remove(&key);
                    });
                    commit(cx, buffer);
                }
                "backspace" => {
                    state.update(cx, |ui, _| {
                        if let Some(buffer) = ui.edit_buffers.get_mut(&key) {
                            buffer.pop();
                        }
                    });
                }
                "space" => {
                    if !numeric_only {
                        state.update(cx, |ui, _| {
                            ui.edit_buffers.entry(key.clone()).or_default().push(' ');
                        });
                    }
                }
                _ => {
                    let text = event.keystroke.key_char.clone().unwrap_or_else(|| {
                        if event.keystroke.key.len() == 1 {
                            event.keystroke.key.clone()
                        } else {
                            String::new()
                        }
                    });
                    let accepted = if numeric_only {
                        text.chars()
                            .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-')
                    } else {
                        !text.chars().any(|ch| ch.is_control())
                    };
                    if !text.is_empty() && accepted {
                        state.update(cx, |ui, _| {
                            ui.edit_buffers
                                .entry(key.clone())
                                .or_default()
                                .push_str(&text);
                        });
                    }
                }
            }
            cx.refresh_windows();
        },
    )
}

/// Writes a numeric setting value, clamping to the declared bounds.
fn write_number(
    cx: &mut App,
    plugin: &str,
    key: &str,
    value: f64,
    min: Option<f64>,
    max: Option<f64>,
) {
    let clamped = match (min, max) {
        (Some(min), Some(max)) => value.clamp(min, max),
        (Some(min), None) => value.max(min),
        (None, Some(max)) => value.min(max),
        (None, None) => value,
    };
    let Some(number) = serde_json::Number::from_f64(clamped) else {
        return;
    };
    let _ = SettingsStore::set_plugin_value(cx, plugin, key, Value::Number(number));
}

/// Formats a number for display: integers stay integral, decimals keep at
/// most two significant places.
fn format_number(value: f64, step: f64) -> String {
    if value.fract() == 0.0 && step.fract() == 0.0 {
        return format!("{}", value as i64);
    }
    let mut text = format!("{value:.2}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// Capitalizes the first character of a dotted-key segment for card titles.
fn title_case(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Renders the theme color-override panel: a searchable list of every color
/// token with inline hex editing. Appended to the group that declares the
/// theme family picker (kind [`SettingKind::Theme`]).
fn render_theme_overrides_panel(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let overrides = PluginSettings::<CoreSettings>::get(cx)
        .theme
        .overrides
        .clone();

    // Every color token: (override key, display name, effective color).
    let mut tokens: Vec<(String, String, Hsla)> = ThemeColors::TOKEN_FIELD_NAMES
        .iter()
        .filter_map(|field| {
            let hsla = theme.colors.color_token(field)?;
            Some((format!("colors.{field}"), (*field).to_string(), hsla))
        })
        .collect();
    for (token_key, declaration) in cx.global::<ThemeManager>().registry().token_schema() {
        tokens.push((
            token_key.clone(),
            token_key.clone(),
            theme
                .extension
                .get(token_key)
                .copied()
                .unwrap_or(declaration.default),
        ));
    }

    let search_key = format!("{id_namespace}-theme-overrides");
    let query = state
        .read(cx)
        .search_queries
        .get(&search_key)
        .cloned()
        .unwrap_or_default()
        .to_lowercase();

    let search_focus = state.update(cx, |ui, cx| ui.focus_handle(&search_key, cx));
    let search_state = state.clone();
    let search_state_key = search_key.clone();
    let search_input = div()
        .id(ElementId::Name(format!("{search_key}-input").into()))
        .key_context("SettingsInput")
        .track_focus(&search_focus)
        .relative()
        .overflow_hidden()
        .cursor_text()
        .w_full()
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(d.select_trigger_radius))
        .bg(c.dialog_secondary_button_bg)
        .border_1()
        .border_color(c.dialog_border)
        .flex()
        .items_center()
        .child(
            div()
                .text_size(px(12.0))
                .text_color(if query.is_empty() {
                    c.dialog_muted
                } else {
                    c.text_default
                })
                .child(if query.is_empty() {
                    "Search color tokens…".to_string()
                } else {
                    query.clone()
                }),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(2.0))
                .rounded_b(px(d.select_trigger_radius))
                .bg(c.dialog_border),
        )
        .on_click(Box::new(
            move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                window.focus(&search_focus, cx);
            },
        ))
        .on_key_down(Box::new(
            move |event: &KeyDownEvent, _window: &mut Window, cx: &mut App| {
                search_state.update(cx, |ui, _| {
                    let query = ui
                        .search_queries
                        .entry(search_state_key.clone())
                        .or_default();
                    match event.keystroke.key.as_str() {
                        "escape" => query.clear(),
                        "backspace" => {
                            query.pop();
                        }
                        _ => {
                            let text = event.keystroke.key_char.clone().unwrap_or_else(|| {
                                if event.keystroke.key.len() == 1 {
                                    event.keystroke.key.clone()
                                } else {
                                    String::new()
                                }
                            });
                            if !text.is_empty() && !text.chars().any(|ch| ch.is_control()) {
                                query.push_str(&text);
                            }
                        }
                    }
                });
                cx.refresh_windows();
            },
        ));

    let mut rows: Vec<AnyElement> = Vec::new();
    for (token_key, display, effective) in tokens {
        if !query.is_empty() && !display.to_lowercase().contains(&query) {
            continue;
        }
        let overridden = overrides.contains_key(&token_key);
        let desc = format!(
            "{} · {}",
            if overridden {
                "Overridden"
            } else {
                "Inherited"
            },
            hsla_to_hex(effective),
        );
        let reset: Option<SettingsClickHandler> = if overridden {
            let reset_key = token_key.clone();
            Some(Box::new(
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    write_color_override(cx, &reset_key, None);
                },
            ))
        } else {
            None
        };
        let control = render_color_control(
            id_namespace,
            state,
            &token_key,
            effective,
            overridden,
            c,
            d,
            cx,
        );
        rows.push(make_row_with_reset(
            c.dialog_border,
            c,
            d,
            display,
            desc,
            reset,
            control,
        ));
    }

    section_card(c, d)
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(c.text_default)
                .child("Color Overrides"),
        )
        .child(search_input)
        .children(rows)
        .into_any_element()
}

/// Inline hex color input with a swatch, committing `#rrggbb[aa]` colors.
#[allow(clippy::too_many_arguments)]
fn render_color_control(
    id_namespace: &str,
    state: &Entity<SettingsUiState>,
    token_key: &str,
    effective: Hsla,
    overridden: bool,
    c: &ThemeColors,
    d: &ThemeDimensions,
    cx: &mut App,
) -> AnyElement {
    let edit_key = format!("override:{token_key}");
    let is_editing = state.read(cx).edit_buffers.contains_key(&edit_key);
    let display = state
        .read(cx)
        .edit_buffers
        .get(&edit_key)
        .cloned()
        .unwrap_or_else(|| hsla_to_hex(effective));
    let focus_handle = state.update(cx, |ui, cx| ui.focus_handle(&edit_key, cx));

    let commit_key = token_key.to_string();
    let on_key_down =
        editable_key_handler(state.clone(), edit_key.clone(), false, move |cx, text| {
            write_color_override(cx, &commit_key, Some(text));
        });

    div()
        .id(ElementId::Name(
            format!("{id_namespace}-color-{token_key}").into(),
        ))
        .key_context("SettingsInput")
        .track_focus(&focus_handle)
        .relative()
        .overflow_hidden()
        .cursor_text()
        .w(px(150.0))
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(d.select_trigger_radius))
        .bg(if is_editing {
            c.dialog_surface
        } else {
            c.dialog_secondary_button_bg
        })
        .border_1()
        .border_color(if overridden {
            c.focus_accent
        } else {
            c.dialog_border
        })
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .flex_shrink_0()
                .w(px(14.0))
                .h(px(14.0))
                .rounded_sm()
                .border_1()
                .border_color(c.dialog_border)
                .bg(effective),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .text_size(px(12.0))
                .text_color(if overridden {
                    c.text_default
                } else {
                    c.dialog_muted
                })
                .child(display),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(2.0))
                .rounded_b(px(d.select_trigger_radius))
                .bg(if is_editing {
                    c.focus_accent
                } else {
                    c.dialog_border
                }),
        )
        .on_click(start_edit(
            state.clone(),
            edit_key.clone(),
            hsla_to_hex(effective),
            focus_handle.clone(),
        ))
        .on_key_down(on_key_down)
        .into_any_element()
}

/// Writes (or, with `None`, removes) one theme color override through the
/// settings store; the theme sync hook applies it live. Invalid hex input
/// is ignored.
fn write_color_override(cx: &mut App, key: &str, hex: Option<String>) {
    let parsed = hex.map(|text| gpui::Rgba::try_from(text.trim()).map(gpui::Hsla::from));
    let _ = PluginSettings::<CoreSettings>::update(cx, |settings| match parsed {
        Some(Ok(hsla)) => {
            settings.theme.overrides.insert(key.to_string(), hsla);
        }
        Some(Err(_)) => {}
        None => {
            settings.theme.overrides.remove(key);
        }
    });
}

/// Formats an Hsla color as its `#rrggbbaa` hex string.
fn hsla_to_hex(hsla: Hsla) -> String {
    format!("#{:08x}", u32::from(gpui::Rgba::from(hsla)))
}
