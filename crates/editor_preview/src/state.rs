//! PreviewPane state — read-only block tree, selection and sync markers.

use std::ops::Range;

use gpui::App;

use crate::node::PreviewBlock;
use crate::selection::{PreviewEndpoint, PreviewSelectionRange};
use editor_model::{EditorDocument, EditorPaneKind, Pane};
use editor_outline::OutlineNode;

/// Read-only block tree shown in the preview panel.
#[derive(Default)]
pub struct PreviewState {
    pub blocks: Vec<PreviewBlock>,
    pub selection: Option<PreviewSelectionRange>,
    pub drag_anchor: Option<PreviewEndpoint>,
    pub search_matches: Vec<(Range<usize>, bool)>,
    pub source_hash: u64,
    /// Document revision the preview tree was last synced at; `None` until
    /// the first build.
    pub synced_revision: Option<u64>,
}

impl Pane for PreviewState {
    fn kind(&self) -> EditorPaneKind {
        EditorPaneKind::Preview
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        doc.serialize_markdown(cx)
    }

    fn set_search_matches(&mut self, matches: &[(Range<usize>, bool)]) {
        self.search_matches = matches.to_vec();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn outline_items(&self, doc: &dyn EditorDocument, cx: &App) -> Vec<OutlineNode> {
        let markdown = doc.serialize_markdown(cx);
        crate::outline::extract_outline_headings(&markdown)
    }
}
