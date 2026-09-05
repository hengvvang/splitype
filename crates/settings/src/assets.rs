//! Embedded SVG icon asset catalog for the Settings plugin.

use std::borrow::Cow;

/// Resolves an icon asset for the settings panel.
///
/// Accepts paths with or without the `"icons/settings/"` prefix.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = if let Some(stripped) = path.strip_prefix("icons/settings/") {
        stripped
    } else if path.starts_with("icons/") {
        return None;
    } else {
        path
    };

    match subpath {
        "select-chevron.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/select-chevron.svg"
        ))),
        "checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkmark.svg"
        ))),
        "chevron-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/chevron-down.svg"
        ))),
        "chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/chevron-right.svg"
        ))),
        "chevron-up-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/chevron-up-down.svg"
        ))),
        "sun.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/sun.svg"
        ))),
        "plus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/plus.svg"
        ))),
        "minus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/minus.svg"
        ))),
        "moon.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/moon.svg"
        ))),
        "undo.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/undo.svg"
        ))),

        // ── Settings: panel top bar (panel header) ──────────────
        "panel.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/panel.svg"
        ))),
        "topbar/check.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/check.svg"
        ))),
        "topbar/split-h.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/split-h.svg"
        ))),
        "topbar/split-v.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/split-v.svg"
        ))),
        "topbar/close.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/close.svg"
        ))),
        "topbar/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/maximize.svg"
        ))),
        "topbar/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/restore.svg"
        ))),

        _ => None,
    }
}
