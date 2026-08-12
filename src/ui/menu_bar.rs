//! In-window menu bar geometry — pure layout math shared by the Shell's
//! titlebar menu bar renderer (`crate::app::window_chrome`).
//!
//! These functions compute menu-bar button widths, panel positioning,
//! submenu bridging, and scrollable-menu heights from theme dimensions.
//! No editor or model imports: reusable UI components must stay below the
//! application layers (see `crate::ui` module docs).

use gpui::OwnedMenuItem;

use crate::infra::theme::ThemeDimensions;

// ── Character width estimation ────────────────────────────────────────────

fn is_wide_menu_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
    )
}

fn estimated_menu_label_width(label: &str, text_size: f32) -> f32 {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_whitespace() {
                text_size * 0.35
            } else if ch.is_ascii_punctuation() {
                text_size * 0.45
            } else if ch.is_ascii() {
                text_size * 0.54
            } else if is_wide_menu_char(ch) {
                text_size
            } else {
                text_size * 0.85
            }
        })
        .sum()
}

// ── Menu bar button geometry ──────────────────────────────────────────────

pub const TITLEBAR_MENU_BUTTON_PADDING_X: f32 = 5.0;
pub const TITLEBAR_MENU_BUTTON_GAP: f32 = 2.0;
pub const TITLEBAR_MENU_START_X: f32 = 32.0;

pub fn menu_bar_button_width(label: &str, dimensions: &ThemeDimensions) -> f32 {
    let content_width = estimated_menu_label_width(label, dimensions.menu_text_size)
        + TITLEBAR_MENU_BUTTON_PADDING_X * 2.0;
    content_width.ceil().max(20.0)
}

// ── Platform guards ───────────────────────────────────────────────────────

pub fn supports_in_window_menu_for_target_os(target_os: &str) -> bool {
    target_os != "macos"
}

pub fn supports_in_window_menu() -> bool {
    supports_in_window_menu_for_target_os(std::env::consts::OS)
}

pub fn in_window_menu_bar_height_for_target_os(
    _target_os: &str,
    _has_menus: bool,
    _dimensions: &ThemeDimensions,
) -> f32 {
    0.0
}

// ── Panel positioning ─────────────────────────────────────────────────────

pub fn menu_panel_left<S: AsRef<str>>(
    open_index: usize,
    menu_labels: &[S],
    dimensions: &ThemeDimensions,
) -> f32 {
    let prior_width: f32 = menu_labels
        .iter()
        .take(open_index)
        .map(|label| menu_bar_button_width(label.as_ref(), dimensions))
        .sum();
    TITLEBAR_MENU_START_X + prior_width + TITLEBAR_MENU_BUTTON_GAP * open_index as f32
}

pub fn menu_panel_width_for_labels<S: AsRef<str>>(
    labels: &[S],
    dimensions: &ThemeDimensions,
) -> f32 {
    let widest_label = labels
        .iter()
        .map(|label| estimated_menu_label_width(label.as_ref(), dimensions.menu_text_size))
        .fold(0.0, f32::max);
    let content_width = widest_label + dimensions.menu_item_padding_x * 2.0;
    dimensions.menu_panel_width.max(content_width.ceil())
}

// ── Item labelling ────────────────────────────────────────────────────────

pub fn owned_menu_item_labels(items: &[OwnedMenuItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            OwnedMenuItem::Action { name, .. } => Some(name.to_string()),
            OwnedMenuItem::Submenu(menu) => Some(menu.name.to_string()),
            OwnedMenuItem::SystemMenu(menu) => Some(menu.name.to_string()),
            OwnedMenuItem::Separator => None,
        })
        .collect()
}

// ── Visual height helpers ─────────────────────────────────────────────────

pub fn menu_item_visual_height(item: &OwnedMenuItem, dimensions: &ThemeDimensions) -> f32 {
    match item {
        OwnedMenuItem::Separator => {
            dimensions.menu_separator_height + dimensions.menu_separator_margin_y * 2.0
        }
        OwnedMenuItem::Action { .. } | OwnedMenuItem::Submenu(_) | OwnedMenuItem::SystemMenu(_) => {
            dimensions.menu_item_height
        }
    }
}

pub const SCROLLABLE_IMPORT_MENU_VISIBLE_ITEMS: usize = 12;

pub fn menu_items_visual_height_with_gaps(
    items: &[OwnedMenuItem],
    dimensions: &ThemeDimensions,
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }

    let items_height: f32 = items
        .iter()
        .map(|item| menu_item_visual_height(item, dimensions))
        .sum();
    items_height + dimensions.menu_panel_gap * items.len().saturating_sub(1) as f32
}

