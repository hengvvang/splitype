//! Raw input helpers — drag & drop detection and keyboard capture.

pub mod keyboard;

use std::path::{Path, PathBuf};

/// Returns true when `path` exists and has a `.md` or `.markdown` extension.
pub(crate) fn is_markdown_file_path(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|extension| {
            extension.to_string_lossy().eq_ignore_ascii_case("md")
                || extension.to_string_lossy().eq_ignore_ascii_case("markdown")
        })
}

/// Returns the first path in `paths` that passes [`is_markdown_file_path`].
pub(crate) fn first_dropped_markdown_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths
        .iter()
        .find(|path| is_markdown_file_path(path))
        .cloned()
}

