//! Outline panel state — heading tree, expansion set, and selection.
//!
//! The outline is an editor inner panel; its state lives here on the Editor
//! entity (`WindowPanels::outline`) instead of in the explorer sidebar
//! state, so the editor never depends on the explorer module.

use std::collections::HashSet;

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

/// Combined outline panel state: the parsed tree plus which nodes are
/// expanded and selected.
#[derive(Clone, Debug, Default)]
pub struct OutlinePanelState {
    pub tree: Vec<OutlineNode>,
    /// Markdown source the tree was built from; `None` until first sync.
    pub source: Option<String>,
    /// Expanded node ids (outline ids are strings, unlike explorer entries).
    pub expanded: HashSet<String>,
    pub selected: Option<String>,
}
