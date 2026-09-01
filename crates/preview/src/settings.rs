//! Preview pane plugin settings — Markdown rendering behavior owned by this
//! plugin.
//!
//! The struct is the plugin's typed view of its settings blob; the schema
//! that drives the settings UI is declared in the plugin's manifest
//! (`assets/plugins/splitype.preview.toml`). The two are verified to match
//! exactly by the test below.

use config::settings::PluginSettingsDefinition;
use serde::{Deserialize, Serialize};

/// Preview pane settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewSettings {
    #[serde(default = "default_true")]
    pub show_table_headers: bool,
    #[serde(default = "default_true")]
    pub render_math: bool,
    #[serde(default = "default_true")]
    pub render_diagrams: bool,
}

fn default_true() -> bool {
    true
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            show_table_headers: true,
            render_math: true,
            render_diagrams: true,
        }
    }
}

impl PluginSettingsDefinition for PreviewSettings {
    const PLUGIN_ID: &'static str = "splitype.preview";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declarations_cover_the_settings_struct() {
        let manifest: platform_contracts::PluginManifest = toml::from_str(include_str!(
            "../../../assets/plugins/splitype.preview.toml"
        ))
        .expect("bundled manifest must be valid TOML");
        let problems = platform_contracts::verify_setting_declarations::<PreviewSettings>(
            &manifest.settings,
            &[],
        );
        assert!(problems.is_empty(), "declaration mismatches: {problems:#?}");
    }
}
