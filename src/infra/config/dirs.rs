//! Cross-platform configuration directory helpers.

use std::path::PathBuf;

use anyhow::Context as _;
use directories::ProjectDirs;

/// Cross-platform configuration directories owned by splitype.
#[derive(Debug, Clone)]
pub(crate) struct SplitypeConfigDirs {
    root: PathBuf,
}

impl SplitypeConfigDirs {
    /// Resolves the platform-specific app config directory.
    ///
    /// GPUI does not currently expose an app config path, so user-imported
    /// language and theme packs are stored under the OS location returned by
    /// `directories::ProjectDirs`.
    pub(crate) fn from_system() -> anyhow::Result<Self> {
        let dirs = ProjectDirs::from("com", "hengvvang", "splitype")
            .context("failed to resolve the splitype config directory")?;
        Ok(Self {
            root: dirs.config_dir().to_path_buf(),
        })
    }

    /// Creates a directory set from a caller-provided root for tests.
    #[cfg(test)]
    pub(crate) fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub(crate) fn languages_dir(&self) -> PathBuf {
        self.root.join("languages")
    }

    pub(crate) fn themes_dir(&self) -> PathBuf {
        self.root.join("themes")
    }

    pub(crate) fn history_file(&self) -> PathBuf {
        self.root.join(".history")
    }

    pub(crate) fn recent_folders_file(&self) -> PathBuf {
        self.root.join(".recent-folders")
    }

    pub(crate) fn app_config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }
}
