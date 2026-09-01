//! Source Code pane plugin settings — display and editing behavior owned by
//! this plugin.
//!
//! The struct is the plugin's typed view of its settings blob; the schema
//! that drives the settings UI is declared in the plugin's manifest
//! (`assets/plugins/splitype.source-code.toml`). The two are verified to
//! match exactly by the test below.

use config::settings::PluginSettingsDefinition;
use serde::{Deserialize, Serialize};

/// Source Code pane settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCodeSettings {
    #[serde(default = "default_true")]
    pub line_numbers: bool,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default = "default_tab_size")]
    pub tab_size: u32,
    #[serde(default = "default_true")]
    pub insert_spaces: bool,
    #[serde(default = "default_true")]
    pub highlight_active_line: bool,
}

fn default_true() -> bool {
    true
}

fn default_tab_size() -> u32 {
    4
}

impl Default for SourceCodeSettings {
    fn default() -> Self {
        Self {
            line_numbers: true,
            word_wrap: true,
            tab_size: 4,
            insert_spaces: true,
            highlight_active_line: true,
        }
    }
}

impl SourceCodeSettings {
    /// The indentation unit inserted by Tab/indent, honoring `insert_spaces`.
    pub fn indent_unit(self) -> String {
        if self.insert_spaces {
            " ".repeat(self.tab_size.max(1) as usize)
        } else {
            "\t".to_string()
        }
    }
}

impl PluginSettingsDefinition for SourceCodeSettings {
    const PLUGIN_ID: &'static str = "splitype.source-code";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declarations_cover_the_settings_struct() {
        let manifest: platform_contracts::PluginManifest = toml::from_str(include_str!(
            "../../../assets/plugins/splitype.source-code.toml"
        ))
        .expect("bundled manifest must be valid TOML");
        let problems = platform_contracts::verify_setting_declarations::<SourceCodeSettings>(
            &manifest.settings,
            &[],
        );
        assert!(problems.is_empty(), "declaration mismatches: {problems:#?}");
    }
}
