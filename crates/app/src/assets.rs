//! Application asset loader for bundled SVG icons and fonts.

use gpui::*;
use std::borrow::Cow;

mod fonts;
mod icons;

pub struct SplitypeAssets;

impl AssetSource for SplitypeAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if let Some(resource) = path.strip_prefix("plugin://") {
            return Ok(crate::plugins::resolve_plugin_resource(resource));
        }
        Ok(icons::match_icon(path))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

/// Looks up one asset-catalog icon by its `assets/`-relative key.
pub(crate) fn icon_bytes(path: &str) -> Option<Cow<'static, [u8]>> {
    icons::match_icon(path)
}

impl SplitypeAssets {
    /// Populate the [`TextSystem`] with all 9 embedded Lexend font variants.
    pub fn load_fonts(cx: &App) -> gpui::Result<()> {
        fonts::load_fonts(cx)
    }
}