pub fn scrollable_import_menu_scroll_height(
    scroll_items: &[OwnedMenuItem],
    footer_items: &[OwnedMenuItem],
    viewport_height: f32,
    top_offset: f32,
    dimensions: &ThemeDimensions,
) -> f32 {
    let visible_count = scroll_items.len().min(SCROLLABLE_IMPORT_MENU_VISIBLE_ITEMS);
    if visible_count == 0 {
        return 0.0;
    }

    let default_height =
        menu_items_visual_height_with_gaps(&scroll_items[..visible_count], dimensions);
    let footer_height = menu_items_visual_height_with_gaps(footer_items, dimensions);
    let footer_gap = if footer_items.is_empty() {
        0.0
    } else {
        dimensions.menu_panel_gap
    };
    let available_height = viewport_height
        - top_offset
        - dimensions.menu_panel_top
        - dimensions.menu_panel_padding * 2.0
        - footer_height
        - footer_gap
        - 8.0;
    let min_height = dimensions.menu_item_height.min(default_height).max(1.0);

    default_height.min(available_height.max(min_height))
}

// ── Sub-menu bridging geometry ────────────────────────────────────────────

pub fn submenu_panel_top(
    items: &[OwnedMenuItem],
    item_index: usize,
    dimensions: &ThemeDimensions,
) -> f32 {
    let prior_items_height: f32 = items
        .iter()
        .take(item_index)
        .map(|item| menu_item_visual_height(item, dimensions))
        .sum();
    let prior_gaps = dimensions.menu_panel_gap * item_index as f32;
    dimensions.menu_panel_top + dimensions.menu_panel_padding + prior_items_height + prior_gaps
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuSubmenuBridgeGeometry {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

pub fn submenu_bridge_geometry<S: AsRef<str>, T: AsRef<str>>(
    open_index: usize,
    menu_labels: &[S],
    items: &[OwnedMenuItem],
    item_index: usize,
    submenu_labels: &[T],
    dimensions: &ThemeDimensions,
) -> Option<MenuSubmenuBridgeGeometry> {
    let item = items.get(item_index)?;
    let main_panel_left = menu_panel_left(open_index, menu_labels, dimensions);
    let main_panel_width = menu_panel_width_for_labels(&owned_menu_item_labels(items), dimensions);
    let submenu_width = menu_panel_width_for_labels(submenu_labels, dimensions);
    let vertical_tolerance = dimensions.menu_panel_padding + dimensions.menu_panel_gap;
    let item_top = submenu_panel_top(items, item_index, dimensions);
    let top = (item_top - vertical_tolerance).max(dimensions.menu_panel_top);
    Some(MenuSubmenuBridgeGeometry {
        left: main_panel_left + main_panel_width,
        top,
        width: dimensions.menu_panel_gap + submenu_width,
        height: menu_item_visual_height(item, dimensions) + vertical_tolerance * 2.0,
    })
}

// ── Shared chrome ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::theme::Theme;
    use gpui::{OwnedMenu, OwnedMenuItem};

    gpui::actions!(test_ui, [DummyMenuAction]);

    fn disabled_menu_action(name: &str) -> OwnedMenuItem {
        OwnedMenuItem::Action {
            name: name.into(),
            action: Box::new(DummyMenuAction),
            os_action: None,
        }
    }

    #[test]
    fn menu_button_width_expands_for_long_ascii_labels() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;

        // Width follows the label text, so a long ASCII label grows the
        // button well beyond a short CJK one.
        assert!(
            menu_bar_button_width("Language", dimensions)
                > menu_bar_button_width("文件", dimensions)
        );
        assert!(menu_bar_button_width("Language", dimensions) > dimensions.menu_bar_button_width);
    }

    #[test]
    fn in_window_menu_is_enabled_for_every_target_except_macos() {
        for target_os in [
            "windows",
            "linux",
            "freebsd",
            "openbsd",
            "netbsd",
            "dragonfly",
            "solaris",
            "illumos",
            "android",
            "unknown",
        ] {
            assert!(
                supports_in_window_menu_for_target_os(target_os),
                "{target_os} should use the in-window fallback menu"
            );
        }
        assert!(!supports_in_window_menu_for_target_os("macos"));
    }

    #[test]
    fn menu_panel_left_uses_accumulated_dynamic_button_widths() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let labels = vec![
            "File".to_string(),
            "Language".to_string(),
            "Theme".to_string(),
            "Help".to_string(),
        ];

        let left = menu_panel_left(2, &labels, dimensions);
        let expected = TITLEBAR_MENU_START_X
            + menu_bar_button_width("File", dimensions)
            + TITLEBAR_MENU_BUTTON_GAP
            + menu_bar_button_width("Language", dimensions)
            + TITLEBAR_MENU_BUTTON_GAP;
        let old_fixed_left = dimensions.menu_bar_padding_x
            + 2.0 * (dimensions.menu_bar_button_width + dimensions.menu_bar_gap);

        assert_eq!(left, expected);
        assert!(left > old_fixed_left);
    }

    #[test]
    fn menu_panel_width_expands_for_long_recent_paths() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let short_labels = vec!["Save".to_string()];
        let long_labels = vec![r"C:\Users\someone\Documents\Very Long Folder\notes.md".to_string()];

        assert_eq!(
            menu_panel_width_for_labels(&short_labels, dimensions),
            dimensions.menu_panel_width
        );
        assert!(
            menu_panel_width_for_labels(&long_labels, dimensions) > dimensions.menu_panel_width
        );
    }

    #[test]
    fn scrollable_import_menu_height_caps_visible_items_and_clamps_to_viewport() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let scroll_items = (0..20)
            .map(|index| disabled_menu_action(&format!("Custom Theme {index}")))
            .collect::<Vec<_>>();
        let footer_items = vec![OwnedMenuItem::Separator];
        let expected_large_height =
            menu_items_visual_height_with_gaps(&scroll_items[..12], dimensions);
        let full_scroll_content_height =
            menu_items_visual_height_with_gaps(&scroll_items, dimensions);
        let footer_height = menu_items_visual_height_with_gaps(&footer_items, dimensions);

        let large_height = scrollable_import_menu_scroll_height(
            &scroll_items,
            &footer_items,
            2000.0,
            0.0,
            dimensions,
        );
        let small_height = scrollable_import_menu_scroll_height(
            &scroll_items,
            &footer_items,
            180.0,
            0.0,
            dimensions,
        );

        assert!((large_height - expected_large_height).abs() < f32::EPSILON);
        assert!(full_scroll_content_height > large_height);
        assert!(large_height < expected_large_height + footer_height);
        assert!(small_height < large_height);
        assert!(small_height >= dimensions.menu_item_height);
    }

    #[test]
    fn submenu_bridge_spans_parent_child_menu_gap() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let labels = vec!["File".to_string()];
        let items = vec![
            OwnedMenuItem::Separator,
            OwnedMenuItem::Submenu(OwnedMenu {
                name: "Recent".into(),
                items: vec![OwnedMenuItem::Action {
                    name: r"C:\Users\someone\Documents\notes.md".into(),
                    action: Box::new(DummyMenuAction),
                    os_action: None,
                }],
            }),
        ];
        let submenu_labels = match &items[1] {
            OwnedMenuItem::Submenu(submenu) => owned_menu_item_labels(&submenu.items),
            _ => Vec::new(),
        };

        let bridge = submenu_bridge_geometry(0, &labels, &items, 1, &submenu_labels, dimensions)
            .expect("submenu bridge geometry should be available");
        let main_width = menu_panel_width_for_labels(&owned_menu_item_labels(&items), dimensions);
        let submenu_width = menu_panel_width_for_labels(&submenu_labels, dimensions);

        assert_eq!(
            bridge.left,
            menu_panel_left(0, &labels, dimensions) + main_width
        );
        assert_eq!(bridge.width, dimensions.menu_panel_gap + submenu_width);
        assert!(bridge.height > dimensions.menu_item_height);
        let item_top = dimensions.menu_panel_top
            + dimensions.menu_panel_padding
            + dimensions.menu_separator_height
            + dimensions.menu_separator_margin_y * 2.0
            + dimensions.menu_panel_gap;
        assert!(bridge.top < item_top);
        assert!(bridge.top >= dimensions.menu_panel_top);
    }

    #[test]
    fn submenu_bridge_uses_dynamic_main_menu_width() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let labels = vec!["File".to_string()];
        let items = vec![OwnedMenuItem::Submenu(OwnedMenu {
            name: "Open Recently Used Markdown File".into(),
            items: vec![OwnedMenuItem::Action {
                name: r"C:\Users\someone\Documents\Very Long Folder\notes.md".into(),
                action: Box::new(DummyMenuAction),
                os_action: None,
            }],
        })];
        let submenu_labels = match &items[0] {
            OwnedMenuItem::Submenu(submenu) => owned_menu_item_labels(&submenu.items),
            _ => Vec::new(),
        };

        let bridge = submenu_bridge_geometry(0, &labels, &items, 0, &submenu_labels, dimensions)
            .expect("submenu bridge geometry should be available");

        assert!(bridge.left > dimensions.menu_bar_padding_x + dimensions.menu_panel_width);
        assert!(bridge.width > dimensions.menu_panel_gap + dimensions.menu_panel_width);
    }
}
