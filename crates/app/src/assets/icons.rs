//! Embedded SVG icon asset catalog for Splitype.

use std::borrow::Cow;

pub(super) fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    match path {
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
