//! Document link relationships and backlink indexing subsystem.

pub mod indexer;
pub mod render;
pub mod types;
pub mod widget;
pub use widget as panel;

pub use indexer::{extract_outgoing_links, scan_workspace_backlinks};
pub use widget::{
    LinksPanelHost, LinksWidgetHost, render_links_panel_overlay, render_links_widget,
};
pub use types::{
    BacklinkGroup, BacklinkMention, LinkRelationReport, LinksPanelState, LinksPanelTab,
    LinksWidgetState, LinksWidgetTab, OutgoingLinkItem,
};
