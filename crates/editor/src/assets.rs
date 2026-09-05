//! Embedded SVG icon asset catalog for the Editor plugin.

use std::borrow::Cow;
use platform_contracts::PluginAssetProvider;

pub struct EditorAssets;

impl PluginAssetProvider for EditorAssets {
    fn load_asset(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        match_icon(path)
    }
}

/// Resolves an icon asset for the editor panel and its editor panes.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = path
        .strip_prefix("plugin://splitype.editor/")
        .unwrap_or(path);

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
        "topbar/plus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/topbar/plus.svg"
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

        // ── Editor: search panel ──────────────────────────────────────
        "search/chevron-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/search/chevron-down.svg"
        ))),
        "search/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/search/chevron-right.svg"
        ))),
        "search/replace-all.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/search/replace-all.svg"
        ))),

        // ── Editor: outline panel ─────────────────────────────────────
        "outline/markdown.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/outline/markdown.svg"
        ))),

        _ => None,
    }
}
