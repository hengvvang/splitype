//! Cross-platform configuration directory helpers.

use std::path::PathBuf;

use anyhow::Context as _;
use directories::ProjectDirs;

/// Cross-platform configuration directories owned by splitype.
#[derive(Debug, Clone)]
pub struct SplitypeConfigDirs {
    root: PathBuf,
}

impl SplitypeConfigDirs {
    /// Resolves the platform-specific app config directory.
    ///
    /// GPUI does not currently expose an app config path, so user-imported
    /// language and theme packs are stored under the OS location returned by
    /// `directories::ProjectDirs`.
    pub fn from_system() -> anyhow::Result<Self> {
        let dirs = ProjectDirs::from("com", "hengvvang", "splitype")
            .context("failed to resolve the splitype config directory")?;
        Ok(Self {
            root: dirs.config_dir().to_path_buf(),
        })
    }

    /// Creates a directory set from a caller-provided root, used by
    /// integration tests and embedders to isolate file I/O.
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn languages_dir(&self) -> PathBuf {
        self.root.join("languages")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.root.join("themes")
    }

    pub fn history_file(&self) -> PathBuf {
        self.root.join(".history")
    }

    pub fn recent_folders_file(&self) -> PathBuf {
        self.root.join(".recent-folders")
    }

    pub fn app_config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// Persisted last-window snapshot (layout + panel states).
    pub fn window_state_file(&self) -> PathBuf {
        self.root.join("window_state.json")
    }

    /// User-installed plugin manifests (`*.toml`).
    pub fn plugins_dir(&self) -> PathBuf {
        self.root.join("plugins")
    }
}
