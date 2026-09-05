//! Embedded SVG icon asset catalog for the Source Code editor plugin.

use std::borrow::Cow;
use platform_contracts::PluginAssetProvider;

pub struct SourceCodeAssets;

impl PluginAssetProvider for SourceCodeAssets {
    fn load_asset(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        match_icon(path)
    }
}

/// Resolves an icon asset for the Source Code pane.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = path
        .strip_prefix("plugin://splitype.source-code/")
        .unwrap_or(path);

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
