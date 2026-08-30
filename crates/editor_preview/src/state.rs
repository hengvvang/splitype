//! PreviewPane state — read-only block tree, selection and sync markers.

use std::ops::Range;

use gpui::{App, IntoElement};

use crate::node::PreviewBlock;
use crate::selection::{PreviewEndpoint, PreviewSelectionRange};

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

use editor_model::{EditorDocument, PaneKindId, PaneRenderContext, PaneView};

impl PaneView for PreviewState {
    fn kind(&self) -> PaneKindId {
        PaneKindId::PREVIEW
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        doc.serialize_markdown(cx)
    }

    fn render(
        &mut self,
        _ctx: &PaneRenderContext,
        _window: &mut gpui::Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        gpui::div().into_any_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
