//! Outline HUD state — heading list, hover state, active section tracking, and debounce timer.

use gpui::EntityId;

use std::path::PathBuf;

/// A heading node in the outline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineNode {
    pub id: String,
    pub label: String,
    pub level: u8,
    pub block_index: usize,
    pub block_id: Option<EntityId>,
}

/// Outline HUD state attached to an Editor.
#[derive(Clone, Debug, Default)]
pub struct OutlineHudState {
    pub headings: Vec<OutlineNode>,
    /// Last synced tab index for cache validation.
    pub synced_tab_index: Option<usize>,
    /// Last synced file path.
    pub synced_file_path: Option<PathBuf>,
    /// Last synced document revision for cache validation.
    pub synced_revision: Option<u64>,
    /// Last synced content hash.
    pub synced_hash: u64,
    /// Whether the hover TOC popover card is currently visible.
    pub is_hovered: bool,
    /// Index of the heading currently in the active viewport.
    pub active_index: Option<usize>,
    /// Debounce generation token for mouse leave closure.
    pub close_token: usize,
}

