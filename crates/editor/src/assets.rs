//! Embedded SVG icon asset catalog for the Editor plugin.

use std::borrow::Cow;

/// Resolves an icon asset for the editor panel and its editor panes.
///
/// Accepts paths with or without the `"icons/editor/"` prefix.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = if let Some(stripped) = path.strip_prefix("icons/editor/") {
        stripped
    } else if path.starts_with("icons/") {
        return None;
    } else {
        path
    };

    match subpath {
        // ── Editor: panel header ────────────────────────────────
        "panel.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/panel.svg"
        ))),
        "topbar/active.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/active.svg"
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
        "topbar/search.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/search.svg"
        ))),
        "topbar/replace.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/replace.svg"
        ))),
        "topbar/prev.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/prev.svg"
        ))),
        "topbar/next.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/next.svg"
        ))),
        "topbar/search-explorer.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/search-explorer.svg"
        ))),
        "topbar/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/maximize.svg"
        ))),
        "topbar/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/restore.svg"
        ))),

        // ── Editor: panel status bar ──────────────────────────────────
        "bottombar/split-h.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/split-h.svg"
        ))),
        "bottombar/split-v.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/split-v.svg"
        ))),
        "bottombar/close.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/close.svg"
        ))),
        "bottombar/checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/checkmark.svg"
        ))),
        "bottombar/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/maximize.svg"
        ))),
        "bottombar/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/restore.svg"
        ))),

        // ── Editor: context menu ──────────────────────────────────────
        "context_menu/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/context_menu/chevron-right.svg"
        ))),
        "context_menu/plus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/context_menu/plus.svg"
        ))),
        "context_menu/minus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/context_menu/minus.svg"
        ))),
        // ── Editor: outline panel ─────────────────────────────────────
        "outline/markdown.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/outline/markdown.svg"
        ))),

        _ => None,
    }
}
