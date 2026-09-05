//! Window-state persistence service: saves and loads the last window's
//! layout topology and per-panel plugin state as a versioned JSON snapshot.

use anyhow::Context as _;
use config::dirs::SplitypeConfigDirs;
use window_assembly::{PersistedWindowState, WINDOW_STATE_VERSION};

/// Loads the persisted last-window snapshot, if one exists.
///
/// Returns `None` when no snapshot has been written yet or when it was
/// written by an incompatible schema version.
pub fn load_window_state() -> anyhow::Result<Option<PersistedWindowState>> {
    let dirs = SplitypeConfigDirs::from_system()?;
    let path = dirs.window_state_file();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    let state: PersistedWindowState = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse '{}'", path.display()))?;
    if state.version != WINDOW_STATE_VERSION {
        tracing::warn!(
            version = state.version,
            "ignoring window state written by an unsupported schema version"
        );
        return Ok(None);
    }
    Ok(Some(state))
}

/// Persists a window snapshot, overwriting any previous one.
pub fn save_window_state(state: &PersistedWindowState) -> anyhow::Result<()> {
    let dirs = SplitypeConfigDirs::from_system()?;
    let path = dirs.window_state_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(state).context("failed to serialize window state")?;
    std::fs::write(&path, text).with_context(|| format!("failed to write '{}'", path.display()))
}
