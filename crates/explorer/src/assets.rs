//! Embedded SVG icon asset catalog for the Explorer plugin.

use std::borrow::Cow;

/// Resolves an icon asset for the explorer panel.
///
/// Accepts paths with or without the `"icons/explorer/"` prefix.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = if let Some(stripped) = path.strip_prefix("icons/explorer/") {
        stripped
    } else if path.starts_with("icons/") {
        return None;
    } else {
        path
    };

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
