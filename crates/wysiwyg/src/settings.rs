//! WYSIWYG pane plugin settings — paste and block behavior owned by this
//! plugin.
//!
//! The struct is the plugin's typed view of its settings blob; the schema
//! that drives the settings UI is declared in the plugin's manifest
//! (`assets/plugins/splitype.wysiwyg.toml`). The two are verified to match
//! exactly by the test below.

use config::settings::PluginSettingsDefinition;
use serde::{Deserialize, Serialize};

/// Where pasted clipboard images should be stored before inserting Markdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImagePasteBehavior {
    #[default]
    None,
    CopyToDocumentFolder,
    CopyToAssetsFolder,
    CopyToNamedAssetsFolder,
}

impl ImagePasteBehavior {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CopyToDocumentFolder => "copy_to_document_folder",
            Self::CopyToAssetsFolder => "copy_to_assets_folder",
            Self::CopyToNamedAssetsFolder => "copy_to_named_assets_folder",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::None,
            Self::CopyToDocumentFolder,
            Self::CopyToAssetsFolder,
            Self::CopyToNamedAssetsFolder,
        ]
    }
}

impl std::fmt::Display for ImagePasteBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ImagePasteBehavior {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "copy_to_document_folder" => Ok(Self::CopyToDocumentFolder),
            "copy_to_assets_folder" => Ok(Self::CopyToAssetsFolder),
            "copy_to_named_assets_folder" => Ok(Self::CopyToNamedAssetsFolder),
            _ => Ok(Self::None),
        }
    }
}

/// WYSIWYG pane settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WysiwygSettings {
    #[serde(default)]
    pub image_paste_behavior: ImagePasteBehavior,
}

impl Default for WysiwygSettings {
    fn default() -> Self {
        Self {
            image_paste_behavior: ImagePasteBehavior::None,
        }
    }
}

impl PluginSettingsDefinition for WysiwygSettings {
    const PLUGIN_ID: &'static str = "splitype.wysiwyg";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declarations_cover_the_settings_struct() {
        let manifest: platform_contracts::PluginManifest = toml::from_str(include_str!(
            "../../../assets/plugins/splitype.wysiwyg.toml"
        ))
        .expect("bundled manifest must be valid TOML");
        let problems = platform_contracts::verify_setting_declarations::<WysiwygSettings>(
            &manifest.settings,
            &[],
        );
        assert!(problems.is_empty(), "declaration mismatches: {problems:#?}");
    }
}
