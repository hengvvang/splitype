//! Outline pane state — heading tree, expansion set, and selection.
//!
//! The outline is an editor pane; its state lives here on each
//! Editor entity (`Editor::outline`) instead of in the explorer sidebar
//! state, so the editor never depends on the explorer module.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Uniform row height for outline nodes (the virtualized list requires a
/// fixed height).
pub const OUTLINE_NODE_HEIGHT: f32 = 28.0;
/// Horizontal indent per heading depth.
pub const OUTLINE_NODE_INDENT: f32 = 14.0;

/// Stable hash for outline node ids, used to build element ids.
pub fn outline_node_hash(id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

/// A node in the outline tree (headings only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineNode {
    pub id: String,
    pub label: String,
    pub kind: OutlineNodeKind,
    pub children: Vec<OutlineNode>,
}

/// Outline node kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutlineNodeKind {
    Heading { line: usize, level: u8 },
}

/// Combined outline pane state: the parsed tree plus which nodes are
/// expanded and selected.
#[derive(Clone, Debug, Default)]
pub struct OutlinePaneState {
    pub tree: Vec<OutlineNode>,
    /// Markdown source the tree was built from; `None` until first sync.
    pub source: Option<String>,
    /// Expanded node ids (outline ids are strings, unlike explorer entries).
    pub expanded: HashSet<String>,
    pub selected: Option<String>,
}
