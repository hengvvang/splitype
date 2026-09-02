//! Window-chrome presentation helpers shared by the shell and panel plugins.

use gpui::SharedString;

/// Icon path for a window-panel top-bar button.
///
/// `icon_prefix` is a plugin-owned asset directory (e.g. `icons/editor`);
/// the panel kind is deliberately NOT part of the path so kinds stay pure
/// identifiers and third-party plugins control their own resources.
pub fn panel_topbar_icon(icon_prefix: &str, name: &str) -> SharedString {
    format!("{icon_prefix}/topbar/{name}.svg").into()
}
