//! Durable explorer panel state persisted across launches.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Durable explorer facts persisted across launches (and shared with the
/// suspend/clone protocol). Live worktree entities are re-scanned from the
/// folder paths on restore.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedExplorerState {
    pub is_open: bool,
    pub open_folders: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_explorer_state_round_trips() {
        let state = PersistedExplorerState {
            is_open: true,
            open_folders: vec![PathBuf::from("/notes")],
        };
        let json = serde_json::to_value(&state).expect("serialize");
        let restored: PersistedExplorerState = serde_json::from_value(json).expect("deserialize");
        assert!(restored.is_open);
        assert_eq!(restored.open_folders, vec![PathBuf::from("/notes")]);
    }
}
