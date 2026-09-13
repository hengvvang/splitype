//! Type definitions for the Links Relationship widget and indexer.

use std::path::PathBuf;

/// Active tab in the link relationships widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinksWidgetTab {
    Backlinks,
    Outgoing,
}

impl Default for LinksWidgetTab {
    fn default() -> Self {
        Self::Backlinks
    }
}

/// Backwards-compatible alias for `LinksWidgetTab`.
pub type LinksPanelTab = LinksWidgetTab;

/// Outgoing link extracted from the current document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutgoingLinkItem {
    pub display_text: String,
    pub target: String,
    pub is_external: bool,
    pub anchor: Option<String>,
}

/// A group of backlinks originating from a specific source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacklinkGroup {
    pub source_path: PathBuf,
    pub source_title: String,
    pub mentions: Vec<BacklinkMention>,
}

/// A single mention / link instance inside a source document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacklinkMention {
    pub line_number: usize,
    pub context_snippet: String,
    pub target_anchor: Option<String>,
}

/// Aggregate report of all link relationships for a document.
#[derive(Clone, Debug, Default)]
pub struct LinkRelationReport {
    pub outgoing: Vec<OutgoingLinkItem>,
    pub backlinks: Vec<BacklinkGroup>,
}

/// UI and operational state of the open links widget.
#[derive(Clone, Debug)]
pub struct LinksWidgetState {
    pub active_tab: LinksWidgetTab,
    pub search_query: String,
    pub target_file_path: Option<PathBuf>,
    pub target_pane_id: Option<editor_contracts::PaneId>,
    pub report: Option<LinkRelationReport>,
    pub is_loading: bool,
}

/// Backwards-compatible alias for `LinksWidgetState`.
pub type LinksPanelState = LinksWidgetState;

impl LinksWidgetState {
    pub fn new(
        target_file_path: Option<PathBuf>,
        target_pane_id: Option<editor_contracts::PaneId>,
    ) -> Self {
        Self {
            active_tab: LinksWidgetTab::Backlinks,
            search_query: String::new(),
            target_file_path,
            target_pane_id,
            report: None,
            is_loading: true,
        }
    }
}
