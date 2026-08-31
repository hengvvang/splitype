//! Application asset loader for bundled SVG icons and fonts.

use gpui::*;
use std::borrow::Cow;

mod fonts;
mod icons;

pub struct SplitypeAssets;

impl AssetSource for SplitypeAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(icons::match_icon(path))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

impl SplitypeAssets {
    /// Populate the [`TextSystem`] with all 9 embedded Lexend font variants.
    pub fn load_fonts(cx: &App) -> gpui::Result<()> {
        fonts::load_fonts(cx)
    }
}
