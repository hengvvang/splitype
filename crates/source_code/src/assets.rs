//! Embedded SVG icon asset catalog for the Source Code editor plugin.

use std::borrow::Cow;

/// Resolves an icon asset for the Source Code pane.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = if let Some(stripped) = path.strip_prefix("icons/source_code/") {
        stripped
    } else if path.starts_with("icons/") {
        return None;
    } else {
        path
    };

    match subpath {
        "chevron-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/chevron-down.svg"
        ))),
        "chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/chevron-right.svg"
        ))),
        "checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkmark.svg"
        ))),
        _ => None,
    }
}
