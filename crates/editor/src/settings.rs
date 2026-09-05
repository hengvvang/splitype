//! Editor plugin settings — status bar metrics owned by this plugin.
//!
//! The struct is the plugin's typed view of its settings blob; the schema
//! that drives the settings UI is declared in the plugin's manifest
//! (`assets/plugins/splitype.editor.toml`). The two are verified to match
//! exactly by the test below.

use config::settings::PluginSettingsDefinition;
use serde::{Deserialize, Serialize};

/// Editor settings: the status bar is editor chrome, so its visibility and
/// metric toggles live with this plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSettings {
    #[serde(default = "default_true")]
    pub status_bar_enabled: bool,
    #[serde(default = "default_true")]
    pub show_word_count: bool,
    #[serde(default = "default_true")]
    pub show_cursor_position: bool,
    #[serde(default = "default_true")]
    pub show_character_count: bool,
    #[serde(default = "default_true")]
    pub show_reading_time: bool,
}

fn default_true() -> bool {
    true
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            status_bar_enabled: true,
            show_word_count: true,
            show_cursor_position: true,
            show_character_count: true,
            show_reading_time: true,
        }
    }
}

impl PluginSettingsDefinition for EditorSettings {
    const PLUGIN_ID: &'static str = crate::plugin::PLUGIN_ID;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declarations_cover_the_settings_struct() {
        let manifest: platform_contracts::PluginManifest =
            toml::from_str(crate::MANIFEST_TOML)
                .expect("bundled manifest must be valid TOML");
        let problems = platform_contracts::verify_setting_declarations::<EditorSettings>(
            &manifest.settings,
            &[],
        );
        assert!(problems.is_empty(), "declaration mismatches: {problems:#?}");
    }
}
