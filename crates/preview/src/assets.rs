//! Embedded SVG icon asset catalog for the Preview plugin.

use std::borrow::Cow;

/// Resolves an icon asset for the Preview pane.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = if let Some(stripped) = path.strip_prefix("icons/preview/") {
        stripped
    } else if let Some(stripped) = path.strip_prefix("icons/editor/preview/") {
        stripped
    } else if path.starts_with("icons/") {
        return None;
    } else {
        path
    };

    match subpath {
        "checkbox.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkbox.svg"
        ))),
        "checkbox-checked.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkbox-checked.svg"
        ))),
        _ => None,
    }
}
