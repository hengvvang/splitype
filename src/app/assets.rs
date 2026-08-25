//! Application asset loader for bundled SVG icons.

use std::borrow::Cow;

use gpui::*;

pub(crate) struct SplitypeAssets;

impl AssetSource for SplitypeAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match path {
            // ── Explorer: worktree (file tree content) ────────────────────
            "icons/explorer/worktree/folder.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/folder.svg"
            )))),
            "icons/explorer/worktree/open_folder.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/open_folder.svg"
            )))),
            "icons/explorer/worktree/file_type_pdf.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/file_type_pdf.svg"
            )))),
            "icons/explorer/worktree/file_type_code.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/explorer/worktree/file_type_code.svg"),
            ))),
            "icons/explorer/worktree/file_type_music.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/explorer/worktree/file_type_music.svg"),
            ))),
            "icons/explorer/worktree/file_type_image.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/explorer/worktree/file_type_image.svg"),
            ))),
            "icons/explorer/worktree/file_type_txt.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/file_type_txt.svg"
            )))),
            "icons/explorer/worktree/file_type_default.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/explorer/worktree/file_type_default.svg"),
            ))),
            "icons/explorer/worktree/markdown.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/markdown.svg"
            )))),
            "icons/explorer/worktree/chevron-down.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/chevron-down.svg"
            )))),
            "icons/explorer/worktree/chevron-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/chevron-right.svg"
            )))),
            "icons/explorer/worktree/view.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/view.svg"
            )))),
            "icons/explorer/worktree/hide.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/hide.svg"
            )))),
            "icons/explorer/worktree/sync_folder.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/sync_folder.svg"
            )))),
            "icons/explorer/worktree/replace_folder.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/explorer/worktree/replace_folder.svg"),
            ))),
            "icons/explorer/worktree/collapse-all.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/collapse-all.svg"
            )))),

            // ── Explorer: panel top bar (panel header) ──────────────
            "icons/explorer/topbar/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/check.svg"
            )))),
            "icons/explorer/topbar/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/split-h.svg"
            )))),
            "icons/explorer/topbar/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/split-v.svg"
            )))),
            "icons/explorer/topbar/close.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/close.svg"
            )))),
            "icons/explorer/topbar/maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/maximize.svg"
            )))),
            "icons/explorer/topbar/restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/restore.svg"
            )))),

            // ── Explorer: status bar ──────────────────────────────────────
            "icons/explorer/bottombar/new_folder.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/bottombar/new_folder.svg"
            )))),

            // ── Titlebar: app menu buttons ────────────────────────────────
            "icons/titlebar/app_menu/app_menu.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/app_menu/app_menu.svg"
            )))),
            "icons/titlebar/app_menu/sun.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/app_menu/sun.svg"
            )))),
            "icons/titlebar/app_menu/moon.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/app_menu/moon.svg"
            )))),
            "icons/titlebar/app_menu/checkmark.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/app_menu/checkmark.svg"
            )))),
            "icons/titlebar/app_menu/chevron-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/app_menu/chevron-right.svg"
            )))),

            // ── Titlebar: window controls ─────────────────────────────────
            "icons/titlebar/chrome/close.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/chrome/close.svg"
            )))),
            "icons/titlebar/chrome/mins.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/chrome/mins.svg"
            )))),
            "icons/titlebar/chrome/maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/chrome/maximize.svg"
            )))),
            "icons/titlebar/chrome/restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/titlebar/chrome/restore.svg"
            )))),

            // ── Settings ──────────────────────────────────────────────────
            "icons/settings/select-chevron.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/select-chevron.svg"
            )))),
            "icons/settings/checkmark.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/checkmark.svg"
            )))),
            "icons/settings/chevron-down.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/chevron-down.svg"
            )))),
            "icons/settings/chevron-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/chevron-right.svg"
            )))),
            "icons/settings/sun.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/sun.svg"
            )))),
            "icons/settings/plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/plus.svg"
            )))),
            "icons/settings/minus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/minus.svg"
            )))),
            "icons/settings/moon.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/moon.svg"
            )))),

            // ── Settings: panel top bar (panel header) ──────────────
            "icons/settings/topbar/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/check.svg"
            )))),
            "icons/settings/topbar/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/split-h.svg"
            )))),
            "icons/settings/topbar/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/split-v.svg"
            )))),
            "icons/settings/topbar/close.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/close.svg"
            )))),
            "icons/settings/topbar/maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/maximize.svg"
            )))),
            "icons/settings/topbar/restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/restore.svg"
            )))),

            // ── Editor: panel header ────────────────────────────────
            "icons/editor/topbar/active.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/active.svg"
            )))),
            "icons/editor/topbar/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/check.svg"
            )))),
            "icons/editor/topbar/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/split-h.svg"
            )))),
            "icons/editor/topbar/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/split-v.svg"
            )))),
            "icons/editor/topbar/close.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/close.svg"
            )))),
            "icons/editor/topbar/maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/maximize.svg"
            )))),
            "icons/editor/topbar/restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/restore.svg"
            )))),

            // ── Editor: panel status bar ──────────────────────────────────
            "icons/editor/bottombar/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/bottombar/split-h.svg"
            )))),
            "icons/editor/bottombar/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/bottombar/split-v.svg"
            )))),
            "icons/editor/bottombar/close.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/bottombar/close.svg"
            )))),
            "icons/editor/bottombar/checkmark.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/bottombar/checkmark.svg"
            )))),
            "icons/editor/bottombar/maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/bottombar/maximize.svg"
            )))),
            "icons/editor/bottombar/restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/bottombar/restore.svg"
            )))),

            // ── Editor: context menu ──────────────────────────────────────
            "icons/editor/context_menu/chevron-right.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/context_menu/chevron-right.svg"),
            ))),
            "icons/editor/context_menu/plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/context_menu/plus.svg"
            )))),
            "icons/editor/context_menu/minus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/context_menu/minus.svg"
            )))),

            // ── Editor: WYSIWYG panel ─────────────────────────────────────
            "icons/editor/wysiwyg/checkbox.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/checkbox.svg"
            )))),
            "icons/editor/wysiwyg/checkbox-checked.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/checkbox-checked.svg"
            )))),
            "icons/editor/wysiwyg/table/plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/table/plus.svg"
            )))),
            "icons/editor/wysiwyg/codeblock/line-numbers.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/codeblock/line-numbers.svg"),
            ))),
            "icons/editor/wysiwyg/codeblock/copy.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/codeblock/copy.svg"
            )))),
            "icons/editor/wysiwyg/codeblock/select-checkmark.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/codeblock/select-checkmark.svg"),
            ))),
            "icons/editor/wysiwyg/codeblock/select-chevron.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/codeblock/select-chevron.svg"),
            ))),

            // ── Editor: preview panel ─────────────────────────────────────
            "icons/editor/preview/checkbox.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/preview/checkbox.svg"
            )))),
            "icons/editor/preview/checkbox-checked.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/preview/checkbox-checked.svg"
            )))),

            // ── Editor: outline panel ─────────────────────────────────────
            "icons/editor/outline/markdown.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/outline/markdown.svg"
            )))),

            // ── Splitter: gesture overlays (Join arrows and Swap) ─────────
            "icons/splitter/arrow-up.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/splitter/arrow-up.svg"
            )))),
            "icons/splitter/arrow-down.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/splitter/arrow-down.svg"
            )))),
            "icons/splitter/arrow-left.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/splitter/arrow-left.svg"
            )))),
            "icons/splitter/arrow-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/splitter/arrow-right.svg"
            )))),
            "icons/splitter/swap.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/splitter/swap.svg"
            )))),

            // ── Identity ──────────────────────────────────────────────────
            "identity/logo.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/identity/logo.svg"
            )))),
            "identity/logo.png" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/identity/logo.png"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
