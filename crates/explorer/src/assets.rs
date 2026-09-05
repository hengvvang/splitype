//! Embedded SVG icon asset catalog for the Explorer plugin.

use std::borrow::Cow;
use platform_contracts::PluginAssetProvider;

pub struct ExplorerAssets;

impl PluginAssetProvider for ExplorerAssets {
    fn load_asset(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        match_icon(path)
    }
}

/// Resolves an icon asset for the explorer panel.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = path
        .strip_prefix("plugin://splitype.explorer/")
        .unwrap_or(path);

    match subpath {
        // ── Explorer: worktree (file tree content) ────────────────────
        "worktree/folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/folder.svg"
        ))),
        "worktree/open_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/open_folder.svg"
        ))),
        "worktree/file_type_pdf.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/file_type_pdf.svg"
        ))),
        "worktree/file_type_code.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/file_type_code.svg"
        ))),
        "worktree/file_type_music.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/file_type_music.svg"
        ))),
        "worktree/file_type_image.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/file_type_image.svg"
        ))),
        "worktree/file_type_txt.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/file_type_txt.svg"
        ))),
        "worktree/file_type_default.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/file_type_default.svg"
        ))),
        "worktree/markdown.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/markdown.svg"
        ))),
        "worktree/chevron-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/chevron-down.svg"
        ))),
        "worktree/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/chevron-right.svg"
        ))),
        "worktree/view.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/view.svg"
        ))),
        "worktree/hide.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/hide.svg"
        ))),
        "worktree/sync_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/sync_folder.svg"
        ))),
        "worktree/replace_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/replace_folder.svg"
        ))),
        "worktree/collapse-all.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/worktree/collapse-all.svg"
        ))),

        // ── Explorer: panel top bar (panel header) ──────────────
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

        // ── Explorer: status bar ──────────────────────────────────────
        "bottombar/new_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/new_folder.svg"
        ))),
        "bottombar/v_three_points.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/bottombar/v_three_points.svg"
        ))),

        _ => None,
    }
}
