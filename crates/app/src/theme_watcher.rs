//! User theme directory watcher — keeps the theme registry in sync with
//! externally added, edited, or removed theme family files.
//!
//! Implemented as a lightweight foreground poll: once per second, the
//! directory's `.json`/`.jsonc` files are hashed and compared with the
//! previous snapshot; any change reloads the registry. Theme files are a
//! handful of small documents, so polling is simpler and more portable than
//! a platform file watcher, and it avoids crossing gpui's non-`Send`
//! `AsyncApp` boundary.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{App, BorrowAppContext};

use theme::ThemeManager;

/// Watches the user themes directory and reloads the theme registry whenever
/// a `.json`/`.jsonc` theme family file is created, modified, or removed.
pub(crate) fn watch_user_theme_directory(cx: &mut App) {
    let Ok(dirs) = config::dirs::SplitypeConfigDirs::from_system() else {
        return;
    };
    let themes_dir = dirs.themes_dir();
    if let Err(err) = std::fs::create_dir_all(&themes_dir) {
        tracing::warn!(error = %err, "failed to create the themes directory; watcher disabled");
        return;
    }

    let mut last_snapshot = snapshot(&themes_dir);
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let current = snapshot(&themes_dir);
            if current == last_snapshot {
                continue;
            }
            last_snapshot = current;
            cx.update(|cx| {
                cx.update_global::<ThemeManager, _>(|manager, _cx| {
                    if let Err(err) = manager.reload_user_themes() {
                        tracing::warn!(error = %err, "failed to reload user themes");
                    }
                });
            });
        }
    })
    .detach();
}

/// Snapshot of the directory: supported file name → content hash.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, u64> {
    let mut files = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || !config::jsonc::is_supported_config_file(&path) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        files.insert(entry.path(), hash_bytes(&bytes));
    }
    files
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
