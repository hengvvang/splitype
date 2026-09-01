//! Shortcuts overview section component — renders the command registry's
//! manifest-declared shortcuts grouped by plugin.

use gpui::*;

use crate::ui_helpers::{SettingsClickHandler, make_row, make_section};
use theme::{ThemeColors, ThemeDimensions};

/// One displayed shortcut row, resolved from the command registry.
pub struct ShortcutItem {
    pub name: String,
    pub context: String,
    pub shortcut: String,
}

/// Groups the registry's commands with declared shortcuts by plugin id,
/// preserving declaration order.
pub fn shortcut_sections() -> Vec<(String, Vec<ShortcutItem>)> {
    let contributions = core_contracts::CommandRegistry::registered_commands().unwrap_or_default();
    let mut sections: Vec<(String, Vec<ShortcutItem>)> = Vec::new();
    for contribution in contributions {
        if contribution.shortcuts.is_empty() {
            continue;
        }
        let (plugin, local) = contribution
            .id
            .as_str()
            .rsplit_once('.')
            .unwrap_or((contribution.id.as_str(), contribution.id.as_str()));
        let item = ShortcutItem {
            name: title_case(local.replace('-', " ").as_str()),
            context: contribution
                .context
                .as_deref()
                .unwrap_or_default()
                .to_string(),
            shortcut: contribution
                .shortcuts
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<_>>()
                .join(" / "),
        };
        match sections.last_mut() {
            Some((section_plugin, items)) if section_plugin == plugin => items.push(item),
            _ => sections.push((plugin.to_string(), vec![item])),
        }
    }
    sections
}

fn title_case(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub(crate) fn render_shortcuts_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    title: &str,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    items: &[ShortcutItem],
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();
    if expanded {
        for item in items {
            let ctrl_sc = div()
                .px(px(8.0))
                .py(px(2.0))
                .rounded(px(d.badge_radius))
                .bg(c.dialog_secondary_button_hover)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(c.text_default)
                .child(item.shortcut.clone())
                .into_any_element();

            rows.push(make_row(
                inner_border_color,
                c,
                d,
                item.name.as_str(),
                item.context.as_str(),
                ctrl_sc,
            ));
        }
    }

    make_section(c, d, id, title, expanded, toggle_fn, rows)
}
