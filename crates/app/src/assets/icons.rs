//! Embedded SVG icon asset catalog for Splitype.

use std::borrow::Cow;

pub(super) fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    match path {
        // ── Explorer: worktree (file tree content) ────────────────────
        "icons/explorer/worktree/folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/folder.svg"
        ))),
        "icons/explorer/worktree/open_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/open_folder.svg"
        ))),
        "icons/explorer/worktree/file_type_pdf.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/file_type_pdf.svg"
        ))),
        "icons/explorer/worktree/file_type_code.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/file_type_code.svg"
        ))),
        "icons/explorer/worktree/file_type_music.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/file_type_music.svg"
        ))),
        "icons/explorer/worktree/file_type_image.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/file_type_image.svg"
        ))),
        "icons/explorer/worktree/file_type_txt.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/file_type_txt.svg"
        ))),
        "icons/explorer/worktree/file_type_default.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/file_type_default.svg"
        ))),
        "icons/explorer/worktree/markdown.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/markdown.svg"
        ))),
        "icons/explorer/worktree/chevron-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/chevron-down.svg"
        ))),
        "icons/explorer/worktree/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/chevron-right.svg"
        ))),
        "icons/explorer/worktree/view.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/view.svg"
        ))),
        "icons/explorer/worktree/hide.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/hide.svg"
        ))),
        "icons/explorer/worktree/sync_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/sync_folder.svg"
        ))),
        "icons/explorer/worktree/replace_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/replace_folder.svg"
        ))),
        "icons/explorer/worktree/collapse-all.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/worktree/collapse-all.svg"
        ))),

        // ── Explorer: panel top bar (panel header) ──────────────
        "icons/explorer/panel.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/panel.svg"
        ))),
        "icons/explorer/topbar/check.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/topbar/check.svg"
        ))),
        "icons/explorer/topbar/split-h.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/topbar/split-h.svg"
        ))),
        "icons/explorer/topbar/split-v.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/topbar/split-v.svg"
        ))),
        "icons/explorer/topbar/close.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/topbar/close.svg"
        ))),
        "icons/explorer/topbar/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/topbar/maximize.svg"
        ))),
        "icons/explorer/topbar/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/topbar/restore.svg"
        ))),

        // ── Explorer: status bar ──────────────────────────────────────
        "icons/explorer/bottombar/new_folder.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/explorer/bottombar/new_folder.svg"
        ))),

        // ── Titlebar: app menu buttons ────────────────────────────────
        "icons/titlebar/app_menu/app_menu.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/app_menu/app_menu.svg"
        ))),
        "icons/titlebar/app_menu/sun.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/app_menu/sun.svg"
        ))),
        "icons/titlebar/app_menu/moon.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/app_menu/moon.svg"
        ))),
        "icons/titlebar/app_menu/checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/app_menu/checkmark.svg"
        ))),
        "icons/titlebar/app_menu/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/app_menu/chevron-right.svg"
        ))),

        // ── Titlebar: window controls ─────────────────────────────────
        "icons/titlebar/chrome/close.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/chrome/close.svg"
        ))),
        "icons/titlebar/chrome/mins.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/chrome/mins.svg"
        ))),
        "icons/titlebar/chrome/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/chrome/maximize.svg"
        ))),
        "icons/titlebar/chrome/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/titlebar/chrome/restore.svg"
        ))),

        // ── Settings ──────────────────────────────────────────────────
        "icons/settings/select-chevron.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/select-chevron.svg"
        ))),
        "icons/settings/checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/checkmark.svg"
        ))),
        "icons/settings/chevron-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/chevron-down.svg"
        ))),
        "icons/settings/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/chevron-right.svg"
        ))),
        "icons/settings/sun.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/sun.svg"
        ))),
        "icons/settings/plus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/plus.svg"
        ))),
        "icons/settings/minus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/minus.svg"
        ))),
        "icons/settings/moon.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/moon.svg"
        ))),

        // ── Settings: panel top bar (panel header) ──────────────
        "icons/settings/panel.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/panel.svg"
        ))),
        "icons/settings/topbar/check.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/topbar/check.svg"
        ))),
        "icons/settings/topbar/split-h.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/topbar/split-h.svg"
        ))),
        "icons/settings/topbar/split-v.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/topbar/split-v.svg"
        ))),
        "icons/settings/topbar/close.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/topbar/close.svg"
        ))),
        "icons/settings/topbar/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/topbar/maximize.svg"
        ))),
        "icons/settings/topbar/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/settings/topbar/restore.svg"
        ))),

        // ── Editor: panel header ────────────────────────────────
        "icons/editor/panel.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/panel.svg"
        ))),
        "icons/editor/topbar/active.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/active.svg"
        ))),
        "icons/editor/topbar/check.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/check.svg"
        ))),
        "icons/editor/topbar/split-h.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/split-h.svg"
        ))),
        "icons/editor/topbar/split-v.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/split-v.svg"
        ))),
        "icons/editor/topbar/close.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/close.svg"
        ))),
        "icons/editor/topbar/search.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/search.svg"
        ))),
        "icons/editor/topbar/replace.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/replace.svg"
        ))),
        "icons/editor/topbar/prev.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/prev.svg"
        ))),
        "icons/editor/topbar/next.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/next.svg"
        ))),
        "icons/editor/topbar/search-explorer.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/search-explorer.svg"
        ))),
        "icons/editor/topbar/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/maximize.svg"
        ))),
        "icons/editor/topbar/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/topbar/restore.svg"
        ))),

        // ── Editor: panel status bar ──────────────────────────────────
        "icons/editor/bottombar/split-h.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/bottombar/split-h.svg"
        ))),
        "icons/editor/bottombar/split-v.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/bottombar/split-v.svg"
        ))),
        "icons/editor/bottombar/close.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/bottombar/close.svg"
        ))),
        "icons/editor/bottombar/checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/bottombar/checkmark.svg"
        ))),
        "icons/editor/bottombar/maximize.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/bottombar/maximize.svg"
        ))),
        "icons/editor/bottombar/restore.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/bottombar/restore.svg"
        ))),

        // ── Editor: context menu ──────────────────────────────────────
        "icons/editor/context_menu/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/context_menu/chevron-right.svg"
        ))),
        "icons/editor/context_menu/plus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/context_menu/plus.svg"
        ))),
        "icons/editor/context_menu/minus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/context_menu/minus.svg"
        ))),

        // ── Editor: WYSIWYG panel ─────────────────────────────────────
        "icons/editor/wysiwyg/checkbox.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/wysiwyg/checkbox.svg"
        ))),
        "icons/editor/wysiwyg/checkbox-checked.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/wysiwyg/checkbox-checked.svg"
        ))),
        "icons/editor/wysiwyg/table/plus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/wysiwyg/table/plus.svg"
        ))),
        "icons/editor/wysiwyg/codeblock/line-numbers.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/wysiwyg/codeblock/line-numbers.svg"
        ))),
        "icons/editor/wysiwyg/codeblock/copy.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/wysiwyg/codeblock/copy.svg"
        ))),
        "icons/editor/wysiwyg/codeblock/select-checkmark.svg" => {
            Some(Cow::Borrowed(include_bytes!(
                "../../../../assets/icons/editor/wysiwyg/codeblock/select-checkmark.svg"
            )))
        }
        "icons/editor/wysiwyg/codeblock/select-chevron.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/wysiwyg/codeblock/select-chevron.svg"
        ))),

        // ── Editor: preview panel ─────────────────────────────────────
        "icons/editor/preview/checkbox.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/preview/checkbox.svg"
        ))),
        "icons/editor/preview/checkbox-checked.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/preview/checkbox-checked.svg"
        ))),

        // ── Editor: outline panel ─────────────────────────────────────
        "icons/editor/outline/markdown.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/editor/outline/markdown.svg"
        ))),

        // ── Window chrome (kind-independent) ──────────────────────────
        "icons/chrome/check.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/chrome/check.svg"
        ))),
        "icons/chrome/missing.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/chrome/missing.svg"
        ))),

        // ── Splitter: gesture overlays ──
        "icons/splitter/arrow-up.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/arrow-up.svg"
        ))),
        "icons/splitter/arrow-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/arrow-down.svg"
        ))),
        "icons/splitter/arrow-left.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/arrow-left.svg"
        ))),
        "icons/splitter/arrow-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/arrow-right.svg"
        ))),
        "icons/splitter/dock-up.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/dock-up.svg"
        ))),
        "icons/splitter/dock-down.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/dock-down.svg"
        ))),
        "icons/splitter/dock-left.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/dock-left.svg"
        ))),
        "icons/splitter/dock-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/dock-right.svg"
        ))),
        "icons/splitter/split-area.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/split-area.svg"
        ))),
        "icons/splitter/swap.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/splitter/swap.svg"
        ))),

        // ── Identity ──────────────────────────────────────────────────
        "identity/logo.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/identity/logo.svg"
        ))),
        "identity/logo.png" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/identity/logo.png"
        ))),

        // ── About dialog emoji icons ──────────────────────────────────
        "icons/emoji/1.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/1.svg"
        ))),
        "icons/emoji/2.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/2.svg"
        ))),
        "icons/emoji/3.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/3.svg"
        ))),
        "icons/emoji/4.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/4.svg"
        ))),
        "icons/emoji/5.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/5.svg"
        ))),
        "icons/emoji/6.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/6.svg"
        ))),
        "icons/emoji/7.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/7.svg"
        ))),
        "icons/emoji/8.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/8.svg"
        ))),
        "icons/emoji/9.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/9.svg"
        ))),
        "icons/emoji/10.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/10.svg"
        ))),
        "icons/emoji/11.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/11.svg"
        ))),
        "icons/emoji/12.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/12.svg"
        ))),
        "icons/emoji/13.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/13.svg"
        ))),
        "icons/emoji/14.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/14.svg"
        ))),
        "icons/emoji/15.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/15.svg"
        ))),
        "icons/emoji/16.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/16.svg"
        ))),
        "icons/emoji/17.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/17.svg"
        ))),
        "icons/emoji/18.svg" => Some(Cow::Borrowed(include_bytes!(
            "../../../../assets/icons/emoji/18.svg"
        ))),

        // ── GPUI SVG renderer bundled font requests ──────────────────
        "fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf" | "fonts/lilex/Lilex-Regular.ttf" => {
            Some(Cow::Borrowed(include_bytes!(
                "../../../../assets/fonts/Lexend-Regular.ttf"
            )))
        }

        _ => None,
    }
}
