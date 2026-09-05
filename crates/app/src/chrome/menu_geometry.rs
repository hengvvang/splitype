//! In-window menu bar geometry — layout math for titlebar menu buttons and dropdowns.

use gpui::OwnedMenuItem;
use theme::ThemeDimensions;

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

pub const MENU_PANEL_MAX_WIDTH: f32 = 560.0;

pub fn menu_panel_width_for_labels<S: AsRef<str>>(
    labels: &[S],
    dimensions: &ThemeDimensions,
) -> f32 {
    let widest_label = labels
        .iter()
        .map(|label| estimated_menu_label_width(label.as_ref(), dimensions.menu_text_size))
        .fold(0.0, f32::max);
    let content_width = widest_label + dimensions.menu_item_padding_x * 2.0;
    dimensions
        .menu_panel_width
        .max(content_width.ceil())
        .min(MENU_PANEL_MAX_WIDTH)
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
