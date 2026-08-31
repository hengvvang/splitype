//! Window-chrome presentation helpers shared by the shell and the panels.

use gpui::SharedString;
use theme::Theme;

use crate::plugin::PanelKindId;

/// Icon path for a window-panel top-bar button, per panel kind.
///
/// Every panel plugin owns its own copies of the top-bar icons, so a button's
/// asset path dynamically derives from the kind identifier of the area it renders in.
pub fn panel_topbar_icon(kind: PanelKindId, name: &str) -> SharedString {
    format!("icons/{}/topbar/{name}.svg", kind.0).into()
}

/// Map a theme to the splitter border-menu style parameters.
///
/// Shared by the outer window-panel border menu and the editor pane
/// border menu so both render identically.
pub fn border_menu_style(theme: &Theme) -> splitter::interaction::MenuStyle {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    splitter::interaction::MenuStyle {
        surface: c.dialog_surface,
        border: c.dialog_border,
        border_width: d.dialog_border_width,
        radius: d.menu_panel_radius,
        width: d.menu_panel_width,
        padding: d.menu_panel_padding,
        gap: d.menu_panel_gap,
        text: c.dialog_secondary_button_text,
        text_size: d.menu_text_size,
        text_weight: t.dialog_body_weight.to_font_weight(),
        item_height: d.menu_item_height,
        item_padding_x: d.menu_item_padding_x,
        item_radius: d.menu_item_radius,
        item_hover: c.panel_row_hover,
        separator_margin_x: d.menu_separator_margin_x,
        separator_margin_y: d.menu_separator_margin_y,
        separator_height: d.menu_separator_height,
    }
}
