//! Plugin asset provider SPI.

use std::borrow::Cow;

/// SPI trait for plugins providing their own embedded or on-disk assets.
pub trait PluginAssetProvider: Send + Sync + 'static {
    /// Loads asset bytes for a relative path within this plugin's scope.
    fn load_asset(&self, path: &str) -> Option<Cow<'static, [u8]>>;
}
