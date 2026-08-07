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
            "icons/explorer/worktree/folder-open.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/folder-open.svg"
            )))),
            "icons/explorer/worktree/file.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/file.svg"
            )))),
            "icons/explorer/worktree/file-plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/file-plus.svg"
            )))),
            "icons/explorer/worktree/markdown.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/markdown.svg"
            )))),
            "icons/explorer/worktree/chevron-down.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/chevron-down.svg"
            )))),
            "icons/explorer/worktree/chevron-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/chevron-right.svg"
            )))),
            "icons/explorer/worktree/eye.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/eye.svg"
            )))),
            "icons/explorer/worktree/eye-off.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/eye-off.svg"
            )))),
            "icons/explorer/worktree/refresh.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/refresh.svg"
            )))),
            "icons/explorer/worktree/collapse-all.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/worktree/collapse-all.svg"
            )))),

            // ── Explorer: title bar (window area header) ──────────────────
            "icons/explorer/topbar/link.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/link.svg"
            )))),
            "icons/explorer/topbar/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/split-h.svg"
            )))),
            "icons/explorer/topbar/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/split-v.svg"
            )))),
            "icons/explorer/topbar/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/topbar/check.svg"
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
            "icons/explorer/bottombar/folder-plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/explorer/bottombar/folder-plus.svg"
            )))),

            // ── Titlebar: app menu buttons ────────────────────────────────
            "icons/topbar/app_menu/sun.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/app_menu/sun.svg"
            )))),
            "icons/topbar/app_menu/moon.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/app_menu/moon.svg"
            )))),
            "icons/topbar/app_menu/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/app_menu/check.svg"
            )))),
            "icons/topbar/app_menu/chevron-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/app_menu/chevron-right.svg"
            )))),

            // ── Titlebar: window controls ─────────────────────────────────
            "icons/topbar/chrome/close.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/chrome/close.svg"
            )))),
            "icons/topbar/chrome/minimize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/chrome/minimize.svg"
            )))),
            "icons/topbar/chrome/maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/chrome/maximize.svg"
            )))),
            "icons/topbar/chrome/restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/topbar/chrome/restore.svg"
            )))),

            // ── Settings ──────────────────────────────────────────────────
            "icons/settings/select-chevron.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/select-chevron.svg"
            )))),
            "icons/settings/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/check.svg"
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
            "icons/settings/moon.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/moon.svg"
            )))),

            // ── Settings: title bar (window area header) ──────────────────
            "icons/settings/topbar/link.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/link.svg"
            )))),
            "icons/settings/topbar/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/split-h.svg"
            )))),
            "icons/settings/topbar/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/split-v.svg"
            )))),
            "icons/settings/topbar/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/settings/topbar/check.svg"
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

            // ── Editor: window area header ────────────────────────────────
            "icons/editor/topbar/link.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/link.svg"
            )))),
            "icons/editor/topbar/split-h.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/split-h.svg"
            )))),
            "icons/editor/topbar/split-v.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/split-v.svg"
            )))),
            "icons/editor/topbar/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/topbar/check.svg"
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
            "icons/editor/bottombar/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/bottombar/check.svg"
            )))),

            // ── Editor: WYSIWYG panel ─────────────────────────────────────
            "icons/editor/wysiwyg/task-check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/task-check.svg"
            )))),
            "icons/editor/wysiwyg/table/plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/table/plus.svg"
            )))),
            "icons/editor/wysiwyg/table/handle-row.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/table/handle-row.svg"
            )))),
            "icons/editor/wysiwyg/table/handle-row-hollow.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/table/handle-row-hollow.svg")
            ))),
            "icons/editor/wysiwyg/table/handle-row-solid.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/table/handle-row-solid.svg")
            ))),
            "icons/editor/wysiwyg/table/handle-column.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/table/handle-column.svg"
            )))),
            "icons/editor/wysiwyg/callout/note.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/callout/note.svg"
            )))),
            "icons/editor/wysiwyg/callout/tip.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/callout/tip.svg"
            )))),
            "icons/editor/wysiwyg/callout/important.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/callout/important.svg"
            )))),
            "icons/editor/wysiwyg/callout/warning.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/callout/warning.svg"
            )))),
            "icons/editor/wysiwyg/callout/caution.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/wysiwyg/callout/caution.svg"
            )))),
            "icons/editor/wysiwyg/codeblock/line-numbers.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/codeblock/line-numbers.svg")
            ))),
            "icons/editor/wysiwyg/codeblock/select-check.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/codeblock/select-check.svg")
            ))),
            "icons/editor/wysiwyg/codeblock/select-chevron.svg" => Ok(Some(Cow::Borrowed(
                include_bytes!("../../assets/icons/editor/wysiwyg/codeblock/select-chevron.svg")
            ))),

            // ── Editor: preview panel ─────────────────────────────────────
            "icons/editor/preview/task-check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/preview/task-check.svg"
            )))),

            // ── Editor: outline panel ─────────────────────────────────────
            "icons/editor/outline/markdown.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/editor/outline/markdown.svg"
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
