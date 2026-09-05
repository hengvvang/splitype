//! Explorer plugin settings — file-tree behavior owned by this plugin.
//!
//! The struct is the plugin's typed view of its settings blob; the schema
//! that drives the settings UI is declared in the plugin's manifest
//! (`assets/plugins/splitype.explorer.toml`). The two are verified to match
//! exactly by the test below.

use config::settings::PluginSettingsDefinition;
use serde::{Deserialize, Serialize};

/// Explorer tree sorting mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerSortMode {
    #[default]
    DirectoriesFirst,
    FilesFirst,
    Mixed,
}

impl ExplorerSortMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DirectoriesFirst => "directories_first",
            Self::FilesFirst => "files_first",
            Self::Mixed => "mixed",
        }
    }
}

impl std::fmt::Display for ExplorerSortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExplorerSortMode {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "files_first" => Ok(Self::FilesFirst),
            "mixed" => Ok(Self::Mixed),
            _ => Ok(Self::DirectoriesFirst),
        }
    }
}

/// Explorer tree sort order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerSortOrder {
    #[default]
    Ascending,
    Descending,
}

impl ExplorerSortOrder {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
        }
    }
}

impl std::fmt::Display for ExplorerSortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExplorerSortOrder {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "descending" => Ok(Self::Descending),
            _ => Ok(Self::Ascending),
        }
    }
}

/// Explorer panel settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplorerSettings {
    #[serde(default = "default_true")]
    pub hide_hidden: bool,
    #[serde(default)]
    pub sort_mode: ExplorerSortMode,
    #[serde(default)]
    pub sort_order: ExplorerSortOrder,
    #[serde(default = "default_true")]
    pub auto_reveal: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ExplorerSettings {
    fn default() -> Self {
        Self {
            hide_hidden: true,
            sort_mode: ExplorerSortMode::DirectoriesFirst,
            sort_order: ExplorerSortOrder::Ascending,
            auto_reveal: true,
        }
    }
}

impl PluginSettingsDefinition for ExplorerSettings {
    const PLUGIN_ID: &'static str = crate::plugin::PLUGIN_ID;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declarations_cover_the_settings_struct() {
        let manifest: platform_contracts::PluginManifest =
            toml::from_str(crate::MANIFEST_TOML).expect("bundled manifest must be valid TOML");
        let problems = platform_contracts::verify_setting_declarations::<ExplorerSettings>(
            &manifest.settings,
            &[],
        );
        assert!(problems.is_empty(), "declaration mismatches: {problems:#?}");
    }
}
