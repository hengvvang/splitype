//! Embedded SVG icon asset catalog for the Preview plugin.

use std::borrow::Cow;
use platform_contracts::PluginAssetProvider;

pub struct PreviewAssets;

impl PluginAssetProvider for PreviewAssets {
    fn load_asset(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        match_icon(path)
    }
}

/// Resolves an icon asset for the Preview pane.
pub fn match_icon(path: &str) -> Option<Cow<'static, [u8]>> {
    let subpath = path
        .strip_prefix("plugin://splitype.preview/")
        .unwrap_or(path);

    match subpath {
        "checkbox.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkbox.svg"
        ))),
        "checkbox-checked.svg" => Some(Cow::Borrowed(include_bytes!(
            "../assets/icons/checkbox-checked.svg"
        ))),
        _ => None,
    }
}
