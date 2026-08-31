use std::path::PathBuf;
use gpui::EntityId;

/// A single heading item in the outline TOC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineNode {
    /// Unique identifier for the heading element in the outline.
    pub id: String,
    /// Display label of the heading.
    pub label: String,
    /// Heading level (1..=6).
    pub level: u8,
    /// 0-indexed line index (in SourceCode/Preview) or block index (in WYSIWYG).
    pub block_index: usize,
    /// Logical EntityId if in active WYSIWYG document.
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

