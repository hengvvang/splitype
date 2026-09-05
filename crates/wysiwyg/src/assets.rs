//! Embedded SVG icon asset catalog for the WYSIWYG plugin.

use std::borrow::Cow;
use platform_contracts::PluginAssetProvider;

pub struct WysiwygAssets;

impl PluginAssetProvider for WysiwygAssets {
    fn load_asset(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        match_icon(path)
    }
}

/// Resolves an icon asset for the WYSIWYG pane.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = path
        .strip_prefix("plugin://splitype.wysiwyg/")
        .unwrap_or(path);

    match subpath {
        "checkbox.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkbox.svg"
        ))),
        "checkbox-checked.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkbox-checked.svg"
        ))),
        "codeblock/copy.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/codeblock/copy.svg"
        ))),
        "codeblock/line-numbers.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/codeblock/line-numbers.svg"
        ))),
        "codeblock/select-checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/codeblock/select-checkmark.svg"
        ))),
        "codeblock/select-chevron.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/codeblock/select-chevron.svg"
        ))),
        "table/plus.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/table/plus.svg"
        ))),
        "context_menu/chevron-right.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/context_menu/chevron-right.svg"
        ))),
        "context_menu/checkmark.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/context_menu/checkmark.svg"
        ))),
        _ => None,
    }
}
